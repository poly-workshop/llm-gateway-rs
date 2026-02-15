# ---- Build stage ----
FROM rust:1.85-slim AS builder

RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Cache dependencies
COPY Cargo.toml Cargo.lock* ./
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release && rm -rf src

# Build real binary
COPY src/ src/
COPY migrations/ migrations/
RUN touch src/main.rs && cargo build --release

# ---- Runtime stage ----
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/llm-gateway-rs /usr/local/bin/llm-gateway-rs

EXPOSE 8080

CMD ["llm-gateway-rs"]
