---
"sheetom": minor
"@sheetom/wasm": minor
---

Add revision-pinned CSS mixin authoring interfaces, CSS.escape(), CSS.supports(),
and the StyleSheet base interface. Support linear() easing and gradient color
interpolation across property contexts; correct selector CSSOM serialization
and namespace-aware mutation. Gate the June 2026 authoring target on native and
WASM backends, including deep-input and invalid-mutation checks.

Validate attribute namespaces and recover forgiving selector lists correctly,
retain namespace state after detachment, and preserve inactive animation
settings through serialization. Compare rule and declaration state on reparse.
