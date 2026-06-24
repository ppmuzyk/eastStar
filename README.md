# eastStar

`eastStar` is a GNOME-first, Wayland-first screensaver-style application written in Rust.

## Product Direction

The first milestone is a local Linux app that:

- detects user inactivity on GNOME
- shows a fullscreen visual experience
- replaces blank-screen behavior with a saver visual
- leaves actual session lock timing under GNOME system settings
- works on a single monitor first
- leaves room for future KDE and multi-monitor support

## Current State

This repository is now in an early controller-app phase with a background daemon.

Running the app opens a normal control panel window that:

- lets you configure the saver activation delay and visual effect
- persists settings to `~/.config/eaststar/settings.conf`
- provides a "Preview Saver" button for immediate fullscreen preview

The background daemon (`eaststar-daemon`) runs independently as a systemd user service:

- polls GNOME idle time through Mutter's idle monitor once per second
- launches the fullscreen saver (`eaststar-saver`) when idle reaches the configured delay
- can optionally request a desktop lock after the saver has been active for a configurable time
- automatically picks up settings changes from the preferences panel (reloads config each cycle)
- leaves GNOME's own lock timing alone

## Current Controls

- `Activation delay` is configured directly in seconds
- `Lock screen after saver starts` is optional and also configured in seconds
- `Visual effect` currently lets you switch between `Nebula Flight`, `Pipes`, and `Fractal Plasma`
- `Preview Saver` launches the fullscreen renderer immediately without forcing a lock

The saver visual is intentionally dark and low-density to reduce static bright-pixel wear on OLED-like panels. The brightest area drifts over time so the screen center is not stressed continuously.

## GNOME Integration Goal

- `eastStar` has its own saver inactivity delay
- when that delay is reached, `eastStar` shows the fullscreen visual instead of a plain blank screen
- GNOME's own automatic lock settings still decide whether and when the session locks afterward

## Architecture

Three binaries:

| Binary | Role |
|--------|------|
| `eaststar` | GTK4 preferences panel |
| `eaststar-daemon` | Background idle monitor (systemd user service) |
| `eaststar-saver` | Fullscreen visual renderer |

The daemon is the core runtime — it runs in the background, watches idle time, and spawns the saver when needed. The preferences panel is only needed when you want to change settings.

## Completed Milestones

1. **Tighten GNOME inactivity behavior** — Uses Mutter idle monitor with configurable saver delay and optional screen lock.
2. **Settings panel and effect selection UI** — GTK4 preferences app with delay/lock controls, visual effect picker (Nebula Flight / Pipes / Fractal Plasma), and preview.
3. **Background daemon with systemd integration** — `eaststar-daemon` runs as a `systemd --user` service, monitors idle time independently of the preferences window, and auto-launches the saver.

## Planned Milestones

1. Add multi-monitor handling.
2. Add KDE support.

## Installation

### Local GNOME Install

For a user-local GNOME install from source:

```bash
./install.sh
```

This installs:

- `eaststar`, `eaststar-daemon`, and `eaststar-saver` into `~/.local/bin`
- the desktop entry into `~/.local/share/applications`
- the generated icon theme entries into `~/.local/share/icons/hicolor`
- a systemd user service at `~/.config/systemd/user/eaststar.service`

The systemd service is enabled and started automatically. The daemon begins watching idle time immediately after login.

Useful options:

- `./install.sh --debug` installs the debug build instead of release
- `./install.sh --prefix /some/prefix` installs into a custom prefix
- `./install.sh --no-systemd` skips systemd service setup

To remove the local install:

```bash
./uninstall.sh
```

### Managing the Daemon

```bash
systemctl --user status eaststar         # check if running
systemctl --user stop eaststar           # stop the daemon
systemctl --user start eaststar          # start the daemon
systemctl --user restart eaststar        # restart after settings changes
journalctl --user -u eaststar -f         # follow daemon logs
```

To disable automatic startup:

```bash
systemctl --user disable eaststar
```

### Release Packages

Release builds are distributed in three formats:

| Format | File |
|--------|------|
| RPM (Fedora/RHEL) | `eaststar-0.1.0-1.x86_64.rpm` |
| DEB (Debian/Ubuntu) | `eaststar_0.1.0-1_amd64.deb` |
| Tarball (any Linux) | `eaststar-0.1.0-x86_64-unknown-linux-gnu.tar.gz` |

Download from the [GitHub Releases](https://github.com/ppmuzyk/eastStar/releases) page.

#### RPM (Fedora, RHEL, CentOS Stream)

```bash
sudo rpm -i eaststar-0.1.0-1.x86_64.rpm
systemctl --user daemon-reload
systemctl --user enable --now eaststar
```

#### DEB (Debian, Ubuntu, Pop!_OS)

```bash
sudo dpkg -i eaststar_0.1.0-1_amd64.deb
systemctl --user daemon-reload
systemctl --user enable --now eaststar
```

#### Tarball (any Linux)

```bash
tar xzf eaststar-0.1.0-x86_64-unknown-linux-gnu.tar.gz
cd eaststar-0.1.0-x86_64-unknown-linux-gnu
./install.sh
```

All formats include:

- `eaststar`, `eaststar-daemon`, and `eaststar-saver` (release-optimized binaries)
- GNOME desktop entry (`com.ppmuzyk.eaststar.desktop`)
- systemd user service unit (`eaststar.service`)
- MIT license and documentation
