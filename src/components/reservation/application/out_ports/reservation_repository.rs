use crate::components::reservation::application::ApplicationError;
use crate::components::reservation::domain::{MeetingTimeRange, Reservation, ReservationId, RoomId};

#[async_trait::async_trait]
pub trait ReservationRepository: Send + Sync {
    async fn find_by_id(
        &self,
        id: ReservationId,
    ) -> Result<Option<Reservation>, ApplicationError>;

    async fn find_confirmed_candidates(
        &self,
        room_id: RoomId,
        time_range: &MeetingTimeRange,
    ) -> Result<Vec<Reservation>, ApplicationError>;

    async fn save(&self, reservation: &Reservation) -> Result<(), ApplicationError>;
}
