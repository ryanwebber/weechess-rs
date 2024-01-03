#!/usr/bin/env bash

set -e

mkdir -p versions

cargo build --release
cp target/release/weechess "versions/$(target/release/weechess version).exe"
