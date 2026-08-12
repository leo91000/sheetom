# ADR 0137: Keep numeric extensions scalar

## Status

Accepted for RC6.

## Context

SheetOM extends Lightning CSS for context-dependent numeric calculations that
Chromium accepts. The extension router previously claimed any value whose first
token looked numeric, even when that token was only one component of a typed
grammar. Consequently `border-image-slice: 1 fill` was sent to a scalar parser
and rejected before Lightning could parse the complete value.

The vendored `BorderImageSlice` parser also accepted direct negative numbers,
although Chromium rejects those at parse time while retaining calculated
negative values until computed-value time.

## Decision

A numeric extension may preempt the standard property parser only when the
entire top-level value is one component. Multi-component values continue
through the typed property grammar, which owns their keywords, cardinality and
ordering. The vendored border-image parser rejects direct negative numbers and
percentages before building its four-sided value, without rejecting math
functions whose range is intentionally deferred.

## Evidence

Vendored and core tests cover `fill` before and after one through four slice
components, direct percentages, context-dependent math, and adjacent invalid
values. Public CSSOM tests verify observable canonicalization, atomic invalid
replacement, shorthand mutation, removal and reparsable round trips. Native
Chromium differentials exercise direct and shorthand sequences, and the crash
suite combines `image-set()`, `fill` and context-dependent math in a subprocess.
The Webref-derived ratchet removes 72 mismatch cases: 21 false rejections, 46
observable/cssText mismatches, 46 item mismatches and five reparse failures.
Atomicity remains at zero mismatches.

## Consequences

- Valid typed grammars can no longer be shadowed merely because they begin with
  a number.
- `border-image-slice` accepts every Chromium `fill` cardinality while keeping
  direct range errors atomic.
- Future numeric extensions must prove that they own the whole top-level value,
  not only its first token.
