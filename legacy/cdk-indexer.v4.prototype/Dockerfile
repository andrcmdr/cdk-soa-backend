# ---------- Build ----------
FROM rust:1.80 as builder
WORKDIR /app

# Cache deps
COPY Cargo.toml Cargo.lock* ./
RUN mkdir -p src && echo "fn main(){}" > src/main.rs && cargo build --release && rm -rf src

# Build
COPY . .
RUN cargo build --release

# ---------- Runtime ----------
FROM gcr.io/distroless/cc-debian12:nonroot
WORKDIR /app
COPY --from=builder /app/target/release/cdk-indexer /app/cdk-indexer
COPY --from=builder /app/config.yaml /app/config.yaml
COPY --from=builder /app/schema.sql /app/schema.sql
COPY --from=builder /app/abi /app/abi
USER nonroot:nonroot
ENTRYPOINT ["/app/cdk-indexer"]
