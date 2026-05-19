#!/usr/bin/env bash
# Build the policy-eval handler to wasm32-wasip1.

set -euo pipefail

cd "$(dirname "$0")"

cargo build --release --target wasm32-wasip1 -p zip-rs-example-policy-eval

cp ../../target/wasm32-wasip1/release/policy_eval.wasm ./policy_eval.wasm

if command -v wasm-opt >/dev/null 2>&1; then
  wasm-opt -Oz -o ./policy_eval.wasm ./policy_eval.wasm
fi

ls -lh ./policy_eval.wasm
