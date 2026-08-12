# ADR 0133: Own the WebKit mask box image grammar

## Status

Accepted for RC6.

## Context

Chromium exposes `-webkit-mask-box-image` as a legacy five-longhand shorthand.
Its grammar resembles `border-image`, but its observable CSSOM state is not an
alias of the standard shorthand. Omitted components become the CSS-wide
`initial` value, every explicit slice gains `fill`, the shorthand getter stays
empty, and the declaration block serializes the five prefixed longhands.

The previous implementation expanded this property through the standard
`border-image` codec and then rewrote longhand names. That path appended a
second `fill`, could not represent an omitted width between two slashes,
misclassified unordered source and repeat components, and only supported a
single value in the prefixed slice, width and outset longhands. It rejected 29
Chromium-accepted shorthand branches and two accepted slice-longhand branches.
Repeated border-image components also retained a noncanonical four-value
spelling in eight observable Webref checks.

## Decision

SheetOM owns a dedicated typed grammar for this legacy family:

- the shorthand parses an optional source and one- or two-keyword repeat group
  before the slice group or after the final width/outset group, matching
  Chromium's ordering and rejecting interleaved components;
- a slice contains one to four nonnegative number or percentage components and
  one optional `fill` token in any position; shorthand expansion always adds a
  single canonical `fill`;
- width accepts one to four nonnegative length-percentage, number or `auto`
  components, while outset accepts one to four nonnegative lengths or numbers;
- `slice / / outset` preserves an explicitly omitted width as `initial`;
- one-to-four component lists use CSS quad compression after semantic parsing;
- escaped and case-insensitive identifiers are recognized through CSS tokens,
  including escaped whitespace terminators, rather than raw prefix checks;
- omitted source, slice, width, outset and repeat components become `initial`,
  while explicit `none` remains a source value;
- `alpha` and `luminance` remain rejected because the pinned Chromium build
  does not accept the mask-border-mode branch on this prefixed property;
- the parser exchanges only owned Rust semantic values with vendored Lightning
  CSS. No JavaScript typed AST is returned to N-API.

The same semantic four-side list projection is used for standard
`border-image-width` and `border-image-outset`, which closes their equivalent
canonicalization gaps without coupling runtime behavior to Webref literals.

## Evidence

Rust tests cover optional slash sections, CSS escapes, one-to-four component
cardinality, canonical compression, invalid ordering and atomic replacement.
Public tests cover parsing, indexed state, longhand mutation and removal,
`image-set()` serialization, and idempotent reparsing. The native Chromium
differential covers the complete shorthand, direct longhand lists, removal and
invalid replacement. Dedicated subprocesses execute both the shorthand and
source longhand with multi-candidate `image-set()` through the native and public
interfaces.

The generated Webref differential now reports zero mismatch for all 84 sampled
`-webkit-mask-box-image` branches. The ratchet removes 40 mismatch cases in
total: 31 acceptance cases, eight getter/`cssText`/item-order cases, and one
reparse case, without introducing any atomicity regression.

## Consequences

- Valid legacy mask box images are no longer dropped from parsed stylesheets.
- Direct prefixed longhand mutation follows Chromium for quad lists.
- `image-set()` cannot reach the former JavaScript-to-native AST round-trip.
- The public shorthand getter intentionally remains empty; the expanded
  longhands are the observable state and the safe stylesheet representation.
- Future grammar changes must update the token grammar and differential
  evidence instead of adding exact accepted-value overrides.
