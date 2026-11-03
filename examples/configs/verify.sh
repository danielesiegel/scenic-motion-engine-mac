#!/usr/bin/env bash
# Round-trip verify every Config example in this directory.
#
# Step 1: ensure the project builds (we invoke `render --help` so the main
#         binary links and runs without needing a track).
# Step 2: parse each .json via python's json loader to prove it is valid JSON.
#
# This script is intentionally non-destructive and does not depend on serde
# at runtime — the Rust-side `Config` round-trip is already exercised by
# `cargo test config::persistence::tests::round_trip_default_config`.

set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "$here/../.." && pwd)"

echo "[1/2] cargo build check via render --help ..."
(cd "$repo_root" && cargo run --release --bin render -- --help >/dev/null)

echo "[2/2] JSON parse check for each config ..."
fail=0
for f in "$here"/*.json; do
  name="$(basename "$f")"
  if python3 -c "import json,sys; json.load(open(sys.argv[1]))" "$f"; then
    echo "  OK   $name"
  else
    echo "  FAIL $name"
    fail=1
  fi
done

if [ "$fail" -ne 0 ]; then
  echo "verify: one or more configs failed to parse" >&2
  exit 1
fi

echo "verify: all configs parsed successfully"
