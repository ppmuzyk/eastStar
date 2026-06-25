#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")"

VERSION="0.3.0"
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

# === Build RPM with rpmbuild ===
echo "=== Building RPM ==="
RPM_NAME="eaststar-${VERSION}-1.${ARCH}"
RPM_BUILD_ROOT="$(pwd)/dist/staging/rpm-build"
RPM_INSTALL_ROOT="${RPM_BUILD_ROOT}"

rm -rf "${RPM_BUILD_ROOT}"
mkdir -p "${RPM_INSTALL_ROOT}/usr/local/bin"
mkdir -p "${RPM_INSTALL_ROOT}/usr/share/applications"
mkdir -p "${RPM_INSTALL_ROOT}/usr/share/icons/hicolor/128x128/apps"
mkdir -p "${RPM_INSTALL_ROOT}/usr/share/icons/hicolor/scalable/apps"
mkdir -p "${RPM_INSTALL_ROOT}/usr/share/doc/eaststar"
mkdir -p "${RPM_INSTALL_ROOT}/usr/share/eaststar/assets"
mkdir -p "${RPM_INSTALL_ROOT}/usr/lib/systemd/user"

cp "$BIN_DIR/eaststar" "${RPM_INSTALL_ROOT}/usr/local/bin/"
cp "$BIN_DIR/eaststar-saver" "${RPM_INSTALL_ROOT}/usr/local/bin/"
cp "$BIN_DIR/eaststar-daemon" "${RPM_INSTALL_ROOT}/usr/local/bin/"
cp "$ASSETS_DIR/nebula2.png" "${RPM_INSTALL_ROOT}/usr/share/eaststar/assets/" 2>/dev/null || true
cp LICENSE "${RPM_INSTALL_ROOT}/usr/share/doc/eaststar/"
cp README.md "${RPM_INSTALL_ROOT}/usr/share/doc/eaststar/"

# Desktop entry
cat > "${RPM_INSTALL_ROOT}/usr/share/applications/com.ppmuzyk.eaststar.desktop" << 'DESKTOP'
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

# systemd user service
cat > "${RPM_INSTALL_ROOT}/usr/lib/systemd/user/eaststar.service" << 'SERVICEUNIT'
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

