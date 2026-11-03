#!/usr/bin/env bash
# run: chmod +x scripts/bundle_app.sh
#
# Package the universal binary produced by scripts/build_universal.sh
# into "scenic motion engine.app" and a UDZO DMG. No code signing.
set -euo pipefail

BIN_NAME="scenic-motion-engine"
APP_DISPLAY="scenic motion engine"
UNIVERSAL_BIN="target/release/${BIN_NAME}-universal"
APP_DIR="target/release/${APP_DISPLAY}.app"
DMG_OUT="target/release/scenic-motion-engine.dmg"
TEMPLATE="scripts/Info.plist.template"

cd "$(dirname "$0")/.."

if [[ ! -f "${UNIVERSAL_BIN}" ]]; then
  echo "ERROR: ${UNIVERSAL_BIN} not found. Run scripts/build_universal.sh first." >&2
  exit 1
fi
if [[ ! -f "${TEMPLATE}" ]]; then
  echo "ERROR: missing ${TEMPLATE}" >&2
  exit 1
fi

VERSION="$(grep -m1 '^version' Cargo.toml | sed -E 's/.*"([^"]+)".*/\1/' || echo "0.0.0")"
echo "==> Version: ${VERSION}"

echo "==> Clearing previous bundle"
rm -rf "${APP_DIR}"
mkdir -p "${APP_DIR}/Contents/MacOS"
mkdir -p "${APP_DIR}/Contents/Resources"

echo "==> Copying universal binary"
cp "${UNIVERSAL_BIN}" "${APP_DIR}/Contents/MacOS/${BIN_NAME}"
chmod +x "${APP_DIR}/Contents/MacOS/${BIN_NAME}"

echo "==> Writing Info.plist"
sed "s/__VERSION__/${VERSION}/g" "${TEMPLATE}" > "${APP_DIR}/Contents/Info.plist"

echo "==> Bundle layout:"
find "${APP_DIR}" -maxdepth 3 -print

echo "==> Removing previous DMG (if any)"
rm -f "${DMG_OUT}"

echo "==> Creating DMG"
hdiutil create \
  -volname "${APP_DISPLAY}" \
  -srcfolder "${APP_DIR}" \
  -ov \
  -format UDZO \
  "${DMG_OUT}"

APP_ABS="$(cd "$(dirname "${APP_DIR}")" && pwd)/$(basename "${APP_DIR}")"
DMG_ABS="$(cd "$(dirname "${DMG_OUT}")" && pwd)/$(basename "${DMG_OUT}")"
DMG_SIZE="$(du -h "${DMG_OUT}" | awk '{print $1}')"

echo ""
echo "App bundle: ${APP_ABS}"
echo "DMG:        ${DMG_ABS} (${DMG_SIZE})"
echo ""
echo "Note: no code signing / notarization performed. Sign + notarize"
echo "separately before public distribution."
