#!/usr/bin/env bash

set -ex

cd "$(dirname "$0")"

cd crates/js-api
cargo publish --no-verify $PUBLISH_FLAGS
cd ../..

cargo publish $PUBLISH_FLAGS
