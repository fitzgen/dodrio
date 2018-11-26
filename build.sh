#!/usr/bin/env bash

cd $(dirname $0)
set -eux

INSTALL=${INSTALL:-""}

function with_dir {
    pushd $1
    shift
    $@
    popd
}

cargo fmt --all
cargo check
wasm-pack build ./example/rust

if test "$INSTALL" != ""; then
    with_dir ./js npm link
    with_dir ./example/rust/pkg npm link
    with_dir ./example/rust/pkg npm link dodrio
    with_dir ./example/js npm install
    with_dir ./example/js npm link dodrio
    with_dir ./example/js npm link dodrio-example
fi

with_dir ./example/js/ npm run build
