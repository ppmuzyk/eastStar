# Architecture Sketch

## Goal

Build a Linux screensaver-style application that is pleasant visually but also behaves like a system-integrated desktop app.

The current implementation is an early controller app with a control panel, a saved inactivity threshold, GNOME idle polling, and a fullscreen saver mode spawned through a separate renderer binary.

The visual direction should prefer low average brightness, continuous motion, sparse highlights, and periodic scene drift so the same pixels are not stressed continuously.

## First Target

The first supported environment is:

- GNOME
- Wayland
- single monitor

## Core Runtime Pieces

### 1. Idle detection

A desktop integration layer determines when the session is inactive long enough to trigger the visual mode.

For GNOME, `eastStar` should eventually have its own saver activation delay. That saver delay is meant to replace the plain "blank screen delay" experience, not override GNOME's lock policy.

The current prototype polls `org.gnome.Mutter.IdleMonitor.GetIdletime` over session DBus via `gdbus`.

### 2. Visual session

A rendering layer should own the fullscreen experience and expose a simple lifecycle:

- prepare
- tick/update
- dismiss

### 3. Lock integration

A session layer should coordinate with the desktop lock state without stealing policy from GNOME.

Target behavior:

- `eastStar` activates after its own inactivity threshold
- GNOME's existing screen lock settings continue to decide whether locking happens
- manual lock requests should still work immediately
- saver dismissal should happen on user activity and hand control back cleanly

### 4. Platform adapters

The codebase should separate generic application flow from GNOME-specific behavior so KDE can be added later without rewriting the core app flow.

## Proposed Module Shape

- `main.rs`: bootstrap and process entrypoint
- `app.rs`: legacy macroquad orchestration prototype kept around during the split
- `platform/`: desktop/session integration
- `settings/`: config load/save and effect settings
- `visual/`: rendering and animation logic
- `lock/`: lock coordination API
- `saver.rs` + `src/bin/eaststar-saver.rs`: fullscreen saver runtime

## Early Technical Questions

- Which GNOME/DBus APIs are the best signal for idle state?
- How should `eastStar` suppress normal blanking while the saver is active, without fighting GNOME's own lock timer?
- How should autostart/background behavior work without forcing the settings window to stay visible all the time?
