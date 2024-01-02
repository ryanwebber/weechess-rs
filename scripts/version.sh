#!/usr/bin/env bash

set -e

cargo build --release
cp target/release/weechess "versions/$(target/release/weechess version).exe"
