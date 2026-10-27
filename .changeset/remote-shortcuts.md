---
'lumaglass': minor
---

Add opt-in remote color-button navigation in General → Remote shortcuts, with
per-button actions, reset and session reception indicators in all nine languages.
Normalize native SDL-webOS color scancodes and keycode variants before Rust enum
conversion. Keep shortcuts local to the focused launcher, suppress held/repeated
presses and modal interference, and provide desktop-only F1–F4 aliases.
