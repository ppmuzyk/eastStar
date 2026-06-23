#!/usr/bin/env bash
set -euo pipefail

APP_ID="com.ppmuzyk.eaststar"
APP_NAME="eaststar"
BUILD_MODE="release"
PREFIX="${HOME}/.local"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --debug)
      BUILD_MODE="debug"
      shift
      ;;
    --prefix)
      PREFIX="$2"
      shift 2
      ;;
    *)
      echo "Unknown argument: $1" >&2
      echo "Usage: ./install.sh [--debug] [--prefix PATH]" >&2
      exit 1
      ;;
  esac
done

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BUILD_ARGS=()
TARGET_DIR="${SCRIPT_DIR}/target/release"

if [[ "${BUILD_MODE}" == "release" ]]; then
  BUILD_ARGS+=(--release)
else
  TARGET_DIR="${SCRIPT_DIR}/target/debug"
fi

BIN_DIR="${PREFIX}/bin"
APP_DIR="${PREFIX}/share/applications"
ICON_DIR="${PREFIX}/share/icons/hicolor"

echo "Building ${APP_NAME} (${BUILD_MODE})..."
cargo build --bins "${BUILD_ARGS[@]}"

echo "Preparing icon set..."
cargo build --bin "${APP_NAME}" "${BUILD_ARGS[@]}"

echo "Installing binaries into ${BIN_DIR}..."
mkdir -p "${BIN_DIR}"
install -m 0755 "${TARGET_DIR}/${APP_NAME}" "${BIN_DIR}/${APP_NAME}"
install -m 0755 "${TARGET_DIR}/${APP_NAME}-saver" "${BIN_DIR}/${APP_NAME}-saver"

echo "Installing desktop entry..."
mkdir -p "${APP_DIR}"
sed "s|^Exec=.*|Exec=${BIN_DIR}/${APP_NAME}|" \
  "${SCRIPT_DIR}/data/${APP_ID}.desktop" > "${APP_DIR}/${APP_ID}.desktop"

echo "Installing icons..."
for size_dir in "${SCRIPT_DIR}"/assets/generated-icons/hicolor/*; do
  [[ -d "${size_dir}" ]] || continue
  size_name="$(basename "${size_dir}")"
  mkdir -p "${ICON_DIR}/${size_name}/apps"
  install -m 0644 \
    "${size_dir}/apps/${APP_ID}.png" \
    "${ICON_DIR}/${size_name}/apps/${APP_ID}.png"
done

if command -v update-desktop-database >/dev/null 2>&1; then
  update-desktop-database "${APP_DIR}" || true
fi

if command -v gtk-update-icon-cache >/dev/null 2>&1; then
  rm -f "${PREFIX}/share/icons/hicolor/icon-theme.cache"
  gtk-update-icon-cache -f -t "${PREFIX}/share/icons/hicolor" >/dev/null 2>&1 || true
fi

echo
echo "Installed ${APP_NAME} to ${PREFIX}"
echo "Launch with: gtk-launch ${APP_ID}"
