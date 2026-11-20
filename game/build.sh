#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")"
cargo build --release --target wasm32-unknown-unknown
cp target/wasm32-unknown-unknown/release/game_core.wasm core.wasm
echo "built core.wasm ($(du -h core.wasm | cut -f1))"
