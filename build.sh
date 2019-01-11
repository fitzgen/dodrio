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
cargo check --all --target wasm32-unknown-unknown

if test "$INSTALL" != ""; then
    with_dir ./js npm link
fi

for x in ./examples/*; do
    wasm-pack build "$x/crate"

    if test "$INSTALL" != ""; then
        with_dir "$x/crate/pkg" npm link
        with_dir "$x/crate/pkg" npm link dodrio
        with_dir "$x/js" npm audit fix
        with_dir "$x/js" npm install
        with_dir "$x/js" npm link dodrio
    fi

    with_dir "$x/js/" npm run build
done
