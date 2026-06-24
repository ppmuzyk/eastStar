# Architecture Sketch

## Goal

Build a Linux screensaver-style application that is pleasant visually but also behaves like a system-integrated desktop app.

The visual direction should prefer low average brightness, continuous motion, sparse highlights, and periodic scene drift so the same pixels are not stressed continuously.

## First Target

The first supported environment is:

- GNOME
- Wayland
- single monitor

## Core Runtime Pieces

### 1. Idle detection

A desktop integration layer determines when the session is inactive long enough to trigger the visual mode.

For GNOME, `eastStar` has its own saver activation delay that replaces the plain "blank screen delay" experience without overriding GNOME's lock policy.

The daemon polls `org.gnome.Mutter.IdleMonitor.GetIdletime` over session DBus via `gdbus` once per second.

### 2. Visual session

A rendering layer owns the fullscreen experience and exposes a simple lifecycle:

- prepare
- tick/update
- dismiss

### 3. Lock integration

A session layer coordinates with the desktop lock state without stealing policy from GNOME.

Target behavior:

- `eastStar` activates after its own inactivity threshold
- optional delayed lock: configurable seconds after saver starts, eastStar requests a system lock
- GNOME's existing screen lock settings continue to decide whether locking happens
- manual lock requests should still work immediately
- saver dismissal happens on user activity and hands control back cleanly

### 4. Platform adapters

The codebase separates generic application flow from GNOME-specific behavior so KDE can be added later without rewriting the core app flow.

## Module Shape

Three binaries:

| Binary | Purpose | Runtime |
|--------|---------|---------|
| `eaststar` | GTK4 preferences panel | User-launched, on-demand |
| `eaststar-daemon` | Background idle monitor | systemd `--user` service, auto-started at login |
| `eaststar-saver` | Fullscreen visual renderer | Spawned by daemon on idle timeout, or by preferences panel for preview |

Library modules:

- `main.rs` — GTK4 preferences panel (settings UI, preview button)
- `app.rs` — legacy macroquad orchestration prototype
- `platform/` — desktop/session integration (idle monitor via DBus)
- `settings/` — config load/save from `~/.config/eaststar/settings.conf`
- `visual/` — rendering and animation logic (Starfield, Pipes, Plasma)
- `lock/` — lock coordination API (loginctl, DBus, xdg-screensaver fallbacks)
- `saver.rs` + `src/bin/eaststar-saver.rs` — fullscreen saver runtime

## Daemon Design

`eaststar-daemon` is the core runtime. It:

1. Loads settings from `~/.config/eaststar/settings.conf`
2. Polls GNOME idle time every second
3. When idle reaches `saver_delay_seconds`, spawns `eaststar-saver` as a child process
4. While the saver is running, skips idle polling (avoids double-launch)
5. If `lock_after_seconds > 0`, requests a system lock that many seconds after saver start
6. Reloads settings each cycle so preferences panel changes are picked up without restart
7. When the saver exits (user moved mouse/pressed key), resumes idle polling

The daemon is packaged as a `systemd --user` service:

```
graphical-session.target → eaststar.service (eaststar-daemon)
```

It uses `Type=simple` with `Restart=on-failure` so it comes back after crashes.

## Data Flow

```
┌─────────────────┐     settings.conf     ┌──────────────────┐
│ eaststar (GTK4) │ ──────────────────────│ eaststar-daemon  │
│  preferences    │     read/write         │  idle monitor    │
└────────┬────────┘                       └────────┬─────────┘
         │ Preview button                           │ idle ≥ delay
         ▼                                          ▼
┌──────────────────────────────────────────────────────────────┐
│                     eaststar-saver                            │
│                (fullscreen macroquad renderer)                │
└──────────────────────────────────────────────────────────────┘
```

- Preferences panel writes to `settings.conf` → daemon reads it live
- Daemon spawns saver process → saver runs fullscreen until user input
- Daemon can also request screen lock via loginctl/DBus

## Early Technical Questions (Resolved)

- ~~Which GNOME/DBus APIs are the best signal for idle state?~~ → `org.gnome.Mutter.IdleMonitor.GetIdletime`
- ~~How should `eastStar` suppress normal blanking while the saver is active, without fighting GNOME's own lock timer?~~ → The saver fullscreen window naturally covers GNOME's blank screen; lock timing is separate.
- ~~How should autostart/background behavior work without forcing the settings window to stay visible all the time?~~ → systemd user service running `eaststar-daemon`.
