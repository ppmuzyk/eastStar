# Project Instructions

- Keep the first implementation GNOME-first and Wayland-first.
- Prefer small, testable modules over one large app file.
- Avoid adding desktop-specific abstractions too early unless they clarify interfaces.
- Do not add KDE-specific code until the GNOME path is stable.
- Keep dependencies lean in the bootstrap stage.
- Favor explicit logging and comments around desktop/session integration points.
- When architecture choices are uncertain, document tradeoffs in `docs/` before broad implementation.