# systemd user preset — enables the service by default for all users
mkdir -p "${RPM_INSTALL_ROOT}/usr/lib/systemd/user-preset"
cat > "${RPM_INSTALL_ROOT}/usr/lib/systemd/user-preset/80-eaststar.preset" << 'PRESET'
enable eaststar.service
PRESET
# Install pre-generated icons (all sizes) for RPM
if [ -d "$ASSETS_DIR/generated-icons/hicolor" ]; then
    for size_dir in "$ASSETS_DIR/generated-icons/hicolor"/*; do
        [ -d "$size_dir" ] || continue
        size_name="$(basename "$size_dir")"
        mkdir -p "${RPM_INSTALL_ROOT}/usr/share/icons/hicolor/${size_name}/apps"
        cp "$size_dir/apps/com.ppmuzyk.eaststar.png" \
           "${RPM_INSTALL_ROOT}/usr/share/icons/hicolor/${size_name}/apps/" 2>/dev/null || true
    done
fi

# AppStream metainfo for GNOME Software
mkdir -p "${RPM_INSTALL_ROOT}/usr/share/metainfo"
cp data/com.ppmuzyk.eaststar.metainfo.xml "${RPM_INSTALL_ROOT}/usr/share/metainfo/"

# Create spec file
SPEC_DIR="dist/staging/rpm-spec"
rm -rf "${SPEC_DIR}"
mkdir -p "${SPEC_DIR}"

cat > "${SPEC_DIR}/eaststar.spec" << SPECEOF
Name:           eaststar
Version:        ${VERSION}
Release:        1%{?dist}
Summary:        GNOME-first Wayland screensaver visualizer
License:        MIT
URL:            https://github.com/ppmuzyk/eastStar
BuildArch:      ${ARCH}

%description
eastStar is a GNOME-first, Wayland-first screensaver application.
Includes Nebula Flight, Pipes (3D), and Fractal Plasma effects.
Burn-in safe with OLED-friendly dark mode.

%files
/usr/local/bin/eaststar
/usr/local/bin/eaststar-saver
/usr/local/bin/eaststar-daemon
/usr/share/applications/com.ppmuzyk.eaststar.desktop
/usr/share/metainfo/com.ppmuzyk.eaststar.metainfo.xml
/usr/share/eaststar/assets/nebula2.png
/usr/share/doc/eaststar/LICENSE
/usr/share/doc/eaststar/README.md
/usr/lib/systemd/user/eaststar.service
/usr/lib/systemd/user-preset/80-eaststar.preset
/usr/lib/systemd/user-preset/80-eaststar.preset
SPECEOF

# Add all icon sizes to %files
for size_dir in "${RPM_INSTALL_ROOT}/usr/share/icons/hicolor"/*; do
    [ -d "$size_dir" ] || continue
    size_name="$(basename "$size_dir")"
    if [ -f "${RPM_INSTALL_ROOT}/usr/share/icons/hicolor/${size_name}/apps/com.ppmuzyk.eaststar.png" ]; then
        echo "/usr/share/icons/hicolor/${size_name}/apps/com.ppmuzyk.eaststar.png" >> "${SPEC_DIR}/eaststar.spec"
    fi
done

# Add post-install message to spec
cat >> "${SPEC_DIR}/eaststar.spec" << 'RPMSCRIPT'
%post
# Try to start the daemon immediately for all currently logged-in users
for uid in $(loginctl list-users --no-legend 2>/dev/null | awk '{print $1}'); do
    if [ -d "/run/user/$uid" ] && [ -S "/run/user/$uid/bus" ]; then
        DBUS_SESSION_BUS_ADDRESS="unix:path=/run/user/$uid/bus" \
            XDG_RUNTIME_DIR="/run/user/$uid" \
            systemctl --user daemon-reload 2>/dev/null || true
        DBUS_SESSION_BUS_ADDRESS="unix:path=/run/user/$uid/bus" \
            XDG_RUNTIME_DIR="/run/user/$uid" \
            systemctl --user enable --now eaststar.service 2>/dev/null || true
    fi
done

echo ""
echo "eastStar: background daemon installed."
echo "If you are currently logged in, the daemon should already be running."
echo "Otherwise it will start automatically on next login."
echo ""
RPMSCRIPT

# Build with rpmbuild
RPM_OUTPUT_DIR="$(pwd)/dist"
rpmbuild -bb \
    --buildroot "${RPM_INSTALL_ROOT}" \
    --define "_rpmdir ${RPM_OUTPUT_DIR}" \
    --define "_rpmfilename %%{NAME}-%%{VERSION}-%%{RELEASE}.%%{ARCH}.rpm" \
    "${SPEC_DIR}/eaststar.spec"

echo "RPM: $(ls -lh "${RPM_OUTPUT_DIR}/eaststar-${VERSION}-1.${ARCH}.rpm" | awk '{print $5}')"

# === Build DEB manually ===
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
mkdir -p "${DEB_ROOT}/usr/lib/systemd/user"

cp "$BIN_DIR/eaststar" "${DEB_ROOT}/usr/local/bin/"
cp "$BIN_DIR/eaststar-saver" "${DEB_ROOT}/usr/local/bin/"
cp "$BIN_DIR/eaststar-daemon" "${DEB_ROOT}/usr/local/bin/"
cp "$ASSETS_DIR/nebula2.png" "${DEB_ROOT}/usr/share/eaststar/assets/" 2>/dev/null || true
mkdir -p "${DEB_ROOT}/usr/share/metainfo"
cp data/com.ppmuzyk.eaststar.metainfo.xml "${DEB_ROOT}/usr/share/metainfo/"
cp LICENSE "${DEB_ROOT}/usr/share/doc/eaststar/copyright"
cp README.md "${DEB_ROOT}/usr/share/doc/eaststar/"

# Desktop entry for DEB
cat > "${DEB_ROOT}/usr/share/applications/com.ppmuzyk.eaststar.desktop" << 'DESKTOP'
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

# systemd user service for DEB
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

# systemd user preset — enables the service by default for all users
mkdir -p "${DEB_ROOT}/usr/lib/systemd/user-preset"
cat > "${DEB_ROOT}/usr/lib/systemd/user-preset/80-eaststar.preset" << 'PRESET'
enable eaststar.service
PRESET

# Install pre-generated icons (all sizes) for DEB
if [ -d "$ASSETS_DIR/generated-icons/hicolor" ]; then
    for size_dir in "$ASSETS_DIR/generated-icons/hicolor"/*; do
        [ -d "$size_dir" ] || continue
        size_name="$(basename "$size_dir")"
        mkdir -p "${DEB_ROOT}/usr/share/icons/hicolor/${size_name}/apps"
        cp "$size_dir/apps/com.ppmuzyk.eaststar.png"            "${DEB_ROOT}/usr/share/icons/hicolor/${size_name}/apps/" 2>/dev/null || true
    done
fi

INSTALLED_SIZE=$(du -sk "${DEB_ROOT}" | cut -f1)

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
rm -f /tmp/eaststar-control.tar.gz /tmp/eaststar-data.tar.gz

echo ""
echo "=== Post-install notes ==="
echo "After installing, enable the background daemon with:"
echo "  systemctl --user daemon-reload"
echo "  systemctl --user enable --now eaststar.service"
echo ""
echo "=== Package summary ==="
ls -lh dist/eaststar*.{rpm,deb,tar.gz} 2>/dev/null || echo "Some packages may have failed - check output above"
