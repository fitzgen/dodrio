#!/usr/bin/env bash

cd $(dirname $0)
set -eux

function with_dir {
    pushd $1
    shift
    $@
    popd
}

cargo fmt --all || true
cargo check --all --target wasm32-unknown-unknown

for x in ./examples/*; do
    wasm-pack build --target no-modules "$x"
done
