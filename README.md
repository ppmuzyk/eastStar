# eastStar

`eastStar` is a GNOME-first, Wayland-first screensaver application written in Rust.

## Features

- Detects user inactivity on GNOME via Mutter's idle monitor
- Fullscreen animated visuals: Nebula Flight, Pipes (3D), Fractal Plasma
- OLED-safe dark visuals with drifting bright areas
- Configurable activation delay and optional auto-lock
- Background daemon runs automatically — no need to keep any window open
- Packaged for RPM (Fedora/RHEL) and DEB (Debian/Ubuntu)

## Quick Start

Download the latest package from [GitHub Releases](https://github.com/ppmuzyk/eastStar/releases).

**Fedora/RHEL:**
```bash
sudo rpm -i eaststar-0.2.0-1.x86_64.rpm
```

**Debian/Ubuntu:**
```bash
sudo dpkg -i eaststar_0.2.0-1_amd64.deb
```

The daemon starts automatically after installation if you're currently logged in. Otherwise it starts on next login.

Launch preferences to adjust settings:
```bash
gtk-launch com.ppmuzyk.eaststar
```

## Architecture

Three binaries, one purpose:

| Binary | Role |
|--------|------|
| `eaststar` | GTK4 preferences panel |
| `eaststar-daemon` | Background idle monitor (systemd user service) |
| `eaststar-saver` | Fullscreen visual renderer |

The daemon runs as a background service, polls GNOME idle time every second, and spawns the saver when idle reaches your configured delay. The preferences panel is only needed when you want to change settings — it writes to `~/.config/eaststar/settings.conf` and the daemon picks up changes live.

```
Preferences panel ──writes──→ settings.conf ──reads──→ Daemon ──spawns──→ Saver
                         (live reload)            (idle monitor)     (fullscreen)
```

## Settings

| Setting | Default | Range |
|---------|---------|-------|
| Activation delay | 180s (3 min) | 30–3600s |
| Lock after saver | 0 (disabled) | 0–7200s |
| Visual effect | Nebula Flight | Nebula Flight / Pipes / Fractal Plasma |

## Managing the Daemon

```bash
systemctl --user status eaststar         # check if running
systemctl --user stop eaststar           # temporarily stop
systemctl --user start eaststar          # manually start
systemctl --user restart eaststar        # restart (picks up binary updates)
journalctl --user -u eaststar -f         # follow logs
```

To disable automatic startup:
```bash
systemctl --user disable eaststar
```

## Building from Source

```bash
git clone https://github.com/ppmuzyk/eastStar.git
cd eastStar
./install.sh
```

Options:
- `--debug` — debug build
- `--prefix /path` — custom install prefix
- `--no-systemd` — skip systemd service setup

To remove:
```bash
./uninstall.sh
```

## Building Release Packages

```bash
./build-packages.sh
```

Produces `dist/` with RPM, DEB, and tarball.

## License

MIT
