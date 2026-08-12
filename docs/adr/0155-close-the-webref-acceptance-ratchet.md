# ADR 0155: Close the Webref acceptance ratchet

## Status

Accepted for RC6.

## Context

After modern clip paths and cursor URL sets, three Chromium-accepted Webref branches remained rejected: two multi-component `hyphenate-limit-chars` values and a `font` shorthand using the `math` font size. The single-value `auto` fallback for hyphenation did not model the property's cardinality or its mixed integer and calculation components. Lightning CSS supported `math` as a generic family but not as a font-size value inside the shorthand.

Treating these three observed strings as exceptions would leave adjacent cardinalities, combinations, calculations, and shorthand expansion unproved.

## Decision

- Own `hyphenate-limit-chars` as one to three ordered components, each either `auto` or a positive direct integer, while preserving browser-accepted number-result calculations as `calc()` values.
- Reject direct zero, direct negative and fractional numbers, incompatible dimensions, and a fourth component atomically.
- Add `math` to the vendored typed `FontSize` AST so the ordinary font shorthand parser and longhand expansion own it.
- Differentially cover mixed components, all cardinalities, calculated branches, invalid neighbors, complete font shorthand expansion, and safe round trips.
- Keep the Chromium/Webref acceptance ratchet at zero; future accepted branches must be typed or explicitly documented before release.

## Consequences

All 10,158 Chromium-accepted checks in the pinned 8,369-branch Webref cross-product are accepted by SheetOM, while all invalid-neighbor mutations remain atomic no-ops. The implementation is grammar-driven rather than a literal capability list, and the new branches participate in the same browser, Rust, fuzz, crash-safety, and packaging gates as existing values.
