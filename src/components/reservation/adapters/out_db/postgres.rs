use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

use crate::components::reservation::application::ApplicationError;
use crate::components::reservation::application::out_ports::{
    ReservationQueryService, ReservationRepository, ScheduleItem,
};
use crate::components::reservation::domain::{
    MeetingTimeRange, Reservation, ReservationId, ReservationStatus, RoomId, UserId,
};

#[derive(Clone)]
pub struct PgReservationStore {
    pool: PgPool,
}

impl PgReservationStore {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(FromRow)]
struct ReservationRow {
    id: Uuid,
    room_id: Uuid,
    user_id: Uuid,
    starts_at: DateTime<Utc>,
    ends_at: DateTime<Utc>,
    status: String,
    cancelled_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
}

impl TryFrom<ReservationRow> for Reservation {
    type Error = ApplicationError;

    fn try_from(row: ReservationRow) -> Result<Self, Self::Error> {
        let time_range = MeetingTimeRange::new(row.starts_at, row.ends_at)?;
        let status = ReservationStatus::parse(&row.status)?;
        Ok(Reservation::reconstitute(
            ReservationId::new(row.id),
            RoomId::new(row.room_id),
            UserId::new(row.user_id),
            time_range,
            status,
            row.cancelled_at,
            row.created_at,
        ))
    }
}

impl TryFrom<ReservationRow> for ScheduleItem {
    type Error = ApplicationError;

    fn try_from(row: ReservationRow) -> Result<Self, Self::Error> {
        Ok(ScheduleItem {
            id: ReservationId::new(row.id),
            room_id: RoomId::new(row.room_id),
            user_id: UserId::new(row.user_id),
            start: row.starts_at,
            end: row.ends_at,
            status: ReservationStatus::parse(&row.status)?,
            cancelled_at: row.cancelled_at,
        })
    }
}

fn map_sqlx(error: sqlx::Error) -> ApplicationError {
    if let sqlx::Error::Database(db) = &error
        && db.code().as_deref() == Some("23P01")
    {
        return ApplicationError::ReservationConflict;
    }
    tracing::error!(error = %error, "database error");
    ApplicationError::Infrastructure
}

#[async_trait::async_trait]
impl ReservationRepository for PgReservationStore {
    async fn find_by_id(&self, id: ReservationId) -> Result<Option<Reservation>, ApplicationError> {
        let row = sqlx::query_as::<_, ReservationRow>(
            r#"
            SELECT id, room_id, user_id, starts_at, ends_at, status, cancelled_at, created_at
            FROM reservations
            WHERE id = $1
            "#,
        )
        .bind(id.as_uuid())
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx)?;
        row.map(Reservation::try_from).transpose()
    }

    async fn find_confirmed_candidates(
        &self,
        room_id: RoomId,
        time_range: &MeetingTimeRange,
    ) -> Result<Vec<Reservation>, ApplicationError> {
        let rows = sqlx::query_as::<_, ReservationRow>(
            r#"
            SELECT id, room_id, user_id, starts_at, ends_at, status, cancelled_at, created_at
            FROM reservations
            WHERE room_id = $1
              AND status = 'confirmed'
              AND tstzrange(starts_at, ends_at, '[)') && tstzrange($2, $3, '[)')
            "#,
        )
        .bind(room_id.as_uuid())
        .bind(time_range.start())
        .bind(time_range.end())
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx)?;
        rows.into_iter().map(Reservation::try_from).collect()
    }

    async fn save(&self, reservation: &Reservation) -> Result<(), ApplicationError> {
        let mut tx = self.pool.begin().await.map_err(map_sqlx)?;
        sqlx::query(
            r#"
            INSERT INTO reservations (
                id, room_id, user_id, starts_at, ends_at, status, cancelled_at, created_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            ON CONFLICT (id) DO UPDATE SET
                status = EXCLUDED.status,
                cancelled_at = EXCLUDED.cancelled_at
            "#,
        )
        .bind(reservation.id().as_uuid())
        .bind(reservation.room_id().as_uuid())
        .bind(reservation.user_id().as_uuid())
        .bind(reservation.time_range().start())
        .bind(reservation.time_range().end())
        .bind(reservation.status().as_str())
        .bind(reservation.cancelled_at())
        .bind(reservation.created_at())
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        tx.commit().await.map_err(map_sqlx)?;
        Ok(())
    }
}

#[async_trait::async_trait]
impl ReservationQueryService for PgReservationStore {
    async fn find_schedule(
        &self,
        room_id: RoomId,
        range_start: DateTime<Utc>,
        range_end: DateTime<Utc>,
    ) -> Result<Vec<ScheduleItem>, ApplicationError> {
        let rows = sqlx::query_as::<_, ReservationRow>(
            r#"
            SELECT id, room_id, user_id, starts_at, ends_at, status, cancelled_at, created_at
            FROM reservations
            WHERE room_id = $1
              AND tstzrange(starts_at, ends_at, '[)') && tstzrange($2, $3, '[)')
            ORDER BY starts_at
            "#,
        )
        .bind(room_id.as_uuid())
        .bind(range_start)
        .bind(range_end)
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx)?;
        rows.into_iter().map(ScheduleItem::try_from).collect()
    }
}
