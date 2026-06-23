#!/usr/bin/env bash
set -euo pipefail

APP_ID="com.ppmuzyk.eaststar"
APP_NAME="eaststar"
PREFIX="${HOME}/.local"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --prefix)
      PREFIX="$2"
      shift 2
      ;;
    *)
      echo "Unknown argument: $1" >&2
      echo "Usage: ./uninstall.sh [--prefix PATH]" >&2
      exit 1
      ;;
  esac
done

BIN_DIR="${PREFIX}/bin"
APP_DIR="${PREFIX}/share/applications"
ICON_DIR="${PREFIX}/share/icons/hicolor"

echo "Removing binaries..."
rm -f "${BIN_DIR}/${APP_NAME}" "${BIN_DIR}/${APP_NAME}-saver"

echo "Removing desktop entry..."
rm -f "${APP_DIR}/${APP_ID}.desktop"

echo "Removing icons..."
for size_dir in "${ICON_DIR}"/*; do
  [[ -d "${size_dir}/apps" ]] || continue
  rm -f "${size_dir}/apps/${APP_ID}.png"
done

if command -v update-desktop-database >/dev/null 2>&1; then
  update-desktop-database "${APP_DIR}" || true
fi

if command -v gtk-update-icon-cache >/dev/null 2>&1; then
  gtk-update-icon-cache "${PREFIX}/share/icons/hicolor" >/dev/null 2>&1 || true
fi

echo
echo "Removed ${APP_NAME} from ${PREFIX}"
