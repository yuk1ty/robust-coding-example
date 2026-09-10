use axum::Json;
use axum::extract::rejection::{JsonRejection, QueryRejection};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Serialize;

use crate::components::reservation::application::ApplicationError;
use crate::components::reservation::domain::DomainError;

#[derive(Debug, Serialize)]
pub struct ErrorBody {
    pub code: &'static str,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,
}

pub struct ApiError {
    status: StatusCode,
    body: ErrorBody,
}

impl ApiError {
    // TODO: staticである必要はない気がする
    pub fn new(status: StatusCode, code: &'static str, message: impl Into<String>) -> Self {
        Self {
            status,
            body: ErrorBody {
                code,
                message: message.into(),
                details: None,
            },
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.status, Json(self.body)).into_response()
    }
}

impl From<JsonRejection> for ApiError {
    fn from(rejection: JsonRejection) -> Self {
        Self::new(
            StatusCode::BAD_REQUEST,
            "invalid_input",
            rejection.body_text(),
        )
    }
}

impl From<QueryRejection> for ApiError {
    fn from(rejection: QueryRejection) -> Self {
        Self::new(
            StatusCode::BAD_REQUEST,
            "invalid_input",
            rejection.body_text(),
        )
    }
}

impl From<ApplicationError> for ApiError {
    fn from(error: ApplicationError) -> Self {
        match error {
            ApplicationError::Domain(DomainError::InvalidTimeRange) => Self::new(
                StatusCode::BAD_REQUEST,
                "invalid_time_range",
                error.to_string(),
            ),
            ApplicationError::Domain(DomainError::InvalidStatus(_)) => Self::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                "internal_error",
                "Unexpected error",
            ),
            ApplicationError::Domain(DomainError::AlreadyCancelled) => {
                Self::new(StatusCode::CONFLICT, "already_cancelled", error.to_string())
            }
            ApplicationError::InvalidScheduleWindow => Self::new(
                StatusCode::BAD_REQUEST,
                "invalid_schedule_window",
                error.to_string(),
            ),
            ApplicationError::ReservationConflict => Self::new(
                StatusCode::CONFLICT,
                "reservation_conflict",
                error.to_string(),
            ),
            ApplicationError::ReservationNotFound => Self::new(
                StatusCode::NOT_FOUND,
                "reservation_not_found",
                error.to_string(),
            ),
            ApplicationError::Infrastructure => Self::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                "internal_error",
                "Unexpected error",
            ),
        }
    }
}
