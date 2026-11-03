#!/usr/bin/env bash
# analyze_preset.sh - run scripts/analyze.py with a genre preset from
# scripts/analyze_presets.yaml.
#
# Usage:
#   ./scripts/analyze_preset.sh <preset> <wav> [extra analyze.py args...]
#
# Example:
#   ./scripts/analyze_preset.sh techno-128bpm assets/audio/foo.wav
#   ./scripts/analyze_preset.sh dnb-174bpm foo.wav --fps 60 --stems
#
# Uses `yq` if available; otherwise falls back to an awk-based parser.

set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd -- "${SCRIPT_DIR}/.." && pwd)"
YAML="${SCRIPT_DIR}/analyze_presets.yaml"
ANALYZE_PY="${SCRIPT_DIR}/analyze.py"

usage() {
  echo "usage: $0 <preset> <wav> [extra analyze.py args...]" >&2
  echo "" >&2
  echo "available presets:" >&2
  list_presets | sed 's/^/  - /' >&2
}

list_presets() {
  if command -v yq >/dev/null 2>&1; then
    yq -r '.presets | keys | .[]' "$YAML"
  else
    awk '
      /^presets:[[:space:]]*$/ { in_presets=1; next }
      in_presets && /^[^[:space:]#]/ { in_presets=0 }
      in_presets && /^[[:space:]]{2}[a-zA-Z0-9_-]+:[[:space:]]*$/ {
        name=$1; sub(/:$/, "", name); print name
      }
    ' "$YAML"
  fi
}

# Emit `--flag value` tokens, one per line, for the given preset.
extract_flags() {
  local preset="$1"
  if command -v yq >/dev/null 2>&1; then
    yq -r --arg p "$preset" '
      .presets[$p].flags
      | to_entries
      | .[]
      | "--\(.key)\n\(.value)"
    ' "$YAML" 2>/dev/null || true
  else
    awk -v target="$preset" '
      # Track block structure by indentation.
      /^presets:[[:space:]]*$/ { in_presets=1; next }
      in_presets && /^[^[:space:]#]/ { in_presets=0 }

      # Match a preset name at exactly 2-space indent.
      in_presets && /^[[:space:]]{2}[a-zA-Z0-9_-]+:[[:space:]]*$/ {
        name=$1; sub(/:$/, "", name)
        in_preset=(name == target) ? 1 : 0
        in_flags=0
        next
      }

      # Enter flags block at 4-space indent.
      in_preset && /^[[:space:]]{4}flags:[[:space:]]*$/ {
        in_flags=1
        next
      }

      # Exit flags block when indent drops below 6 and line is not blank/comment.
      in_flags && /^[[:space:]]{0,5}[^[:space:]#]/ {
        in_flags=0
      }

      # Flag line: 6-space indent, "  key: value".
      in_flags && /^[[:space:]]{6}[a-zA-Z0-9_-]+:[[:space:]]*.+$/ {
        line=$0
        sub(/^[[:space:]]+/, "", line)
        # split on first colon
        key=line
        val=line
        sub(/:.*$/, "", key)
        sub(/^[^:]+:[[:space:]]*/, "", val)
        # strip trailing whitespace and inline comments
        sub(/[[:space:]]*#.*$/, "", val)
        sub(/[[:space:]]+$/, "", val)
        # strip surrounding quotes
        gsub(/^"|"$/, "", val)
        gsub(/^'\''|'\''$/, "", val)
        printf("--%s\n%s\n", key, val)
      }
    ' "$YAML"
  fi
}

if [[ $# -lt 2 ]]; then
  usage
  exit 2
fi

PRESET="$1"
WAV="$2"
shift 2
EXTRA_ARGS=("$@")

if [[ ! -f "$YAML" ]]; then
  echo "error: preset YAML not found: $YAML" >&2
  exit 1
fi

if [[ ! -f "$ANALYZE_PY" ]]; then
  echo "error: analyze.py not found: $ANALYZE_PY" >&2
  exit 1
fi

# Validate preset exists.
if ! list_presets | grep -qx "$PRESET"; then
  echo "error: unknown preset '$PRESET'" >&2
  usage
  exit 2
fi

if [[ ! -f "$WAV" ]]; then
  echo "warning: wav not found at '$WAV' (passing through to analyze.py)" >&2
fi

# Build flag array from preset.
FLAGS=()
while IFS= read -r line; do
  [[ -z "$line" ]] && continue
  FLAGS+=("$line")
done < <(extract_flags "$PRESET")

if command -v yq >/dev/null 2>&1; then
  PARSER="yq"
else
  PARSER="awk"
fi

echo "[analyze_preset] preset=$PRESET parser=$PARSER flags=${#FLAGS[@]}" >&2
echo "[analyze_preset] exec: python3 $ANALYZE_PY --input $WAV ${FLAGS[*]} ${EXTRA_ARGS[*]}" >&2

exec python3 "$ANALYZE_PY" --input "$WAV" "${FLAGS[@]}" "${EXTRA_ARGS[@]}"
