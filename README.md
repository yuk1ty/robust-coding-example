# Conference Room Reservation Sample

Rust sample of Explicit Architecture (package-by-component, ports and adapters, CQRS) for conference-room reservations.

## Requirements

- Rust 1.85+ (edition 2024)
- Docker (for PostgreSQL, Compose, and Testcontainers)

## Configuration

| Variable | Description | Default |
| --- | --- | --- |
| `APP_HOST` | Bind host | `0.0.0.0` |
| `APP_PORT` | Bind port | `8080` |
| `DATABASE_URL` | PostgreSQL URL | required at runtime |

## Docker

```bash
docker compose up --build
```

The app waits for Postgres health, then runs SQLx migrations on startup.

## Tests

Unit tests (no Docker):

```bash
cargo test --lib --bins --test crate_compiles
```

PostgreSQL and HTTP tests start Postgres via Testcontainers:

```bash
cargo test
```

PostgreSQL adapter tests need Docker (Testcontainers):

```bash
cargo test --test postgres_integration -- --ignored
```

## API examples

Create:

```bash
curl -sS -X POST http://localhost:8080/reservations \
  -H 'content-type: application/json' \
  -d '{
    "room_id": "00000000-0000-0000-0000-000000000001",
    "user_id": "00000000-0000-0000-0000-000000000002",
    "start": "2026-09-10T10:00:00Z",
    "end": "2026-09-10T11:00:00Z"
  }'
```

Schedule for a civil day in Tokyo (omit `timezone` to use UTC):

```bash
curl -sS 'http://localhost:8080/rooms/00000000-0000-0000-0000-000000000001/schedule?date=2026-09-10&timezone=Asia/Tokyo'
```

Cancel (a second cancel returns `409 already_cancelled`):

```bash
curl -sS -X DELETE http://localhost:8080/reservations/<reservation-id>
```

Errors use `{ "code", "message", "details"? }` with `400` / `404` / `409` / `500`.
