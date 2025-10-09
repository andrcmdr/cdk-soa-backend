# Stage 1: Build Rust Apps
FROM debian:testing-slim AS rust_builder

ARG VERSION=0.7.0
ENV VERSION=${VERSION}

ENV SHELL="/usr/bin/env bash"

RUN apt-get update -y
RUN apt-get install -y git gcc pkgconf pkgconf-bin openssl time curl make && apt autoclean && apt autoremove && apt autopurge

ENV RUST_LOG="debug"
ENV RUST_BACKTRACE="full"

ENV CARGO_HOME="/opt/rust" RUSTUP_HOME="/opt/rustup" PATH="$PATH:/opt/rust/bin"
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

cd /app-builder/cdk-soa-backend/merkle-trie-cli/
# git checkout v${VERSION}
# cargo build --release --all
cargo build --all
mv -T /app-builder/cdk-soa-backend/target/debug/merkle-cli /app-builder/merkle-cli
mv -T /app-builder/cdk-soa-backend/target/debug/merkle-cli-ref /app-builder/merkle-cli-ref
mv -T /app-builder/cdk-soa-backend/target/debug/merkle-cli-viem-compat /app-builder/merkle-cli-viem-compat
EOF

# Stage 2: Runtime
FROM debian:testing-slim AS apps_runtime

ENV SHELL="/usr/bin/env bash"

WORKDIR /apps

COPY --from=rust_builder /app-builder/merkle-cli /apps/merkle-cli
COPY --from=rust_builder /app-builder/merkle-cli-ref /apps/merkle-cli-ref
COPY --from=rust_builder /app-builder/merkle-cli-viem-compat /apps/merkle-cli-viem-compat

RUN apt-get update -y
RUN apt-get install -y time && apt autoclean && apt autoremove && apt autopurge

ENV RUST_LOG="debug"
ENV RUST_BACKTRACE="full"

# CMD sleep infinity
CMD tail -f /dev/null
