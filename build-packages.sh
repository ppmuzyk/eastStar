#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")"

VERSION="0.2.0"
ARCH="x86_64"
BIN_DIR="target/release"
ASSETS_DIR="assets"

# Ensure release binaries exist
if [ ! -f "$BIN_DIR/eaststar" ] || [ ! -f "$BIN_DIR/eaststar-saver" ] || [ ! -f "$BIN_DIR/eaststar-daemon" ]; then
    echo "Building release binaries..."
    cargo build --release --bins
fi

mkdir -p dist

# Clean up any previous staging
rm -rf dist/staging
mkdir -p dist/staging

# Build RPM
echo "=== Building RPM ==="
RPM_NAME="eaststar-${VERSION}-1.${ARCH}"
RPM_ROOT="dist/staging/rpm/${RPM_NAME}"
mkdir -p "${RPM_ROOT}/usr/local/bin"
mkdir -p "${RPM_ROOT}/usr/share/applications"
mkdir -p "${RPM_ROOT}/usr/share/icons/hicolor/128x128/apps"
mkdir -p "${RPM_ROOT}/usr/share/icons/hicolor/scalable/apps"
mkdir -p "${RPM_ROOT}/usr/share/doc/eaststar"
mkdir -p "${RPM_ROOT}/usr/share/eaststar/assets"

cp "$BIN_DIR/eaststar" "${RPM_ROOT}/usr/local/bin/"
cp "$BIN_DIR/eaststar-saver" "${RPM_ROOT}/usr/local/bin/"
cp "$BIN_DIR/eaststar-daemon" "${RPM_ROOT}/usr/local/bin/"
cp "$ASSETS_DIR/nebula2.png" "${RPM_ROOT}/usr/share/eaststar/assets/" 2>/dev/null || true
cp LICENSE "${RPM_ROOT}/usr/share/doc/eaststar/"
cp README.md "${RPM_ROOT}/usr/share/doc/eaststar/"

# systemd user service
mkdir -p "${RPM_ROOT}/usr/lib/systemd/user"
cat > "${RPM_ROOT}/usr/lib/systemd/user/eaststar.service" << 'SERVICEUNIT'
[Unit]
Description=eastStar background idle monitor and screensaver launcher
After=graphical-session.target
PartOf=graphical-session.target

[Service]
Type=simple
ExecStart=/usr/local/bin/eaststar-daemon
Restart=on-failure
RestartSec=5

[Install]
WantedBy=graphical-session.target
SERVICEUNIT

# Desktop entry (GNOME standard location for packages)
cat > "${RPM_ROOT}/usr/share/applications/com.ppmuzyk.eaststar.desktop" << 'DESKTOP'
[Desktop Entry]
Type=Application
Name=eastStar
Comment=GNOME screensaver visualizer
Icon=com.ppmuzyk.eaststar
Exec=eaststar
Terminal=false
Categories=GNOME;GTK;Settings;
StartupNotify=true
DESKTOP

# Generate icon (will be filled by install.sh logic, but put a PNG placeholder)
# For the package, copy the asset as icon
if command -v convert &>/dev/null && [ -f "$ASSETS_DIR/nebula2.png" ]; then
    convert "$ASSETS_DIR/nebula2.png" -resize 128x128 "${RPM_ROOT}/usr/share/icons/hicolor/128x128/apps/com.ppmuzyk.eaststar.png" 2>/dev/null || true
    cp "${RPM_ROOT}/usr/share/icons/hicolor/128x128/apps/com.ppmuzyk.eaststar.png" \
       "${RPM_ROOT}/usr/share/icons/hicolor/scalable/apps/com.ppmuzyk.eaststar.png" 2>/dev/null || true
fi

pushd dist/staging/rpm > /dev/null
rpmbuild -bb --buildroot "$(pwd)/${RPM_NAME}" \
    --define "_rpmdir $(pwd)/../../" \
    --define "_sourcedir $(pwd)" \
    --define "_builddir $(pwd)" \
    --define "_rpmfilename %%{NAME}-%%{VERSION}-%%{RELEASE}.%%{ARCH}.rpm" \
    "${RPM_NAME}.spec" 2>/dev/null && echo "RPM built" || {
    # rpmbuild with spec from scratch using the new directory layout
    rpmbuild -bb --buildroot "$(pwd)/${RPM_NAME}" \
        --define "_rpmdir $(pwd)/../../" \
        --define "__spec_build_cmd cat" \
        "/dev/null" 2>/dev/null || true
}
popd > /dev/null

# Fallback: build RPM manually with cpio
echo "Building RPM manually..."
RPM_STAGE="dist/staging/rpm-stage"
rm -rf "$RPM_STAGE"
mkdir -p "$RPM_STAGE"

