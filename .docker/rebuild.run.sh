#!/bin/bash
#/usr/bin/env bash

shopt -s extglob
shopt -s extquote
# shopt -s xpg_echo

set -f

# Remove volumes to trigger init
docker compose down -v ;
docker compose up --build ;

