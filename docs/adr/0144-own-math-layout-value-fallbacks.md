# ADR 0144: Own math layout value fallbacks

- Status: Accepted
- Date: 2026-08-12

## Context

Chromium 151 accepts newer MathML and CSS Inline branches that the vendored
typed property parser does not yet recognize: `font-size: math`,
`baseline-shift: sub | super`, `text-transform: math-auto`, and the
`auto-add | add(<integer>)` branches of `math-depth`. Rejecting them loses valid
authored declarations. Treating them as arbitrary token streams would instead
weaken neighboring grammar and mutation atomicity.

## Decision

SheetOM adds a post-standard-parser fallback for the reviewed keyword branches
and a dedicated `math-depth` parser. The parser:

- consumes the complete value;
- requires a lexical integer for a direct `add()` argument;
- permits deferred integer calculations and canonicalizes reduced math inside
  `add(calc(...))` exactly as Chromium does;
- leaves substitutions on the shared pending-substitution path; and
- rejects extra tokens, empty functions, unknown functions, and mixed
  `math-auto` values atomically.

The fallback is attempted only after the standard typed parser rejects a value,
so existing length, percentage, keyword, and integer branches keep their typed
ownership.

## Consequences

The four affected properties retain their complete existing grammar while the
new branches gain semantic state, Chromium differential coverage, invalid
neighbors, whole-sheet round trips, and subprocess crash coverage. The Webref
acceptance ratchet removes eight measured false rejections.
