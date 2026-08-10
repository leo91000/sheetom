---
"sheetom": patch
---

Expose custom `@function` rules, typed parameters, return types, conditional
declaration runs, and live function descriptors through a Rust-owned parser
and Chromium-differential CSSOM interface. Preserve internal token-stream
comments and CSS-significant Unicode whitespace, and keep recovered group-rule
insertion atomic when trailing input is invalid.
