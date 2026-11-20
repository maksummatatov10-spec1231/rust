#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")"
cargo build --release --target wasm32-unknown-unknown
cp target/wasm32-unknown-unknown/release/forest_bloom_3d_core.wasm core.wasm
python3 make_single.py
