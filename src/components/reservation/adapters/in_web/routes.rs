use axum::extract::rejection::{JsonRejection, QueryRejection};
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::routing::{delete, get, post};
use axum::{Json, Router};
use chrono_tz::Tz;
use std::sync::Arc;
use uuid::Uuid;

use crate::components::reservation::adapters::in_web::dto::{
    CreateReservationRequest, ReservationResponse, ScheduleQuery,
};
use crate::components::reservation::adapters::in_web::error::ApiError;
use crate::components::reservation::application::ApplicationError;
use crate::components::reservation::application::commands::{
    CancelReservationCommand, CancelReservationHandler, ReserveRoomCommand, ReserveRoomHandler,
};
use crate::components::reservation::application::out_ports::{
    Clock, ReservationQueryService, ReservationRepository, SystemClock,
};
use crate::components::reservation::application::queries::{
    GetRoomScheduleHandler, GetRoomScheduleQuery,
};
pub fn router<S>(store: Arc<S>) -> Router
where
    S: ReservationRepository + ReservationQueryService + Send + Sync + 'static,
{
    router_with_clock(store, Arc::new(SystemClock))
}

pub fn router_with_clock<S, C>(store: Arc<S>, clock: Arc<C>) -> Router
where
    S: ReservationRepository + ReservationQueryService + Send + Sync + 'static,
    C: Clock + Send + Sync + 'static,
{
    let state = HandlerState { store, clock };
    Router::new()
        .route("/reservations", post(create_reservation::<S, C>))
        .route("/reservations/{id}", delete(cancel_reservation::<S, C>))
        .route("/rooms/{room_id}/schedule", get(get_schedule::<S, C>))
        .with_state(state)
}

struct HandlerState<S, C> {
    store: Arc<S>,
    clock: Arc<C>,
}

impl<S, C> Clone for HandlerState<S, C> {
    fn clone(&self) -> Self {
        Self {
            store: self.store.clone(),
            clock: self.clock.clone(),
        }
    }
}

async fn create_reservation<S, C>(
    State(state): State<HandlerState<S, C>>,
    payload: Result<Json<CreateReservationRequest>, JsonRejection>,
) -> Result<(StatusCode, Json<ReservationResponse>), ApiError>
where
    S: ReservationRepository + Send + Sync + 'static,
    C: Clock + Send + Sync + 'static,
{
    let Json(body) = payload?;
    let handler = ReserveRoomHandler::new(state.store.clone(), state.clock.clone());
    let id = handler
        .handle(ReserveRoomCommand {
            room_id: body.room_id,
            user_id: body.user_id,
            start: body.start,
            end: body.end,
        })
        .await?;
    let reservation = state
        .store
        .find_by_id(id)
        .await?
        .ok_or(ApplicationError::Infrastructure)?;
    Ok((
        StatusCode::CREATED,
        Json(ReservationResponse::from(&reservation)),
    ))
}

async fn cancel_reservation<S, C>(
    State(state): State<HandlerState<S, C>>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, ApiError>
where
    S: ReservationRepository + Send + Sync + 'static,
    C: Clock + Send + Sync + 'static,
{
    let handler = CancelReservationHandler::new(state.store.clone(), state.clock.clone());
    handler
        .handle(CancelReservationCommand { reservation_id: id })
        .await?;
    Ok(StatusCode::OK)
}

async fn get_schedule<S, C>(
    State(state): State<HandlerState<S, C>>,
    Path(room_id): Path<Uuid>,
    query: Result<Query<ScheduleQuery>, QueryRejection>,
) -> Result<Json<Vec<ReservationResponse>>, ApiError>
where
    S: ReservationQueryService + Send + Sync + 'static,
{
    let Query(query) = query?;
    let timezone = match query.timezone {
        Some(name) => name.parse::<Tz>().map_err(|_| {
            ApiError::new(
                StatusCode::BAD_REQUEST,
                "invalid_timezone",
                "Unknown timezone",
            )
        })?,
        None => Tz::UTC,
    };
    let handler = GetRoomScheduleHandler::new(state.store.clone());
    let items = handler
        .handle(GetRoomScheduleQuery {
            room_id,
            date: query.date,
            timezone,
        })
        .await?;
    Ok(Json(
        items.into_iter().map(ReservationResponse::from).collect(),
    ))
}
