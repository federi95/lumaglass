---
'lumaglass': minor
---

Add a native read-only Privacy page for Live Plus, limit-ad-tracking and a fixed
set of consent records, preserving unknown/denied/unavailable states. Reads use
a bounded worker and do not request advertising identifiers or start ACR services.
