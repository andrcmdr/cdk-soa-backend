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

# via Docker
docker pull postgres:17
docker pull nats:2-scratch
pwgen -1cnys 20 1 | tr -d "\n" > ./.secret/pg-passwd
docker stop postgres_17 ; docker rm postgres_17
docker run --name postgres_17 -e POSTGRES_PASSWORD_FILE=/run/secrets/pg-passwd -e PGDATA=/var/lib/postgresql/data/pgdata -v ./.secret/pg-passwd:/run/secrets/pg-passwd -v ./.pgdata/:/var/lib/postgresql/data/pgdata/ -v ./init.sql:/docker-entrypoint-initdb.d/init.sql -p 5432:5432 -d postgres:17
docker exec -ti postgres_17 bash
