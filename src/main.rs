use robust_coding_example::AppConfig;
use robust_coding_example::components::reservation::adapters::in_web::router;
use robust_coding_example::components::reservation::adapters::out_db::PgReservationStore;
use sqlx::postgres::PgPoolOptions;
use std::sync::Arc;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();

    let config = AppConfig::from_env().expect("invalid configuration");
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&config.database_url)
        .await
        .expect("failed to connect to postgres");

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("failed to run migrations");

    let store = PgReservationStore::new(pool);
    let app = router(Arc::new(store));
    let listener = TcpListener::bind(config.bind_addr)
        .await
        .expect("failed to bind address");
    tracing::info!("listening on {}", config.bind_addr);
    axum::serve(listener, app).await.expect("server error");
}
