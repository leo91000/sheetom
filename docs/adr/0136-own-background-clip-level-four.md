# ADR 0136: Own Backgrounds Level 4 clipping

## Status

Accepted for RC6.

## Context

Chromium's `background-clip` grammar accepts the Backgrounds Level 4 values
`border-area`, `text`, and the unordered compound `border-area text`. The
vendored Lightning source used a legacy `border` spelling and represented a
clip as one enum keyword, so it could neither parse the compound value nor
expand valid `background` layers containing it. This caused 15 valid Chromium
declarations to become atomic no-ops, including multilayer backgrounds.

Shorthand observability adds two non-obvious rules. A layer containing
`border-area` without an authored visual origin exposes `border-box` as its
`background-origin`, while a `text`-only layer keeps the origin observable as
`initial`. After the containing `background` shorthand is broken, a repeated
list of CSS-wide `initial` position values cannot be reconstructed as a valid
`background-position` shorthand.

## Decision

The vendored Lightning `BackgroundClip` value is now a typed enum that owns
visual boxes, `border-area`, `text`, and the canonical compound
`border-area text`:

- the compound parses in either keyword order and serializes in canonical
  order;
- duplicate keywords, the obsolete `border` spelling, and visual-box compounds
  are rejected by the longhand grammar;
- a shorthand layer with `border-area` and no visual origin receives an
  explicit `border-box` origin;
- text-aware prefix handling recognizes both the simple and compound values.

SheetOM projects the authored layer tokens into Chromium's observable origin
and clip lists, synthesizes a text-only layer without inventing an origin, and
requires all other non-initial clips to have a compatible origin. Its
`background-position` synthesizer now zips layer lists and refuses CSS-wide
keywords embedded in a multi-layer shorthand.

## Evidence

The vendored source has focused parser and serializer tests for direct
longhands, shorthand ordering, multilayer values, and invalid neighbors. Public
CSSOM tests cover canonical getters, direct clip lists, atomic invalid
replacement, longhand mutation, priority mismatch, group breakage, and
reparsable stylesheet round trips. Native differentials execute the same
sequences against pinned Chromium, and a subprocess combines the compound clip
with `image-set()` through both native and public boundaries.

The Webref-derived gate removes 19 mismatch cases: all 15 related acceptance
mismatches, two observable/cssText mismatches, and four item-order mismatches,
without changing atomicity or reparse counts.

## Consequences

- Valid `border-area` custom CSS survives SheetOM ownership and serialization.
- The runtime grammar is typed and upstreamable rather than an observed literal
  allowlist.
- Broken multilayer backgrounds no longer emit an invalid nested position
  shorthand.
- Future Backgrounds Level 4 clip changes belong in the vendored typed value,
  with SheetOM retaining only browser-facing CSSOM policy.
