# ADR 0134: Own the complete transform-origin grammar

## Status

Accepted for RC6.

## Context

CSS Transforms defines `transform-origin` as one or two position components
followed by an optional third-axis `<length>`. Vendored Lightning CSS modeled
the property as a general two-dimensional `Position` and carried an explicit
TODO for the depth offset. Consequently, SheetOM rejected every sampled
Chromium value with a Z component. Reusing the general position parser would
also be incorrect: it accepts three- and four-component side-offset forms such
as `left 10px top`, which Chromium rejects for `transform-origin`.

## Decision

The vendored Lightning CSS source now exposes a typed `TransformOrigin` value
containing a resolved two-dimensional position and an optional `Length` depth:

- one-component values resolve the omitted axis to `center`;
- two-component values enforce horizontal and vertical roles while allowing
  keyword reordering such as `top left`;
- a third component must be a length, including zero and typed calculations;
- percentages, percentage calculations, side-offset positions and fourth
  components are rejected;
- non-minified serialization emits both position axes, and emits the depth
  whenever it was explicitly authored, including an explicit zero;
- SheetOM's observable projection retains calculation provenance and the
  browser's dimensioned-zero spelling.

The source correction is committed separately from the SheetOM integration so
it can be reviewed and proposed upstream without unrelated compatibility data.

## Evidence

Vendored Rust tests cover axis resolution, keyword reordering, positive,
negative and zero depths, and invalid neighboring forms. SheetOM tests cover
the public CSSOM getter, alias state, atomic replacement and serialization
round-trips. Native subprocess and Chromium differential cases cover calculated
depth, canonical ordering, zero units, the WebKit alias and invalid mutation.

The full Webref-derived differential removes all 22 acceptance mismatches for
`transform-origin` and `-webkit-transform-origin`. All 240 native declaration
sequences and the value, math and color capability corpora match Chromium.

## Consequences

- Valid three-dimensional origins survive parsing and stylesheet ownership.
- Invalid general-position forms cannot mutate an existing declaration.
- The vendored model no longer needs an untyped fallback or an exact-value
  capability override for this grammar.
- Future changes to transform-origin syntax belong in the typed vendored value
  and its browser differential evidence.
