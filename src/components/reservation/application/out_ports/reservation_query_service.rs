use chrono::{DateTime, Utc};

use crate::components::reservation::application::ApplicationError;
use crate::components::reservation::domain::{ReservationId, ReservationStatus, RoomId, UserId};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScheduleItem {
    pub id: ReservationId,
    pub room_id: RoomId,
    pub user_id: UserId,
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
    pub status: ReservationStatus,
    pub cancelled_at: Option<DateTime<Utc>>,
}

#[async_trait::async_trait]
pub trait ReservationQueryService: Send + Sync {
    async fn find_schedule(
        &self,
        room_id: RoomId,
        range_start: DateTime<Utc>,
        range_end: DateTime<Utc>,
    ) -> Result<Vec<ScheduleItem>, ApplicationError>;
}
