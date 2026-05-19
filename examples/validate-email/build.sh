#!/usr/bin/env bash
# Build the validate-email handler to wasm32-wasip1.
#
# Usage: ./build.sh
# Output: ./validate.wasm (next to extension.json so this directory is
# a drop-in HIP-0105 extension).

set -euo pipefail

cd "$(dirname "$0")"

cargo build --release --target wasm32-wasip1 -p zip-rs-example-validate-email

cp ../../target/wasm32-wasip1/release/validate.wasm ./validate.wasm

if command -v wasm-opt >/dev/null 2>&1; then
  wasm-opt -Oz -o ./validate.wasm ./validate.wasm
fi

ls -lh ./validate.wasm
