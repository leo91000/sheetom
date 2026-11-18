---
status: accepted
---

# Validate legacy HSL channels and round-trip their alpha

CSS Color 4 gives `hsl()` and `hsla()` the same grammar. Comma syntax requires
percentage saturation and lightness, including calculations of percentage
type. Space-separated syntax also accepts numeric channels. The legacy alpha
grammar in both HSL and RGB accepts a number or percentage. See [CSS Color
4](https://drafts.csswg.org/css-color-4/#the-hsl-notation).

Enforce those distinctions in the vendored typed color parser, so they apply to
declarations, support queries, and nested colors on both backends. Use the
parsed sRGB bytes when projecting resolved HSL colors to observable CSSOM.
For non-opaque alpha, prefer a whole percentage when it reparses to the same
byte, with the three-decimal byte value as the fallback. Apply the same
serialization to hex colors so a color projected from HSL and its safe
serialized form agree on the stored alpha. The safe stylesheet printer can
still select a named or hexadecimal color with identical sRGB state.
See [CSS Color 4 alpha serialization](https://drafts.csswg.org/css-color-4/#serializing-alpha-values).

Chromium 151, Firefox 153, Chromium 153, and WebKit 26.5 agree on the scoped corrections.
The [dated audit](../css-baseline-audit-2026-11-18.md) records their observations
and the shared native/WASM contract.
