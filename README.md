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

This repository is now in an early controller-app phase.

Running the app opens a normal control panel window that:

- polls GNOME idle time through Mutter's idle monitor
- persists its own saver settings to `~/.config/eaststar/settings.conf`
- promotes itself into fullscreen saver mode after the configured inactivity delay
- can optionally request a desktop lock some time after the saver starts
- launches the fullscreen renderer through the separate `eaststar-saver` binary
- leaves GNOME's own lock timing alone

At the moment, the idle watcher is active while the preferences app is running. Autostart/background packaging is still a separate next step.

## Current Controls

- `Activation delay` is configured directly in seconds
- `Lock screen after saver starts` is optional and also configured in seconds
- `Visual effect` currently lets you switch between `Nebula Flight` and `Pipes`
- `Preview Saver` launches the fullscreen renderer immediately without forcing a lock

The saver visual is intentionally dark and low-density to reduce static bright-pixel wear on OLED-like panels. The brightest area drifts over time so the screen center is not stressed continuously.

## GNOME Integration Goal

The intended GNOME behavior is:

- `eastStar` has its own saver inactivity delay
- when that delay is reached, `eastStar` shows the fullscreen visual instead of a plain blank screen
- GNOME's own automatic lock settings still decide whether and when the session locks afterward

So the saver and the lock policy are separate on purpose.

## Planned Milestones

1. Tighten GNOME inactivity behavior against real desktop blanking settings.
2. Add a richer settings panel and proper effect selection UI.
3. Add auto-start / background-run packaging for GNOME sessions.
4. Add multi-monitor handling.
5. Add KDE support.

## Installation

### Local GNOME Install

For a user-local GNOME install from source:

```bash
./install.sh
gtk-launch com.ppmuzyk.eaststar
```

This installs:

- `eaststar` and `eaststar-saver` into `~/.local/bin`
- the desktop entry into `~/.local/share/applications`
- the generated icon theme entries into `~/.local/share/icons/hicolor`

Useful options:

- `./install.sh --debug` installs the debug build instead of release
- `./install.sh --prefix /some/prefix` installs into a custom prefix

To remove the local install:

```bash
./uninstall.sh
```

### Release Packages

When public release packages are published, link them from the repository description and the GitHub Releases page.

Recommended short install copy for release notes or the repo description:

```text
Linux (GNOME/Wayland): download a package from Releases, install it, then launch eastStar from the app grid or with gtk-launch com.ppmuzyk.eaststar.
```

Until those packages exist, the source install flow above is the supported path.
