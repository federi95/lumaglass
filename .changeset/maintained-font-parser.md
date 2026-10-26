---
'lumaglass': patch
---

Replace owned_ttf_parser and ttf-parser with Skrifa's maintained font model and
unhinted scaler. Keep lazy glyph geometry, legacy horizontal kerning, existing
tessellation and CJK/Hangul fallback. Reject invalid font metrics before loading;
retain the existing antialiasing settings. Visual parity still needs verification.
