---
'lumaglass': minor
---

Reject foreign, nested and stacked Home replacement mounts before applying or
restoring Lumaglass integration. Keep disabled boot application inactive and
provide an explicit recovery path when the saved takeover state cannot be read.

Detect known Luma/Glasshouse Home startup configuration before takeover changes,
while allowing Lumaglass recovery without altering other projects' state or hooks.
