# Stage 1: Build
FROM debian:testing-slim AS builder

ARG VERSION=0.1.0
ENV VERSION=${VERSION}

ENV SHELL="/usr/bin/env bash"

RUN apt-get update -y
RUN apt-get install -y git gcc pkgconf pkgconf-bin openssl time && apt autoclean && apt autoremove && apt autopurge

ENV RUST_LOG="debug"
ENV RUST_BACKTRACE="full"

ENV CARGO_HOME="$HOME/rust" RUSTUP_HOME="$HOME/rustup" PATH="$PATH:$HOME/rust/bin"
RUN curl -fsSL https://sh.rustup.rs | bash -is -- -y --verbose --no-modify-path --default-toolchain stable --profile minimal
# RUN rustup -v toolchain install nightly --profile minimal
# RUN rustup target add x86_64-unknown-linux-musl

WORKDIR /app-builder

# COPY --link cdk-soa-backend/ /app-builder/cdk-soa-backend/
RUN git clone -b main https://github.com/andrcmdr/cdk-soa-backend.git

RUN <<EOF
#!/usr/bin/env bash

shopt -s extglob
shopt -s extquote
# shopt -s xpg_echo

set -f

cd /app-builder/cdk-soa-backend
git checkout v${VERSION}
cargo build --release --all
mv -T /app-builder/cdk-soa-backend/target/release/cdk-indexer /app-builder/cdk-indexer
mv -T /app-builder/cdk-soa-backend/cdk-indexer.v1.legacy/config.toml /app-builder/config.toml
cp -vrf /app-builder/cdk-soa-backend/cdk-indexer.v1.legacy/abi/ -T /app-builder/abi/
EOF


# Stage 2: Runtime
FROM debian:testing-slim AS cdk_indexer_app

ENV SHELL="/usr/bin/env bash"

WORKDIR /apps

RUN mkdir -vp /apps/.logs/

COPY --from=builder /app-builder/cdk-indexer /apps/cdk-indexer
COPY --from=builder /app-builder/config.toml /apps/config.toml
COPY --from=builder /app-builder/abi/ /apps/abi/

# Install libpq for tokio-postgres
RUN apt-get update -y
RUN apt-get install -y libpq-dev libpq5 time && apt autoclean && apt autoremove && apt autopurge

# ENV RUST_LOG="cdk_indexer=debug"
ENV RUST_LOG="debug"
ENV RUST_BACKTRACE="full"
CMD cd /apps/; ./cdk-indexer >> /apps/.logs/cdk-indexer.log 2>&1 & disown; tail -f /apps/.logs/cdk-indexer.log
