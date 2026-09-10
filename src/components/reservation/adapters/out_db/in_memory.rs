use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use chrono::{DateTime, Utc};

use crate::components::reservation::application::ApplicationError;
use crate::components::reservation::application::out_ports::{
    ReservationQueryService, ReservationRepository, ScheduleItem,
};
use crate::components::reservation::domain::{
    MeetingTimeRange, Reservation, ReservationId, ReservationStatus, RoomId,
};

#[derive(Clone, Default)]
pub struct InMemoryReservationStore {
    inner: Arc<Mutex<HashMap<ReservationId, Reservation>>>,
}

impl InMemoryReservationStore {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait::async_trait]
impl ReservationRepository for InMemoryReservationStore {
    async fn find_by_id(&self, id: ReservationId) -> Result<Option<Reservation>, ApplicationError> {
        Ok(self.inner.lock().expect("lock").get(&id).cloned())
    }

    async fn find_confirmed_candidates(
        &self,
        room_id: RoomId,
        time_range: &MeetingTimeRange,
    ) -> Result<Vec<Reservation>, ApplicationError> {
        Ok(self
            .inner
            .lock()
            .expect("lock")
            .values()
            .filter(|reservation| {
                reservation.room_id() == room_id
                    && reservation.status() == ReservationStatus::Confirmed
                    && reservation.time_range().overlaps(time_range)
            })
            .cloned()
            .collect())
    }

    async fn save(&self, reservation: &Reservation) -> Result<(), ApplicationError> {
        self.inner
            .lock()
            .expect("lock")
            .insert(reservation.id(), reservation.clone());
        Ok(())
    }
}

#[async_trait::async_trait]
impl ReservationQueryService for InMemoryReservationStore {
    async fn find_schedule(
        &self,
        room_id: RoomId,
        range_start: DateTime<Utc>,
        range_end: DateTime<Utc>,
    ) -> Result<Vec<ScheduleItem>, ApplicationError> {
        Ok(self
            .inner
            .lock()
            .expect("lock")
            .values()
            .filter(|reservation| {
                reservation.room_id() == room_id
                    && reservation.time_range().start() < range_end
                    && range_start < reservation.time_range().end()
            })
            .map(|reservation| ScheduleItem {
                id: reservation.id(),
                room_id: reservation.room_id(),
                user_id: reservation.user_id(),
                start: reservation.time_range().start(),
                end: reservation.time_range().end(),
                status: reservation.status(),
                cancelled_at: reservation.cancelled_at(),
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::reservation::domain::UserId;
    use chrono::TimeZone;
    use uuid::Uuid;

    #[tokio::test]
    async fn should_persist_a_reservation() {
        let store = InMemoryReservationStore::default();
        let start = Utc.with_ymd_and_hms(2026, 9, 10, 10, 0, 0).unwrap();
        let end = Utc.with_ymd_and_hms(2026, 9, 10, 11, 0, 0).unwrap();
        let reservation = Reservation::new(
            ReservationId::new(Uuid::from_u128(1)),
            RoomId::new(Uuid::from_u128(2)),
            UserId::new(Uuid::from_u128(3)),
            MeetingTimeRange::new(start, end).unwrap(),
            start,
        );
        store.save(&reservation).await.unwrap();
        let loaded = store.find_by_id(reservation.id()).await.unwrap().unwrap();
        assert_eq!(loaded.id(), reservation.id());
        assert_eq!(loaded.status(), ReservationStatus::Confirmed);
    }
}
