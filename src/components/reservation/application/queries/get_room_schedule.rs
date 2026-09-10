use std::sync::Arc;

use chrono::{DateTime, MappedLocalTime, NaiveDate, NaiveTime, Utc};
use chrono_tz::Tz;
use uuid::Uuid;

use crate::components::reservation::application::ApplicationError;
use crate::components::reservation::application::out_ports::{
    ReservationQueryService, ScheduleItem,
};
use crate::components::reservation::domain::RoomId;

pub struct GetRoomScheduleQuery {
    pub room_id: Uuid,
    pub date: NaiveDate,
    pub timezone: Tz,
}

pub struct GetRoomScheduleHandler {
    query_service: Arc<dyn ReservationQueryService>,
}

impl GetRoomScheduleHandler {
    pub fn new(query_service: Arc<dyn ReservationQueryService>) -> Self {
        Self { query_service }
    }

    pub async fn handle(
        &self,
        query: GetRoomScheduleQuery,
    ) -> Result<Vec<ScheduleItem>, ApplicationError> {
        let (start, end) = civil_day_bounds(query.date, query.timezone)?;
        self.query_service
            .find_schedule(RoomId::new(query.room_id), start, end)
            .await
    }
}

fn civil_day_bounds(
    date: NaiveDate,
    timezone: Tz,
) -> Result<(DateTime<Utc>, DateTime<Utc>), ApplicationError> {
    let start = local_midnight(date, timezone)?;
    let end_date = date
        .succ_opt()
        .ok_or(ApplicationError::InvalidScheduleWindow)?;
    let end = local_midnight(end_date, timezone)?;
    Ok((start, end))
}

fn local_midnight(date: NaiveDate, timezone: Tz) -> Result<DateTime<Utc>, ApplicationError> {
    let naive = date.and_time(NaiveTime::MIN);
    let local = match naive.and_local_timezone(timezone) {
        MappedLocalTime::Single(dt) => dt,
        MappedLocalTime::Ambiguous(earliest, _) => earliest,
        MappedLocalTime::None => return Err(ApplicationError::InvalidScheduleWindow),
    };
    Ok(local.with_timezone(&Utc))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::reservation::domain::{
        ReservationId, ReservationStatus, RoomId, UserId,
    };
    use std::sync::Arc;
    use std::sync::Mutex;

    struct FakeReadPort {
        items: Mutex<Vec<ScheduleItem>>,
    }

    #[async_trait::async_trait]
    impl ReservationQueryService for FakeReadPort {
        async fn find_schedule(
            &self,
            room_id: RoomId,
            range_start: DateTime<Utc>,
            range_end: DateTime<Utc>,
        ) -> Result<Vec<ScheduleItem>, ApplicationError> {
            let items = self.items.lock().unwrap();
            Ok(items
                .iter()
                .filter(|item| {
                    item.room_id == room_id && item.start < range_end && range_start < item.end
                })
                .cloned()
                .collect())
        }
    }

    #[tokio::test]
    async fn should_return_reservations_for_a_civil_day_in_the_given_timezone() {
        let room = Uuid::from_u128(1);
        let tokyo_morning = DateTime::parse_from_rfc3339("2026-09-09T16:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let tokyo_afternoon = DateTime::parse_from_rfc3339("2026-09-10T03:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let next_day = DateTime::parse_from_rfc3339("2026-09-10T16:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let port = FakeReadPort {
            items: Mutex::new(vec![
                ScheduleItem {
                    id: ReservationId::new(Uuid::from_u128(1)),
                    room_id: RoomId::new(room),
                    user_id: UserId::new(Uuid::from_u128(2)),
                    start: tokyo_morning,
                    end: tokyo_morning + chrono::Duration::hours(1),
                    status: ReservationStatus::Confirmed,
                    cancelled_at: None,
                },
                ScheduleItem {
                    id: ReservationId::new(Uuid::from_u128(2)),
                    room_id: RoomId::new(room),
                    user_id: UserId::new(Uuid::from_u128(2)),
                    start: tokyo_afternoon,
                    end: tokyo_afternoon + chrono::Duration::hours(1),
                    status: ReservationStatus::Confirmed,
                    cancelled_at: None,
                },
                ScheduleItem {
                    id: ReservationId::new(Uuid::from_u128(3)),
                    room_id: RoomId::new(room),
                    user_id: UserId::new(Uuid::from_u128(2)),
                    start: next_day,
                    end: next_day + chrono::Duration::hours(1),
                    status: ReservationStatus::Confirmed,
                    cancelled_at: None,
                },
            ]),
        };
        let handler = GetRoomScheduleHandler::new(Arc::new(port));
        let items = handler
            .handle(GetRoomScheduleQuery {
                room_id: room,
                date: NaiveDate::from_ymd_opt(2026, 9, 10).unwrap(),
                timezone: "Asia/Tokyo".parse().unwrap(),
            })
            .await
            .unwrap();
        let ids: Vec<_> = items.iter().map(|item| item.id).collect();
        assert_eq!(
            ids,
            vec![
                ReservationId::new(Uuid::from_u128(1)),
                ReservationId::new(Uuid::from_u128(2))
            ]
        );
        assert!(
            items
                .iter()
                .all(|item| matches!(item.status, ReservationStatus::Confirmed))
        );
    }
}
