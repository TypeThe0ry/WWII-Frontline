#!/usr/bin/env bash
# Local M0-029 run (see DEVELOPING.md): native x2, wasm x2, compare. Pass --strict to fail on native/wasm difference.
set -euo pipefail
cd "$(dirname "$0")/../.."
cargo run --release -p frontline-sim-wasm --example determinism
node tools/determinism/run-wasm.mjs
node tools/determinism/compare.mjs "$@"
