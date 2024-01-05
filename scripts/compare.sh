#!/usr/bin/env bash

set -e

which cutechess-cli > /dev/null || (echo "cutechess-cli not found" && exit 1)

SCRIPT_DIR=$(dirname $(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" &> /dev/null && pwd))

ENGINE_CTRL_PATH="${SCRIPT_DIR}/versions/weechess.005-jackal.exe"
ENGINE_CTRL_NAME="$(${ENGINE_CTRL_PATH} version)"
ENGINE_CTRL_ARGS="uci"

ENGINE_TEST_PATH="${SCRIPT_DIR}/target/release/weechess"
ENGINE_TEST_NAME="$(${ENGINE_TEST_PATH} version)"
ENGINE_TEST_ARGS="uci"

ls $ENGINE_CTRL_PATH > /dev/null
ls $ENGINE_TEST_PATH > /dev/null

set -x

# Missing args to consder:
#  -openings file=C:\c\performance.bin
cutechess-cli \
    -engine "name=${ENGINE_CTRL_NAME}" proto=uci "cmd=${ENGINE_CTRL_PATH}" "arg=${ENGINE_CTRL_ARGS}" \
    -engine "name=${ENGINE_TEST_NAME}" proto=uci "cmd=${ENGINE_TEST_PATH}" "arg=${ENGINE_TEST_ARGS}" \
    -debug \
    -concurrency 16 \
    -ratinginterval 2 \
    -games 100 \
    -pgnout /tmp/result.pgn \
    -repeat \
    -each tc=300+1 \
    -recover \
    -sprt elo0=0 elo1=10 alpha=0.05 beta=0.05
