# Stage 1: Build
FROM rust:1.88 as builder

WORKDIR /usr/src/app

# Pre-cache dependencies
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release

# Copy actual source and rebuild
COPY . .
RUN cargo build --release

# Stage 2: Runtime
FROM debian:testing-slim

# Install libpq for tokio-postgres
RUN apt-get update && apt-get install -y \
    libpq-dev \
 && rm -rf /var/lib/apt/lists/*

COPY --from=builder /usr/src/app/target/release/oracle-service /usr/local/bin/oracle-service

WORKDIR /app
COPY config.toml ./config.toml

ENTRYPOINT ["/usr/local/bin/oracle-service"]
