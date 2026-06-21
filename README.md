# eastStar

`eastStar` is a GNOME-first, Wayland-first screensaver-style application written in Rust.

## Product Direction

The first milestone is a local Linux app that:

- detects user inactivity on GNOME
- shows a fullscreen visual experience
- locks the session as part of the flow
- works on a single monitor first
- leaves room for future KDE and multi-monitor support

## Current State

This repository is in the bootstrap phase. The first code target is a thin executable shell plus architecture notes for:

- idle detection
- fullscreen rendering
- session locking
- desktop-specific adapters

## Planned Milestones

1. Define the runtime architecture and interfaces.
2. Add a minimal app shell that can start, log, and exit cleanly.
3. Implement GNOME-focused idle detection.
4. Implement fullscreen visual mode on Wayland.
5. Integrate session locking.
6. Add multi-monitor handling.
7. Add KDE support.

## Notes

Rust tooling is not installed on this host yet, so this is a manually scaffolded Cargo project.
