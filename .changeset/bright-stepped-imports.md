---
"sheetom": patch
---

Correct stepped CSS math and conditional imports on the native and WASM backends. Number-valued `round()` accepts an omitted step, negative nearest ties round upward, and `round()`, `mod()`, and `rem()` reject incompatible operands. `CSSImportRule` preserves quoted supports conditions and the following media queries through edits and serialization.
