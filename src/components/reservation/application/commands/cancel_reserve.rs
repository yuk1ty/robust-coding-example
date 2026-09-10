use std::sync::Arc;

use crate::components::reservation::application::ApplicationError;
use crate::components::reservation::application::out_ports::{Clock, ReservationRepository};
use crate::components::reservation::domain::ReservationId;
use uuid::Uuid;

pub struct CancelReservationCommand {
    pub reservation_id: Uuid,
}

pub struct CancelReservationHandler {
    repository: Arc<dyn ReservationRepository>,
    clock: Arc<dyn Clock>,
}

impl CancelReservationHandler {
    pub fn new(repository: Arc<dyn ReservationRepository>, clock: Arc<dyn Clock>) -> Self {
        Self { repository, clock }
    }

    pub async fn handle(&self, command: CancelReservationCommand) -> Result<(), ApplicationError> {
        let reservation = self
            .repository
            .find_by_id(ReservationId::new(command.reservation_id))
            .await?
            .ok_or(ApplicationError::ReservationNotFound)?;
        let cancelled = reservation.cancel(self.clock.now())?;
        self.repository.save(&cancelled).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::reservation::adapters::out_db::InMemoryReservationStore;
    use crate::components::reservation::application::out_ports::FixedClock;
    use crate::components::reservation::domain::{
        DomainError, MeetingTimeRange, Reservation, ReservationStatus, RoomId, UserId,
    };
    use chrono::{TimeZone, Utc};
    use std::sync::Arc;

    fn clock() -> FixedClock {
        FixedClock(Utc.with_ymd_and_hms(2026, 9, 9, 12, 0, 0).unwrap())
    }

    fn sample() -> Reservation {
        let start = Utc.with_ymd_and_hms(2026, 9, 10, 10, 0, 0).unwrap();
        let end = Utc.with_ymd_and_hms(2026, 9, 10, 11, 0, 0).unwrap();
        Reservation::new(
            ReservationId::new(Uuid::from_u128(10)),
            RoomId::new(Uuid::from_u128(1)),
            UserId::new(Uuid::from_u128(2)),
            MeetingTimeRange::new(start, end).unwrap(),
            Utc.with_ymd_and_hms(2026, 9, 1, 0, 0, 0).unwrap(),
        )
    }

    #[tokio::test]
    async fn should_return_not_found_when_the_reservation_does_not_exist() {
        let store = InMemoryReservationStore::default();
        let handler = CancelReservationHandler::new(Arc::new(store), Arc::new(clock()));
        let err = handler
            .handle(CancelReservationCommand {
                reservation_id: Uuid::from_u128(99),
            })
            .await
            .unwrap_err();
        assert_eq!(err, ApplicationError::ReservationNotFound);
    }

    #[tokio::test]
    async fn should_cancel_a_confirmed_reservation_using_injected_current_time() {
        let store = InMemoryReservationStore::default();
        let reservation = sample();
        let id = reservation.id().as_uuid();
        store.save(&reservation).await.unwrap();
        let store = Arc::new(store);
        let handler = CancelReservationHandler::new(store.clone(), Arc::new(clock()));
        handler
            .handle(CancelReservationCommand { reservation_id: id })
            .await
            .unwrap();
        let saved = store
            .find_by_id(ReservationId::new(id))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(saved.status(), ReservationStatus::Cancelled);
        assert_eq!(saved.cancelled_at(), Some(clock().now()));
    }

    #[tokio::test]
    async fn should_surface_already_cancelled_from_the_domain() {
        let store = InMemoryReservationStore::default();
        let reservation = sample().cancel(clock().now()).unwrap();
        store.save(&reservation).await.unwrap();
        let handler = CancelReservationHandler::new(Arc::new(store), Arc::new(clock()));
        let err = handler
            .handle(CancelReservationCommand {
                reservation_id: reservation.id().as_uuid(),
            })
            .await
            .unwrap_err();
        assert_eq!(err, ApplicationError::Domain(DomainError::AlreadyCancelled));
    }
}
