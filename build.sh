#!/usr/bin/env bash

cd $(dirname $0)
set -eux

cargo fmt --all || true
cargo build --all --target wasm32-unknown-unknown

for x in ./examples/*; do
    wasm-pack build --target web "$x"
done
