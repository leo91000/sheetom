# ADR 0125: Own the complete font-variant grammar

## Status

Accepted for RC6.

## Context

`font-variant` combines seven independent longhand grammars with unordered
components. Chromium accepts hundreds of valid combinations that the vendored
Lightning CSS shorthand parser does not currently expose as a typed property.
Keeping only `normal` and `none` made valid authored declarations disappear.

The observable order is not a single metadata order. For non-keyword values,
Chromium stores expanded longhands as ligatures, numeric, East Asian, caps,
alternates, position and emoji. Shorthand synthesis uses the grammar order, and
some component grammars have their own canonical order. `normal`, `none` and
CSS-wide keywords retain their existing expansion orders.

## Decision

SheetOM classifies each top-level shorthand component by parsing it against all
seven owned longhand grammars. Exactly one longhand must accept each component.
The complete grouped longhand value is parsed again before mutation so duplicate
or mutually exclusive components are rejected atomically.

Expansion and synthesis use separate explicit Chromium orders. East Asian
keywords serialize in variant, width and ruby order. Alternate functions
serialize as stylistic, historical forms, styleset, character variant, swash,
ornaments and annotation. Numeric and ligature keywords preserve Chromium's
authored ordering instead of being sorted generically.

The generated Webref corpus remains test-only evidence. Every accepted
`font-variant` sample must match Chromium for acceptance, getter, `cssText`,
indexed longhands, invalid-neighbor atomicity and safe reparsing.

## Consequences

- All generated Chromium-supported `font-variant` branches are accepted without
  a literal runtime allowlist.
- Invalid cross-component combinations remain atomic no-ops.
- Longhand mutation can reconstruct the shorthand only while all seven members
  remain present and compatible.
- Grammar order and declaration-list order stay separate, preventing metadata
  regeneration from silently changing CSSOM observability.
