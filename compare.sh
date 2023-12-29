#!/usr/bin/env bash

set -e

SCRIPT_DIR=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" &> /dev/null && pwd)

ENGINE_CTRL_NAME="weechess_control"
ENGINE_CTRL_PATH="${SCRIPT_DIR}/target/release/weechess"
ENGINE_CTRL_ARGS="uci"

ENGINE_TEST_NAME="weechess_test"
ENGINE_TEST_PATH="${SCRIPT_DIR}/target/release/weechess"
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
    -concurrency 1 \
    -ratinginterval 2 \
    -games 1000 \
    -pgnout /tmp/result.pgn \
    -repeat \
    -each tc=300+1 \
    -recover \
    -sprt elo0=0 elo1=10 alpha=0.05 beta=0.05
