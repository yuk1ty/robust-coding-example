use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::components::reservation::application::out_ports::ScheduleItem;
use crate::components::reservation::domain::Reservation;

#[derive(Debug, Deserialize)]
pub struct CreateReservationRequest {
    pub room_id: Uuid,
    pub user_id: Uuid,
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct ReservationResponse {
    pub id: Uuid,
    pub room_id: Uuid,
    pub user_id: Uuid,
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
    pub status: String,
    pub cancelled_at: Option<DateTime<Utc>>,
}

impl From<&Reservation> for ReservationResponse {
    fn from(reservation: &Reservation) -> Self {
        Self {
            id: reservation.id().as_uuid(),
            room_id: reservation.room_id().as_uuid(),
            user_id: reservation.user_id().as_uuid(),
            start: reservation.time_range().start(),
            end: reservation.time_range().end(),
            status: reservation.status().as_str().to_owned(),
            cancelled_at: reservation.cancelled_at(),
        }
    }
}

impl From<ScheduleItem> for ReservationResponse {
    fn from(item: ScheduleItem) -> Self {
        Self {
            id: item.id.as_uuid(),
            room_id: item.room_id.as_uuid(),
            user_id: item.user_id.as_uuid(),
            start: item.start,
            end: item.end,
            status: item.status.as_str().to_owned(),
            cancelled_at: item.cancelled_at,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct ScheduleQuery {
    pub date: chrono::NaiveDate,
    pub timezone: Option<String>,
}
