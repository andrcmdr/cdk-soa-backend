# Stage 1: Build Rust Apps
FROM debian:testing-slim AS rust_builder

ARG VERSION=0.4.0
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
# git checkout v${VERSION}
# cargo build --release --all
cargo build --all
mv -T /app-builder/cdk-soa-backend/target/debug/events-monitor /app-builder/events-monitor
mv -T /app-builder/cdk-soa-backend/events-monitor/.config/events_monitor.config.yaml /app-builder/.config/events_monitor.config.yaml
mv -T /app-builder/cdk-soa-backend/target/debug/abi-fetcher /app-builder/abi-fetcher
mv -T /app-builder/cdk-soa-backend/abi-fetcher/.config/abi_fetcher.config.yaml /app-builder/.config/abi_fetcher.config.yaml
cp -vrf /app-builder/cdk-soa-backend/events-monitor/abi/ -T /app-builder/abi/
mv -T /app-builder/cdk-soa-backend/target/debug/contracts-fetcher /app-builder/contracts-fetcher
mv -T /app-builder/cdk-soa-backend/abi-fetcher/.config/contracts_fetcher.config.yaml /app-builder/.config/contracts_fetcher.config.yaml
EOF

# COPY --link abi-fetcher/abi/ /app-builder/abi/


# Stage 2: Build Golang and CLang Apps
FROM debian:testing-slim AS go_builder

ENV SHELL="/usr/bin/env bash"

RUN apt update -y
RUN apt install -y git gcc
RUN apt install -y wget tar

WORKDIR /app-builder

RUN mkdir -vp /app-builder

ARG GO_VERSION=1.25.1
ENV GO_VERSION=${GO_VERSION}

RUN wget -c --trust-server-names --content-disposition https://go.dev/dl/go${GO_VERSION}.linux-amd64.tar.gz
RUN rm -rf /opt/go && tar -C /opt/ -xzf go${GO_VERSION}.linux-amd64.tar.gz

ENV PATH="$PATH:/opt/go/bin"

# COPY --link app.go/ /app-builder/app.go/
RUN git clone -o github https://github.com/nats-io/nats-server.git ./nats-server/

RUN <<EOF
#!/usr/bin/env bash

shopt -s extglob
shopt -s extquote
# shopt -s xpg_echo

set -f

cd /app-builder/nats-server/;
mkdir -vp ./build/;
git checkout $(git tag --sort="-version:refname" | grep -iP "^v?[0-9]+\.?[0-9]+?\.?[0-9]*?$" | awk "NR==1{print \$1}");
go mod download -x;
CGO_ENABLED=0 go build -v -x -a -trimpath -ldflags "-s -w -extldflags=-static \
-X main.Version=$(git describe --tags) \
-X main.version=$(git describe --tags) \
-X github.com/nats-io/nats-server/v2/server.serverVersion=$(git describe --tags) \
-X github.com/nats-io/nats-server/v2/server.gitCommit=$(git rev-parse HEAD)" -o ./build/nats-server ./main.go;
mkdir -vp /app-builder/nats/;
mv -T /app-builder/nats-server/build/nats-server /app-builder/nats/nats-server
EOF


# Stage 3: Runtime
FROM debian:testing-slim AS apps_runtime

ENV SHELL="/usr/bin/env bash"

WORKDIR /apps

RUN mkdir -vp /apps/.config/
RUN mkdir -vp /apps/.logs/

COPY --from=rust_builder /app-builder/events-monitor /apps/events-monitor
COPY --from=rust_builder /app-builder/.config/events_monitor.config.yaml /apps/.config/events_monitor.config.yaml
COPY --from=rust_builder /app-builder/abi-fetcher /apps/abi-fetcher
COPY --from=rust_builder /app-builder/.config/abi_fetcher.config.yaml /apps/.config/abi_fetcher.config.yaml
COPY --from=rust_builder /app-builder/abi/ /apps/abi/
COPY --from=rust_builder /app-builder/contracts-fetcher /apps/contracts-fetcher
COPY --from=rust_builder /app-builder/.config/contracts_fetcher.config.yaml /apps/.config/contracts_fetcher.config.yaml

COPY --from=go_builder /app-builder/nats/nats-server /apps/nats-server

# Install libpq for tokio-postgres
RUN apt-get update -y
RUN apt-get install -y libpq-dev libpq5 time && apt autoclean && apt autoremove && apt autopurge

EXPOSE 4222 4242 8222 6222

# ENV RUST_LOG="events_monitor=debug"
ENV RUST_LOG="debug"
ENV RUST_BACKTRACE="full"
CMD cd /apps/; ./events-monitor ./.config/events_monitor.config.yaml >> /apps/.logs/events-monitor.log 2>&1 & disown; tail -f /apps/.logs/events-monitor.log
