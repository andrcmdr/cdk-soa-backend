#!/bin/bash
#/usr/bin/env bash

shopt -s extglob
shopt -s extquote
# shopt -s xpg_echo

set -f

# 1) Set and adjust actual parameters/values in config.yaml file
# 2) Put ABI JSONs under ./abi/
# 3) Run locally:
# cargo run --debug -- ./config.yaml ./init.sql
# cargo run --release -- ./config.yaml ./init.sql
# ../target/debug/events-monitor ./config.yaml ./init.sql
# ../target/release/events-monitor ./config.yaml ./init.sql

# or via Docker Compose
docker compose up --build

# ABI and contracts fetcher

./abi-fetcher ./abi_fetcher.config.yaml
./contracts-fetcher ./contracts_fetcher.config.yaml

# via Docker

docker pull postgres:17
docker rmi postgres:17
docker pull nats:2-scratch
docker rmi nats:2-scratch
DOCKER_BUILDKIT=1 docker build --no-cache -f ./cdk-soa-backend.dockerfile -t "cdk-soa-backend" ./
docker rmi cdk-soa-backend

mkdir -vp ./.secret/; pwgen -1cnys 20 1 | tr -d "\n" > ./.secret/pg-passwd
docker stop postgres_17 ; docker rm postgres_17
docker run --name postgres_17 -e POSTGRES_PASSWORD_FILE=/run/secrets/pg-passwd -e PGDATA=/var/lib/postgresql/data/pgdata -v ./.secret/pg-passwd:/run/secrets/pg-passwd:ro -v ./.pgdata/:/var/lib/postgresql/data/pgdata/ -v ./init.sql:/docker-entrypoint-initdb.d/init.sql:ro --network host -p 127.0.0.1:5432:5432 -d postgres:17
docker exec -ti postgres_17 bash
docker logs postgres_17

docker stop nats_2; docker rm nats_2
docker run --name nats_2 -v ./.natsdata/:/apps/nats.db/ -v ./.logs/:/apps/.logs/ -v ./.config/nats.config:/apps/.config/nats.config:ro --network host -p 127.0.0.1:4222:4222 -p 127.0.0.1:4242:4242 -p 127.0.0.1:8222:8222 -p 127.0.0.1:6222:6222 -d cdk-soa-backend ./nats-server --name 'events_monitor_bus_nats_server' --addr 127.0.0.1 --port 4222 --http_port 4242 --config ./.config/nats.config --log_size_limit 1073741824 --jetstream
docker exec -ti nats_2 bash
docker logs nats_2

docker stop abi-fetcher; docker rm abi-fetcher
docker run --name abi-fetcher -v ./.events_data/contracts_abi/:/apps/contracts_abi/ -v ./.config/abi_fetcher.config.yaml:/apps/.config/abi_fetcher.config.yaml:ro cdk-soa-backend ./abi-fetcher ./.config/abi_fetcher.config.yaml
docker cp abi-fetcher:/apps/contracts_abi/ ./.events_data/
docker start abi-fetcher && docker logs abi-fetcher
docker logs abi-fetcher

docker stop contracts-fetcher; docker rm contracts-fetcher
docker run --name contracts-fetcher -v ./.events_data/contracts/:/apps/contracts/ -v ./.config/contracts_fetcher.config.yaml:/apps/.config/contracts_fetcher.config.yaml:ro cdk-soa-backend ./contracts-fetcher ./.config/contracts_fetcher.config.yaml
docker cp contracts-fetcher:/apps/contracts/ ./.events_data/
docker start contracts-fetcher && docker logs contracts-fetcher
docker logs contracts-fetcher

docker stop events_monitor; docker rm events_monitor
docker run --name events_monitor -v ./events_monitor.config.yaml:/apps/.config/events_monitor.config.yaml:ro -v ./init_table.sql:/apps/.config/init_table.sql:ro -v ./abi/:/apps/abi/:ro --network host cdk-soa-backend ./events-monitor ./.config/events_monitor.config.yaml ./.config/init_table.sql 2>&1 | tee -a ./.logs/events_monitor.log & disown ;
docker logs events_monitor

