# Architecture

## Overview

Conference-room reservation sample in Rust. The codebase demonstrates **Explicit Architecture** (package-by-component, ports and adapters, CQRS) with **Data / Calculation / Action** separation, an **always-valid domain model**, and **parse-don't-validate** at boundaries.

The binary (`src/main.rs`) loads config, connects to PostgreSQL, runs SQLx migrations, then serves an Axum router.

## Layout

```
src/
  lib.rs                     # library root; re-exports router, PgReservationStore, AppConfig
  main.rs                    # binary: env → PgPool → migrate → axum::serve
  config.rs                  # APP_HOST / APP_PORT / DATABASE_URL
  components/
    reservation/
      domain/                # pure domain; no framework dependencies
        ids.rs               # ReservationId / RoomId / UserId (newtypes)
        error.rs             # DomainError (thiserror)
        value_objects.rs     # MeetingTimeRange, ReservationStatus, CancellationFee
        reservation.rs       # Reservation aggregate
      application/
        error.rs             # ApplicationError
        commands/            # CQRS write path
          reserve_room.rs
          cancel_reserve.rs
        queries/             # CQRS read path
          get_room_schedule.rs
        out_ports/           # outbound traits (async, dyn-compatible via async-trait)
          clock.rs           # Clock, SystemClock, FixedClock
          reservation_repository.rs
          reservation_read.rs
      adapters/
        in_web/              # Axum inbound adapter
          routes.rs          # router + handlers
          dto.rs             # request/response DTOs
          error.rs           # ApiError mapping
        out_db/
          in_memory.rs       # test double for both ports
          postgres.rs        # SQLx adapter
migrations/                  # SQLx SQL migrations
tests/                       # crate, HTTP (in-memory), Postgres (ignored without Docker)
```

## Dependency rule

Dependencies point inward. The domain does not import Axum, SQLx, or `Utc::now()`. The application layer depends on domain types and outbound traits. Adapters implement those traits and map external data into domain constructors.

```
Axum handlers → Commands / Queries → Domain
                     ↓
              Outbound traits
                     ↓
        PgReservationStore (SQLx) / InMemoryReservationStore
```

## Data, Calculation, Action

- **Data:** `MeetingTimeRange`, `ReservationStatus`, `CancellationFee`, DTOs, `ScheduleItem`.
- **Calculations:** `MeetingTimeRange::new`, `overlaps`, `Reservation::cancel`, `civil_day_bounds`.
- **Actions:** repository I/O, `Uuid::new_v4()`, `Clock::now()`, HTTP, SQLx, migrations.

`Clock` is injected into handlers. Tests use `FixedClock`. UUID generation happens in `ReserveRoomHandler::handle`, not in the domain.

## Domain model

`Reservation` keeps private fields:

```rust
pub struct Reservation {
    id: ReservationId,
    room_id: RoomId,
    user_id: UserId,
    time_range: MeetingTimeRange,
    status: ReservationStatus,
    cancelled_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
}
```

ID newtypes are domain-internal. `Reservation::new` and `reconstitute` take `Uuid` at the boundary and wrap them; getters return `Uuid`. `MeetingTimeRange::new` rejects `start >= end`. `cancel` sets status and `cancelled_at`, and returns `AlreadyCancelled` on a second call.

`CancellationFee` exists as a value object only. It is not wired to handlers, persistence, or HTTP.

## CQRS

- **Write:** `POST /reservations` → `ReserveRoomHandler` → domain checks → `ReservationRepository::save`.
- **Write:** `DELETE /reservations/{id}` → `CancelReservationHandler` → load → `cancel(clock.now())` → save. A second cancel returns `409 already_cancelled`.
- **Read:** `GET /rooms/{room_id}/schedule?date=...&timezone=...` → `GetRoomScheduleHandler` → `ReservationReadPort` → `ScheduleItem` DTOs. The query path does not construct `Reservation`.

## Ports and adapters

Outbound traits are async and dyn-compatible via `#[async_trait]`. Handlers depend on them through `Arc<dyn …>` (dynamic dispatch):

```rust
pub trait ReservationRepository: Send + Sync {
    async fn find_by_id(&self, id: ReservationId) -> Result<Option<Reservation>, ApplicationError>;
    async fn find_confirmed_candidates(&self, room_id: RoomId, time_range: &MeetingTimeRange)
        -> Result<Vec<Reservation>, ApplicationError>;
    async fn save(&self, reservation: &Reservation) -> Result<(), ApplicationError>;
}

pub trait ReservationReadPort: Send + Sync {
    async fn find_schedule(&self, room_id: RoomId, range_start: DateTime<Utc>, range_end: DateTime<Utc>)
        -> Result<Vec<ScheduleItem>, ApplicationError>;
}
```

Implementations:

- `PgReservationStore` — SQLx, parameterized queries, `TryFrom<ReservationRow>` re-parses rows into domain types.
- `InMemoryReservationStore` — `HashMap` behind `Arc<Mutex<…>>`, used by unit and HTTP tests.

## HTTP boundary

Parsing happens in the inbound adapter. JSON `room_id` / `user_id` / `start` / `end` become `Uuid` and `DateTime<Utc>` before commands are built. `JsonRejection` and `QueryRejection` map to `400 invalid_input`. Unknown timezone maps to `400 invalid_timezone`.

Error mapping (`ApiError`):

| Condition | Status | Code |
| --- | --- | --- |
| Invalid JSON / query | 400 | `invalid_input` |
| Invalid time range | 400 | `invalid_time_range` |
| Invalid schedule window | 400 | `invalid_schedule_window` |
| Reservation not found | 404 | `reservation_not_found` |
| Overlap / conflict | 409 | `reservation_conflict` |
| Already cancelled | 409 | `already_cancelled` |
| Infrastructure / unexpected | 500 | `internal_error` |

## PostgreSQL

Migrations enable `btree_gist`, then create `reservations` with:

- `starts_at < ends_at` check
- `status` as `TEXT` with `CHECK (status IN ('confirmed', 'cancelled'))`
- Partial exclusion constraint so only confirmed rows block overlap:

```sql
EXCLUDE USING GIST (
    room_id WITH =,
    tstzrange(starts_at, ends_at, '[)') WITH &&
) WHERE (status = 'confirmed');
```

`[)` matches the domain rule that adjacent slots do not overlap. SQLSTATE `23P01` maps to `ReservationConflict`. `save` uses a transaction and `ON CONFLICT (id) DO UPDATE` for status and `cancelled_at`.

## Configuration and deployment

`AppConfig::from_env` reads `APP_HOST`, `APP_PORT`, and `DATABASE_URL`. Docker Compose runs `postgres:16` with a healthcheck, then the app container. The app runs `sqlx::migrate!()` before binding the listener.

## Testing

- Unit tests live next to modules (`#[cfg(test)]`).
- `tests/http_api.rs` uses `InMemoryReservationStore` and runs without Docker.
- `tests/postgres_integration.rs` uses Testcontainers and is `#[ignore]` unless Docker is available:

```bash
cargo test
cargo test --test postgres_integration -- --ignored
```

## Explicitly out of scope

Authentication, Room/User aggregates or tables, cancellation-fee calculation or events, reservation update/restore, Actix.
