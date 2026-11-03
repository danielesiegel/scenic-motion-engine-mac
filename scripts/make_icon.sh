#!/usr/bin/env bash
# chmod +x scripts/make_icon.sh
#
# Build assets/icon/AppIcon.icns from assets/icon/AppIcon.svg.
# Uses rsvg-convert if available, else sips on a pre-rasterized master PNG.
# Gracefully warns and exits non-zero if required tools are missing.
set -euo pipefail

cd "$(dirname "$0")/.."

SVG="assets/icon/AppIcon.svg"
OUT_ICNS="assets/icon/AppIcon.icns"
ICONSET_DIR="assets/icon/AppIcon.iconset"
MASTER_PNG="assets/icon/AppIcon-1024.png"

if [[ ! -f "${SVG}" ]]; then
  echo "ERROR: ${SVG} not found." >&2
  exit 1
fi

if ! command -v iconutil >/dev/null 2>&1; then
  echo "ERROR: 'iconutil' not found on PATH. Install Xcode command line tools:" >&2
  echo "  xcode-select --install" >&2
  exit 2
fi

HAVE_RSVG=0
HAVE_SIPS=0
command -v rsvg-convert >/dev/null 2>&1 && HAVE_RSVG=1
command -v sips          >/dev/null 2>&1 && HAVE_SIPS=1

if [[ ${HAVE_RSVG} -eq 0 && ${HAVE_SIPS} -eq 0 ]]; then
  echo "ERROR: need either 'rsvg-convert' (brew install librsvg) or 'sips' to rasterize." >&2
  exit 3
fi

# Build master 1024 PNG. rsvg-convert handles SVG directly; sips cannot read SVG,
# so without rsvg we can't produce a master from this SVG and must warn.
if [[ ${HAVE_RSVG} -eq 1 ]]; then
  echo "==> Rasterizing master PNG via rsvg-convert"
  rsvg-convert -w 1024 -h 1024 "${SVG}" -o "${MASTER_PNG}"
else
  echo "WARN: rsvg-convert missing; cannot rasterize SVG with sips alone." >&2
  echo "      Install librsvg (brew install librsvg) or supply ${MASTER_PNG} manually." >&2
  if [[ ! -f "${MASTER_PNG}" ]]; then
    exit 4
  fi
  echo "==> Using existing ${MASTER_PNG}"
fi

echo "==> Building iconset directory"
rm -rf "${ICONSET_DIR}"
mkdir -p "${ICONSET_DIR}"

# iconset entries required by iconutil.
# name                        size
# icon_16x16.png              16
# icon_16x16@2x.png           32
# icon_32x32.png              32
# icon_32x32@2x.png           64
# icon_128x128.png            128
# icon_128x128@2x.png         256
# icon_256x256.png            256
# icon_256x256@2x.png         512
# icon_512x512.png            512
# icon_512x512@2x.png         1024

render() {
  local size="$1"
  local out="$2"
  if [[ ${HAVE_RSVG} -eq 1 ]]; then
    rsvg-convert -w "${size}" -h "${size}" "${SVG}" -o "${out}"
  else
    sips -z "${size}" "${size}" "${MASTER_PNG}" --out "${out}" >/dev/null
  fi
}

render 16   "${ICONSET_DIR}/icon_16x16.png"
render 32   "${ICONSET_DIR}/icon_16x16@2x.png"
render 32   "${ICONSET_DIR}/icon_32x32.png"
render 64   "${ICONSET_DIR}/icon_32x32@2x.png"
render 128  "${ICONSET_DIR}/icon_128x128.png"
render 256  "${ICONSET_DIR}/icon_128x128@2x.png"
render 256  "${ICONSET_DIR}/icon_256x256.png"
render 512  "${ICONSET_DIR}/icon_256x256@2x.png"
render 512  "${ICONSET_DIR}/icon_512x512.png"
render 1024 "${ICONSET_DIR}/icon_512x512@2x.png"

echo "==> Running iconutil"
iconutil -c icns "${ICONSET_DIR}" -o "${OUT_ICNS}"

echo "==> Cleanup"
rm -rf "${ICONSET_DIR}"
rm -f "${MASTER_PNG}"

ICNS_SIZE="$(du -h "${OUT_ICNS}" | awk '{print $1}')"
echo ""
echo "SUCCESS: ${OUT_ICNS} (${ICNS_SIZE})"
