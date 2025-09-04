#!/bin/bash
#/usr/bin/env bash

shopt -s extglob
shopt -s extquote
# shopt -s xpg_echo

set -f

# 1) Put ABIs under ./abi and update config.yaml addresses/paths
# 2) Start services
docker compose up --build
# 3) Tail logs
docker compose logs -f indexer
