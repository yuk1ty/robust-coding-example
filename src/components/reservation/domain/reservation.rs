use chrono::{DateTime, Utc};

use super::error::DomainError;
use super::ids::{ReservationId, RoomId, UserId};
use super::value_objects::{MeetingTimeRange, ReservationStatus};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reservation {
    id: ReservationId,
    room_id: RoomId,
    user_id: UserId,
    time_range: MeetingTimeRange,
    status: ReservationStatus,
    cancelled_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
}

impl Reservation {
    pub fn new(
        id: ReservationId,
        room_id: RoomId,
        user_id: UserId,
        time_range: MeetingTimeRange,
        created_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            room_id,
            user_id,
            time_range,
            status: ReservationStatus::Confirmed,
            cancelled_at: None,
            created_at,
        }
    }

    pub fn reconstitute(
        id: ReservationId,
        room_id: RoomId,
        user_id: UserId,
        time_range: MeetingTimeRange,
        status: ReservationStatus,
        cancelled_at: Option<DateTime<Utc>>,
        created_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            room_id,
            user_id,
            time_range,
            status,
            cancelled_at,
            created_at,
        }
    }

    pub fn id(&self) -> ReservationId {
        self.id
    }

    pub fn room_id(&self) -> RoomId {
        self.room_id
    }

    pub fn user_id(&self) -> UserId {
        self.user_id
    }

    pub fn time_range(&self) -> &MeetingTimeRange {
        &self.time_range
    }

    pub fn status(&self) -> ReservationStatus {
        self.status
    }

    pub fn cancelled_at(&self) -> Option<DateTime<Utc>> {
        self.cancelled_at
    }

    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    pub fn cancel(self, current_time: DateTime<Utc>) -> Result<Self, DomainError> {
        if self.status == ReservationStatus::Cancelled {
            return Err(DomainError::AlreadyCancelled);
        }
        Ok(Self {
            status: ReservationStatus::Cancelled,
            cancelled_at: Some(current_time),
            ..self
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use uuid::Uuid;

    fn range() -> MeetingTimeRange {
        let start = Utc.with_ymd_and_hms(2026, 9, 10, 10, 0, 0).unwrap();
        let end = Utc.with_ymd_and_hms(2026, 9, 10, 11, 0, 0).unwrap();
        MeetingTimeRange::new(start, end).unwrap()
    }

    fn reservation() -> Reservation {
        Reservation::new(
            ReservationId::new(Uuid::nil()),
            RoomId::new(Uuid::from_u128(1)),
            UserId::new(Uuid::from_u128(2)),
            range(),
            Utc.with_ymd_and_hms(2026, 9, 1, 0, 0, 0).unwrap(),
        )
    }

    #[test]
    fn should_create_a_confirmed_reservation() {
        let reservation = reservation();
        assert_eq!(reservation.status(), ReservationStatus::Confirmed);
        assert_eq!(reservation.room_id(), RoomId::new(Uuid::from_u128(1)));
        assert!(reservation.cancelled_at().is_none());
    }

    #[test]
    fn should_cancel_a_confirmed_reservation() {
        let now = Utc.with_ymd_and_hms(2026, 9, 9, 12, 0, 0).unwrap();
        let cancelled = reservation().cancel(now).unwrap();
        assert_eq!(cancelled.status(), ReservationStatus::Cancelled);
        assert_eq!(cancelled.cancelled_at(), Some(now));
    }

    #[test]
    fn should_reject_cancelling_an_already_cancelled_reservation() {
        let now = Utc.with_ymd_and_hms(2026, 9, 9, 12, 0, 0).unwrap();
        let cancelled = reservation().cancel(now).unwrap();
        let err = cancelled.cancel(now).unwrap_err();
        assert_eq!(err, DomainError::AlreadyCancelled);
    }
}
