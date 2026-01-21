# Stage 1: Build
FROM rust:1.91 as builder

WORKDIR /usr/src/app

# Pre-cache dependencies
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release
RUN rm -rf src

# Copy actual source and rebuild
COPY src ./src
RUN cargo build --release

# Stage 2: Runtime
FROM debian:testing-slim

# Install libpq for tokio-postgres and debugging tools
RUN apt-get update && apt-get install -y \
    libpq-dev \
    procps \
    net-tools \
    curl \
 && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=builder /usr/src/app/target/release/oracle-service ./oracle-service
RUN chmod +x ./oracle-service

EXPOSE 8080
# Run the Oracle Service
CMD ["./oracle-service"]
