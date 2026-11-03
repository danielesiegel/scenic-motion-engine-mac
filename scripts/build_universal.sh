#!/usr/bin/env bash
# run: chmod +x scripts/build_universal.sh
#
# Build a macOS universal (aarch64 + x86_64) release binary for
# scenic-motion-engine using cargo + lipo. No code signing performed.
set -euo pipefail

BIN_NAME="scenic-motion-engine"
UNIVERSAL_OUT="target/release/${BIN_NAME}-universal"
ARM_TARGET="aarch64-apple-darwin"
X86_TARGET="x86_64-apple-darwin"

cd "$(dirname "$0")/.."

echo "==> Ensuring rustup targets installed"
for t in "${ARM_TARGET}" "${X86_TARGET}"; do
  if ! rustup target list --installed | grep -qx "${t}"; then
    echo "    adding ${t}"
    rustup target add "${t}"
  else
    echo "    ${t} already installed"
  fi
done

echo "==> Building release for ${ARM_TARGET}"
cargo build --release --target "${ARM_TARGET}"

echo "==> Building release for ${X86_TARGET}"
cargo build --release --target "${X86_TARGET}"

ARM_BIN="target/${ARM_TARGET}/release/${BIN_NAME}"
X86_BIN="target/${X86_TARGET}/release/${BIN_NAME}"

if [[ ! -f "${ARM_BIN}" ]]; then
  echo "ERROR: missing ${ARM_BIN}" >&2
  exit 1
fi
if [[ ! -f "${X86_BIN}" ]]; then
  echo "ERROR: missing ${X86_BIN}" >&2
  exit 1
fi

mkdir -p target/release

echo "==> Fusing with lipo -> ${UNIVERSAL_OUT}"
lipo -create -output "${UNIVERSAL_OUT}" "${ARM_BIN}" "${X86_BIN}"

echo "==> Verifying architectures"
lipo -info "${UNIVERSAL_OUT}"

SIZE_BYTES=$(stat -f%z "${UNIVERSAL_OUT}")
SIZE_HUMAN=$(du -h "${UNIVERSAL_OUT}" | awk '{print $1}')
ABS_PATH="$(cd "$(dirname "${UNIVERSAL_OUT}")" && pwd)/$(basename "${UNIVERSAL_OUT}")"

echo ""
echo "Universal binary: ${ABS_PATH}"
echo "Size:             ${SIZE_HUMAN} (${SIZE_BYTES} bytes)"
