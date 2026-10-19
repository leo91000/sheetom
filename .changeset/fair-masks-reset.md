---
"sheetom": patch
---

Preserve authored mask image defaults and position keywords through safe serialization and reparsing. Make all reset mutations remove affected earlier declarations atomically, including revert-rule, while retaining custom properties, direction and unicode-bidi. Affected property getters expose the reset value and priority until explicitly overridden.
