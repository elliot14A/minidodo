FROM lukemathwalker/cargo-chef:latest-rust-slim AS planner
WORKDIR /app
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM lukemathwalker/cargo-chef:latest-rust-slim AS builder
WORKDIR /app
RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*

COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json

COPY . .
RUN cargo build --release --bin minidodo

FROM debian:bookworm-slim AS runtime
WORKDIR /app
RUN apt-get update && apt-get install -y ca-certificates libssl3 postgresql-client && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/minidodo /usr/local/bin/minidodo

EXPOSE 3000

ENTRYPOINT ["minidodo"]
CMD ["server"]
