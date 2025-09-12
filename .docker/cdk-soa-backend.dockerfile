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

RUN mkdir -vp /app-builder/.config/

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
mv -T /app-builder/cdk-soa-backend/target/release/events-monitor /app-builder/events-monitor
mv -T /app-builder/cdk-soa-backend/events-monitor/.config/events_monitor.config.yaml /app-builder/.config/events_monitor.config.yaml
mv -T /app-builder/cdk-soa-backend/target/release/abi-fetcher /app-builder/abi-fetcher
mv -T /app-builder/cdk-soa-backend/abi-fetcher/.config/abi_fetcher.config.yaml /app-builder/.config/abi_fetcher.config.yaml
# cp -vrf /app-builder/cdk-soa-backend/events-monitor/abi/ -T /app-builder/abi/
mv -T /app-builder/cdk-soa-backend/target/release/json-rpc-fetcher /app-builder/json-rpc-fetcher
mv -T /app-builder/cdk-soa-backend/abi-fetcher/.config/json_rpc_fetcher.config.yaml /app-builder/.config/json_rpc_fetcher.config.yaml
EOF

COPY --link abi-fetcher/abi/ /app-builder/abi/


# Stage 2: Runtime
FROM debian:testing-slim AS events_monitor_app

ENV SHELL="/usr/bin/env bash"

WORKDIR /apps

RUN mkdir -vp /apps/.config/
RUN mkdir -vp /apps/.logs/

COPY --from=builder /app-builder/events-monitor /apps/events-monitor
COPY --from=builder /app-builder/.config/events_monitor.config.yaml /apps/.config/events_monitor.config.yaml
COPY --from=builder /app-builder/abi-fetcher /apps/abi-fetcher
COPY --from=builder /app-builder/.config/abi_fetcher.config.yaml /apps/.config/abi_fetcher.config.yaml
COPY --from=builder /app-builder/abi/ /apps/abi/
COPY --from=builder /app-builder/json-rpc-fetcher /apps/json-rpc-fetcher
COPY --from=builder /app-builder/.config/json_rpc_fetcher.config.yaml /apps/.config/json_rpc_fetcher.config.yaml

# Install libpq for tokio-postgres
RUN apt-get update -y
RUN apt-get install -y libpq-dev libpq5 time && apt autoclean && apt autoremove && apt autopurge

# ENV RUST_LOG="events_monitor=debug"
ENV RUST_LOG="debug"
ENV RUST_BACKTRACE="full"
CMD cd /apps/; ./events-monitor ./.config/events_monitor.config.yaml >> /apps/.logs/events-monitor.log 2>&1 & disown; tail -f /apps/.logs/events-monitor.log
