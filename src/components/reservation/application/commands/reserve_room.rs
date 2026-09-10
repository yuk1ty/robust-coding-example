use std::sync::Arc;

use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::components::reservation::application::ApplicationError;
use crate::components::reservation::application::out_ports::{Clock, ReservationRepository};
use crate::components::reservation::domain::{
    MeetingTimeRange, Reservation, ReservationId, RoomId, UserId,
};

pub struct ReserveRoomCommand {
    pub room_id: Uuid,
    pub user_id: Uuid,
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
}

pub struct ReserveRoomHandler {
    repository: Arc<dyn ReservationRepository>,
    clock: Arc<dyn Clock>,
}

impl ReserveRoomHandler {
    pub fn new(repository: Arc<dyn ReservationRepository>, clock: Arc<dyn Clock>) -> Self {
        Self { repository, clock }
    }

    pub async fn handle(&self, command: ReserveRoomCommand) -> Result<ReservationId, ApplicationError> {
        let time_range = MeetingTimeRange::new(command.start, command.end)?;
        let room_id = RoomId::new(command.room_id);
        let existing = self
            .repository
            .find_confirmed_candidates(room_id, &time_range)
            .await?;

        if existing
            .iter()
            .any(|reservation| reservation.time_range().overlaps(&time_range))
        {
            return Err(ApplicationError::ReservationConflict);
        }

        let id = ReservationId::new(Uuid::new_v4());
        let reservation = Reservation::new(
            id,
            room_id,
            UserId::new(command.user_id),
            time_range,
            self.clock.now(),
        );
        self.repository.save(&reservation).await?;
        Ok(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::reservation::adapters::out_db::InMemoryReservationStore;
    use crate::components::reservation::application::out_ports::FixedClock;
    use crate::components::reservation::domain::{DomainError, ReservationStatus};
    use chrono::TimeZone;
    use std::sync::Arc;

    fn dt(hour: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 9, 10, hour, 0, 0).unwrap()
    }

    fn clock() -> FixedClock {
        FixedClock(Utc.with_ymd_and_hms(2026, 9, 1, 0, 0, 0).unwrap())
    }

    async fn seed_confirmed(store: &InMemoryReservationStore, start: u32, end: u32) -> ReservationId {
        let range = MeetingTimeRange::new(dt(start), dt(end)).unwrap();
        let id = ReservationId::new(Uuid::from_u128(10));
        let reservation = Reservation::new(
            id,
            RoomId::new(room()),
            UserId::new(user()),
            range,
            clock().now(),
        );
        store.save(&reservation).await.unwrap();
        id
    }

    fn room() -> Uuid {
        Uuid::from_u128(1)
    }

    fn user() -> Uuid {
        Uuid::from_u128(2)
    }

    #[tokio::test]
    async fn should_reject_a_reservation_that_overlaps_an_existing_confirmed_reservation() {
        let store = InMemoryReservationStore::default();
        seed_confirmed(&store, 10, 11).await;
        let handler = ReserveRoomHandler::new(Arc::new(store), Arc::new(clock()));
        let err = handler
            .handle(ReserveRoomCommand {
                room_id: room(),
                user_id: user(),
                start: dt(10) + chrono::Duration::minutes(30),
                end: dt(11) + chrono::Duration::minutes(30),
            })
            .await
            .unwrap_err();
        assert_eq!(err, ApplicationError::ReservationConflict);
    }

    #[tokio::test]
    async fn should_allow_a_reservation_that_only_touches_an_existing_range() {
        let store = InMemoryReservationStore::default();
        seed_confirmed(&store, 10, 11).await;
        let store = Arc::new(store);
        let handler = ReserveRoomHandler::new(store.clone(), Arc::new(clock()));
        let id = handler
            .handle(ReserveRoomCommand {
                room_id: room(),
                user_id: user(),
                start: dt(11),
                end: dt(12),
            })
            .await
            .unwrap();
        let saved = store.find_by_id(id).await.unwrap().unwrap();
        assert_eq!(saved.status(), ReservationStatus::Confirmed);
    }

    #[tokio::test]
    async fn should_allow_a_reservation_that_overlaps_a_cancelled_reservation() {
        let store = InMemoryReservationStore::default();
        let range = MeetingTimeRange::new(dt(10), dt(11)).unwrap();
        let cancelled = Reservation::new(
            ReservationId::new(Uuid::from_u128(10)),
            RoomId::new(room()),
            UserId::new(user()),
            range,
            clock().now(),
        )
        .cancel(clock().now())
        .unwrap();
        store.save(&cancelled).await.unwrap();
        let handler = ReserveRoomHandler::new(Arc::new(store), Arc::new(clock()));
        let result = handler
            .handle(ReserveRoomCommand {
                room_id: room(),
                user_id: user(),
                start: dt(10),
                end: dt(11),
            })
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn should_reject_a_command_whose_start_is_not_earlier_than_end() {
        let store = InMemoryReservationStore::default();
        let handler = ReserveRoomHandler::new(Arc::new(store), Arc::new(clock()));
        let err = handler
            .handle(ReserveRoomCommand {
                room_id: room(),
                user_id: user(),
                start: dt(10),
                end: dt(10),
            })
            .await
            .unwrap_err();
        assert_eq!(err, ApplicationError::Domain(DomainError::InvalidTimeRange));
    }
}
