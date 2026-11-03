#!/usr/bin/env bash
# Thin wrapper around batch_analyze.py so callers can do
#   scripts/batch_analyze.sh "assets/audio/*.wav" [--force --fps 60 ...]
# Depends on scripts/analyze.py existing alongside this file.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PY="${PYTHON:-python3}"

if [[ ! -f "${SCRIPT_DIR}/analyze.py" ]]; then
  echo "error: ${SCRIPT_DIR}/analyze.py not found" >&2
  exit 2
fi

if [[ ! -f "${SCRIPT_DIR}/batch_analyze.py" ]]; then
  echo "error: ${SCRIPT_DIR}/batch_analyze.py not found" >&2
  exit 2
fi

if [[ $# -lt 1 ]]; then
  cat >&2 <<EOF
Usage: $(basename "$0") "<glob>" [--force] [--fps N] [--stems] [--analysis-dir DIR] [-- <extra analyze.py args>]
Example:
  $(basename "$0") "assets/audio/*.wav" --fps 60
  $(basename "$0") "assets/audio/*.wav" --force
EOF
  exit 2
fi

exec "${PY}" "${SCRIPT_DIR}/batch_analyze.py" "$@"
