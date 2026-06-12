#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/../.."
. "$HOME/.cargo/env"

run_id="${RUN_ID:-kpg-$(date +%F)}"
out_dir="research/runs/${run_id}"
mkdir -p "$out_dir"

/usr/bin/time -v cargo run --manifest-path solver/Cargo.toml --release --bin solve KPG \
  --out-dir "$out_dir" \
  --dump-table \
  --audit-large 2>&1 | tee "$out_dir/console.log"
