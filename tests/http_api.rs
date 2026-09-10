use chrono::{TimeZone, Utc};
use http_body_util::BodyExt;
use robust_coding_example::components::reservation::adapters::in_web::router;
use robust_coding_example::components::reservation::adapters::out_db::InMemoryReservationStore;
use serde_json::{Value, json};
use std::sync::Arc;
use tower::ServiceExt;
use uuid::Uuid;

fn app() -> axum::Router {
    router(Arc::new(InMemoryReservationStore::default()))
}

async fn body_json(response: axum::response::Response) -> Value {
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap_or(Value::Null)
}

fn rfc3339(hour: u32) -> String {
    Utc.with_ymd_and_hms(2026, 9, 10, hour, 0, 0)
        .unwrap()
        .to_rfc3339()
}

#[tokio::test]
async fn should_create_a_reservation_from_rfc3339_json() {
    let room = Uuid::from_u128(1);
    let user = Uuid::from_u128(2);
    let response = app()
        .oneshot(
            axum::http::Request::builder()
                .method("POST")
                .uri("/reservations")
                .header("content-type", "application/json")
                .body(axum::body::Body::from(
                    json!({
                        "room_id": room,
                        "user_id": user,
                        "start": rfc3339(10),
                        "end": rfc3339(11)
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), axum::http::StatusCode::CREATED);
    let body = body_json(response).await;
    assert_eq!(body["status"], "confirmed");
    assert_eq!(body["room_id"], room.to_string());
}

#[tokio::test]
async fn should_return_400_for_an_unparsable_start_timestamp() {
    let response = app()
        .oneshot(
            axum::http::Request::builder()
                .method("POST")
                .uri("/reservations")
                .header("content-type", "application/json")
                .body(axum::body::Body::from(
                    json!({
                        "room_id": Uuid::from_u128(1),
                        "user_id": Uuid::from_u128(2),
                        "start": "not-a-date",
                        "end": rfc3339(11)
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), axum::http::StatusCode::BAD_REQUEST);
    let body = body_json(response).await;
    assert_eq!(body["code"], "invalid_input");
}

#[tokio::test]
async fn should_return_409_when_the_room_is_already_reserved() {
    let app = app();
    let payload = json!({
        "room_id": Uuid::from_u128(1),
        "user_id": Uuid::from_u128(2),
        "start": rfc3339(10),
        "end": rfc3339(11)
    })
    .to_string();
    let first = app
        .clone()
        .oneshot(
            axum::http::Request::builder()
                .method("POST")
                .uri("/reservations")
                .header("content-type", "application/json")
                .body(axum::body::Body::from(payload.clone()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(first.status(), axum::http::StatusCode::CREATED);
    let second = app
        .oneshot(
            axum::http::Request::builder()
                .method("POST")
                .uri("/reservations")
                .header("content-type", "application/json")
                .body(axum::body::Body::from(payload))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(second.status(), axum::http::StatusCode::CONFLICT);
    let body = body_json(second).await;
    assert_eq!(body["code"], "reservation_conflict");
}

#[tokio::test]
async fn should_return_the_daily_room_schedule_for_a_timezone() {
    let app = app();
    let room = Uuid::from_u128(1);
    let create = app
        .clone()
        .oneshot(
            axum::http::Request::builder()
                .method("POST")
                .uri("/reservations")
                .header("content-type", "application/json")
                .body(axum::body::Body::from(
                    json!({
                        "room_id": room,
                        "user_id": Uuid::from_u128(2),
                        "start": "2026-09-10T01:00:00Z",
                        "end": "2026-09-10T02:00:00Z"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(create.status(), axum::http::StatusCode::CREATED);
    let response = app
        .oneshot(
            axum::http::Request::builder()
                .method("GET")
                .uri(format!(
                    "/rooms/{room}/schedule?date=2026-09-10&timezone=Asia/Tokyo"
                ))
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), axum::http::StatusCode::OK);
    let body = body_json(response).await;
    assert_eq!(body.as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn should_return_conflict_when_deleting_a_reservation_twice() {
    let app = app();
    let create = app
        .clone()
        .oneshot(
            axum::http::Request::builder()
                .method("POST")
                .uri("/reservations")
                .header("content-type", "application/json")
                .body(axum::body::Body::from(
                    json!({
                        "room_id": Uuid::from_u128(1),
                        "user_id": Uuid::from_u128(2),
                        "start": rfc3339(10),
                        "end": rfc3339(11)
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    let id = body_json(create).await["id"].as_str().unwrap().to_owned();
    let delete = |app: axum::Router| {
        let id = id.clone();
        async move {
            app.oneshot(
                axum::http::Request::builder()
                    .method("DELETE")
                    .uri(format!("/reservations/{id}"))
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap()
        }
    };
    let first = delete(app.clone()).await;
    assert_eq!(first.status(), axum::http::StatusCode::OK);
    let second = delete(app.clone()).await;
    assert_eq!(second.status(), axum::http::StatusCode::CONFLICT);
    let body = body_json(second).await;
    assert_eq!(body["code"].as_str().unwrap(), "already_cancelled");
    let missing = app
        .oneshot(
            axum::http::Request::builder()
                .method("DELETE")
                .uri(format!("/reservations/{}", Uuid::from_u128(99)))
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(missing.status(), axum::http::StatusCode::NOT_FOUND);
}
