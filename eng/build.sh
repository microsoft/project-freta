#!/usr/bin/bash

set -exu

cargo fmt -- --check
cargo clippy --release --all-targets --all-features --locked -- -D warnings -D clippy::pedantic
cargo test --release --locked
cargo build --release --locked
cargo build --examples --all-features --release --locked