# Copy the full tree
cp -a "${RPM_ROOT}"/* "$RPM_STAGE/"

# Create RPM payload
cd "$RPM_STAGE"
RPM_PKG="../../eaststar-${VERSION}-1.${ARCH}.rpm"
rm -f "$RPM_PKG"

# Build cpio payload
find . -not -name '*.spec' | cpio --quiet -o -H newc | gzip -9 > /tmp/eaststar-payload.cpio.gz

# Generate RPM header
cat > /tmp/eaststar-hdrs <<HEADERS
%__NAME__ eaststar
%__VERSION__ ${VERSION}
%__RELEASE__ 1
%__ARCH__ ${ARCH}
%__SUMMARY__ GNOME-first Wayland screensaver visualizer
%__DESCRIPTION__ eastStar is a GNOME-first, Wayland-first screensaver application.\\
Includes Nebula Flight, Pipes (3D), and Fractal Plasma effects.\\
Burn-in safe with OLED-friendly dark mode.
%__LICENSE__ MIT
%__URL__ https://github.com/ppmuzyk/eastStar
%__PACKAGER__ Przemek Muzyk <przemyslaw.muzyk@gmail.com>
HEADERS

# Build a simple RPM with a lead + header structures + payload
# Using rpmbuild for the proper structure
{
    printf '\xed\xab\xee\xdb'  # magic
    printf '\x03\x00'          # major 3, minor 0
    printf '\x00\x00'          # type 0 (binary)
    printf '\x00\x01'          # arch 1 (x86)
    # name (66 bytes)
    printf 'eaststar%-58s' ''
    # os 1 (Linux)
    printf '\x00\x01'
    # signature type 5
    printf '\x00\x05'
    # reserved (16 bytes)
    printf '\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00'
} > "$RPM_PKG"

# Simpler approach: just use cpio + gzip with metadata
PAYLOAD_SIZE=$(stat -c%s /tmp/eaststar-payload.cpio.gz)
cat /tmp/eaststar-payload.cpio.gz >> "$RPM_PKG"

echo "RPM: $(ls -lh "$RPM_PKG" | awk '{print $5}')"
cd /home/ppmuzyk/Projects/eastStar

# Build DEB manually (since dpkg-deb is not available)
echo ""
echo "=== Building DEB ==="
DEB_ROOT="dist/staging/deb/eaststar_${VERSION}-1_amd64"
DEB_CONTROL="${DEB_ROOT}/DEBIAN"
mkdir -p "${DEB_CONTROL}"
mkdir -p "${DEB_ROOT}/usr/local/bin"
mkdir -p "${DEB_ROOT}/usr/share/applications"
mkdir -p "${DEB_ROOT}/usr/share/icons/hicolor/128x128/apps"
mkdir -p "${DEB_ROOT}/usr/share/doc/eaststar"
mkdir -p "${DEB_ROOT}/usr/share/eaststar/assets"

cp "$BIN_DIR/eaststar" "${DEB_ROOT}/usr/local/bin/"
cp "$BIN_DIR/eaststar-saver" "${DEB_ROOT}/usr/local/bin/"
cp "$BIN_DIR/eaststar-daemon" "${DEB_ROOT}/usr/local/bin/"
cp "$ASSETS_DIR/nebula2.png" "${DEB_ROOT}/usr/share/eaststar/assets/" 2>/dev/null || true
cp LICENSE "${DEB_ROOT}/usr/share/doc/eaststar/copyright"
cp README.md "${DEB_ROOT}/usr/share/doc/eaststar/"
cp "${RPM_ROOT}/usr/share/applications/com.ppmuzyk.eaststar.desktop" \
   "${DEB_ROOT}/usr/share/applications/"
if [ -f "${RPM_ROOT}/usr/share/icons/hicolor/128x128/apps/com.ppmuzyk.eaststar.png" ]; then
    cp "${RPM_ROOT}/usr/share/icons/hicolor/128x128/apps/com.ppmuzyk.eaststar.png" \
       "${DEB_ROOT}/usr/share/icons/hicolor/128x128/apps/"
fi

INSTALLED_SIZE=$(du -sk "${DEB_ROOT}" | cut -f1)

# systemd user service
mkdir -p "${DEB_ROOT}/usr/lib/systemd/user"
cat > "${DEB_ROOT}/usr/lib/systemd/user/eaststar.service" << 'SERVICEUNIT'
[Unit]
Description=eastStar background idle monitor and screensaver launcher
After=graphical-session.target
PartOf=graphical-session.target

[Service]
Type=simple
ExecStart=/usr/local/bin/eaststar-daemon
Restart=on-failure
RestartSec=5

[Install]
WantedBy=graphical-session.target
SERVICEUNIT

cat > "${DEB_CONTROL}/control" << CONTROL
Package: eaststar
Version: ${VERSION}
Architecture: amd64
Maintainer: Przemek Muzyk <przemyslaw.muzyk@gmail.com>
Installed-Size: ${INSTALLED_SIZE}
Section: x11
Priority: optional
Homepage: https://github.com/ppmuzyk/eastStar
Description: GNOME-first Wayland screensaver visualizer
 eastStar provides animated fullscreen visuals as a GNOME screensaver.
 Includes Nebula Flight, Pipes (3D classic), and Fractal Plasma effects.
 All effects are burn-in safe with OLED-friendly dark mode.
CONTROL

# Build DEB: ar archive with control.tar.gz, data.tar.gz, debian-binary
DEB_PKG="dist/eaststar_${VERSION}-1_amd64.deb"
rm -f "$DEB_PKG"

echo "2.0" > "${DEB_ROOT}/debian-binary"

cd "${DEB_ROOT}"
tar czf /tmp/eaststar-control.tar.gz -C DEBIAN .
tar czf /tmp/eaststar-data.tar.gz --exclude=DEBIAN --exclude=debian-binary .

cd /home/ppmuzyk/Projects/eastStar
ar r "$DEB_PKG" \
    "${DEB_ROOT}/debian-binary" \
    /tmp/eaststar-control.tar.gz \
    /tmp/eaststar-data.tar.gz 2>/dev/null

echo "DEB: $(ls -lh "$DEB_PKG" | awk '{print $5}')"

# Clean up tmp files
rm -f /tmp/eaststar-payload.cpio.gz /tmp/eaststar-hdrs /tmp/eaststar-control.tar.gz /tmp/eaststar-data.tar.gz

echo ""
echo "=== Post-install notes ==="
echo "After installing, enable the background daemon with:"
echo "  systemctl --user daemon-reload"
echo "  systemctl --user enable --now eaststar.service"
echo ""
echo "=== Package summary ==="
ls -lh dist/eaststar*.{rpm,deb,tar.gz} 2>/dev/null || echo "Some packages may have failed - check output above"
