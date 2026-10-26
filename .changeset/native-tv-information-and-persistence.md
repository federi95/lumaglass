---
"lumaglass": minor
---

Add a native, read-only TV information page showing model, webOS SDK, firmware,
elevated helper access, Home replacement observations and installation location.
Desktop preview explicitly shows that TV data is not applicable.

Move preference writes off the SDL thread, flush atomic preference/cache writes,
refresh the local timezone offset while running, bound subscription line reads,
resolve autostart from the installed package and verify pinned SDK archives before
extraction and relocation.
