#!/usr/bin/env bash
# Build the hello handler to wasm32-wasip1.
#
# Usage: ./build.sh
# Output: ../../target/wasm32-wasip1/release/hello.wasm
# Also copies the artifact next to extension.json as hello.wasm so the
# directory is a ready-to-mount HIP-0105 extension.

set -euo pipefail

cd "$(dirname "$0")"

cargo build --release --target wasm32-wasip1 -p zip-rs-example-hello

WASM_SRC="../../target/wasm32-wasip1/release/hello.wasm"
cp "$WASM_SRC" ./hello.wasm

# Try to shrink with wasm-opt if available. Optional; raw build is
# already production-grade.
if command -v wasm-opt >/dev/null 2>&1; then
  wasm-opt -Oz -o ./hello.wasm ./hello.wasm
fi

ls -lh ./hello.wasm
