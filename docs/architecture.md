# Architecture Sketch

## Goal

Build a Linux screensaver-style application that is pleasant visually but also behaves like a system-integrated desktop app.

The current implementation is a manual fullscreen visual demo so rendering and app structure can be tested before idle and lock integration are real.

## First Target

The first supported environment is:

- GNOME
- Wayland
- single monitor

## Core Runtime Pieces

### 1. Idle detection

A desktop integration layer should determine when the session is inactive long enough to trigger the visual mode.

### 2. Visual session

A rendering layer should own the fullscreen experience and expose a simple lifecycle:

- prepare
- show
- tick/update
- hide

### 3. Lock integration

A session layer should trigger or coordinate locking when the saver activates.

### 4. Platform adapters

The codebase should separate generic application flow from GNOME-specific behavior so KDE can be added later without rewriting the core app flow.

## Proposed Module Shape

- `main.rs`: bootstrap and process entrypoint
- `app.rs`: high-level application state and orchestration
- `platform/`: desktop/session integration
- `visual/`: rendering and animation logic
- `lock/`: lock coordination API

## Early Technical Questions

- Which Wayland/windowing stack should back the fullscreen visual layer?
- Which GNOME/DBus APIs are the best signal for idle state?
- Should locking be delegated entirely to the desktop session, or should the app coordinate pre-lock transitions?
