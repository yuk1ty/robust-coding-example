use chrono::{TimeZone, Utc};
use robust_coding_example::components::reservation::adapters::out_db::PgReservationStore;
use robust_coding_example::components::reservation::application::ApplicationError;
use robust_coding_example::components::reservation::application::out_ports::ReservationRepository;
use robust_coding_example::components::reservation::domain::{
    MeetingTimeRange, Reservation, ReservationId, ReservationStatus, RoomId, UserId,
};
use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;
use testcontainers::runners::AsyncRunner;
use testcontainers_modules::postgres::Postgres;
use uuid::Uuid;

async fn setup_pool() -> (PgPool, testcontainers::ContainerAsync<Postgres>) {
    let container = Postgres::default()
        .start()
        .await
        .expect("postgres container");
    let host = container.get_host().await.expect("host");
    let port = container.get_host_port_ipv4(5432).await.expect("port");
    let url = format!("postgres://postgres:postgres@{host}:{port}/postgres");
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&url)
        .await
        .expect("connect");
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("migrate");
    (pool, container)
}

fn range(start_hour: u32, end_hour: u32) -> MeetingTimeRange {
    let start = Utc.with_ymd_and_hms(2026, 9, 10, start_hour, 0, 0).unwrap();
    let end = Utc.with_ymd_and_hms(2026, 9, 10, end_hour, 0, 0).unwrap();
    MeetingTimeRange::new(start, end).unwrap()
}

fn reservation(id: u128, range: MeetingTimeRange) -> Reservation {
    Reservation::new(
        ReservationId::new(Uuid::from_u128(id)),
        RoomId::new(Uuid::from_u128(1)),
        UserId::new(Uuid::from_u128(2)),
        range,
        Utc.with_ymd_and_hms(2026, 9, 1, 0, 0, 0).unwrap(),
    )
}

#[tokio::test]
#[ignore = "requires Docker for Testcontainers PostgreSQL"]
async fn should_apply_migrations_on_a_fresh_database() {
    let (pool, _container) = setup_pool().await;
    let exists: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'reservations')",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(exists);
}

#[tokio::test]
#[ignore = "requires Docker for Testcontainers PostgreSQL"]
async fn should_save_and_load_a_confirmed_reservation() {
    let (pool, _container) = setup_pool().await;
    let store = PgReservationStore::new(pool);
    let original = reservation(1, range(10, 11));
    store.save(&original).await.unwrap();
    let loaded = store.find_by_id(original.id()).await.unwrap().unwrap();
    assert_eq!(loaded.status(), ReservationStatus::Confirmed);
    assert_eq!(loaded.time_range().start(), original.time_range().start());
}

#[tokio::test]
#[ignore = "requires Docker for Testcontainers PostgreSQL"]
async fn should_reject_overlapping_confirmed_reservations_at_the_database() {
    let (pool, _container) = setup_pool().await;
    let store = PgReservationStore::new(pool);
    store.save(&reservation(1, range(10, 11))).await.unwrap();
    let err = store
        .save(&reservation(2, range(10, 11)))
        .await
        .unwrap_err();
    assert_eq!(err, ApplicationError::ReservationConflict);
}

#[tokio::test]
#[ignore = "requires Docker for Testcontainers PostgreSQL"]
async fn should_allow_a_confirmed_reservation_overlapping_a_cancelled_row() {
    let (pool, _container) = setup_pool().await;
    let store = PgReservationStore::new(pool);
    let cancelled = reservation(1, range(10, 11))
        .cancel(Utc.with_ymd_and_hms(2026, 9, 9, 0, 0, 0).unwrap())
        .unwrap();
    store.save(&cancelled).await.unwrap();
    store.save(&reservation(2, range(10, 11))).await.unwrap();
}

#[tokio::test]
#[ignore = "requires Docker for Testcontainers PostgreSQL"]
async fn should_allow_adjacent_confirmed_reservations() {
    let (pool, _container) = setup_pool().await;
    let store = PgReservationStore::new(pool);
    store.save(&reservation(1, range(10, 11))).await.unwrap();
    store.save(&reservation(2, range(11, 12))).await.unwrap();
}
