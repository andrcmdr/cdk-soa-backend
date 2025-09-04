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
../target/debug/cdk-indexer ./config.yaml ./init.sql
# ../target/release/cdk-indexer ./config.yaml ./init.sql

# or via Docker Compose
docker compose up --build
