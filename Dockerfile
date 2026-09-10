FROM rust:1.91-bookworm AS builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates libssl3 \
    && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/robust-coding-example /usr/local/bin/app
COPY --from=builder /app/migrations /migrations
ENV APP_HOST=0.0.0.0
ENV APP_PORT=8080
EXPOSE 8080
CMD ["app"]
