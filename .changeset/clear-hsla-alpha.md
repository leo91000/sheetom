---
"sheetom": patch
"@sheetom/wasm": patch
---

Match browser CSSOM for authored HSL colors. Require percentages for saturation and lightness in legacy comma syntax, reject legacy `none` alpha, and serialize colors with an alpha spelling that preserves their stored value and matches browser output.
