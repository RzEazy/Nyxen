#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")"
cargo build --release
OUT="${1:-./nyxen}"
cp -f target/release/nyxen "$OUT"
chmod +x "$OUT"
echo "Built: $OUT ($(du -h "$OUT" | cut -f1))"
