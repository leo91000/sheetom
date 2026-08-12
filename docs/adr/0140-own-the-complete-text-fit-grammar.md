# ADR 0140: Own the complete text-fit grammar

## Status

Accepted for RC6.

## Context

SheetOM previously recognized only the three leading `text-fit` modes. The
Chromium grammar also accepts an optional line strategy and percentage limit in
a fixed order:

`[ none | grow | shrink ] [ consistent | per-line | per-line-all ]? <percentage>?`

Direct negative percentages are parse-time invalid, while negative results
inside CSS math are retained until computed-value time. CSSOM also preserves
the authored math function boundary: a reduced `calc()` remains wrapped and
functions such as `min()` keep their shape.

## Decision

`text-fit` has a dedicated typed browser-longhand value with a required mode,
optional line strategy and optional calculated percentage. The parser enforces
component order and cardinality. It rejects direct negative limits without
rejecting deferred calculated negatives.

The value stores whether the limit was authored as a math function so getter,
`cssText` and safe serialization retain Chromium's observable function shape.
Pending substitutions continue through the generic token-preserving path.

## Evidence

Core and public tests cover all modes, all line strategies, omitted optional
components, direct and calculated limits, function preservation, reordered and
duplicate invalid neighbors, atomic replacement, pending substitution and
idempotent whole-sheet serialization.

The versioned value corpus executes five accepted and four rejected branches
against Chromium. Native differentials cover a complete value, reduced math
and invalid replacement. The process-safety suite evaluates nested percentage
math in isolated native and public subprocesses.

The Webref-derived ratchet removes all eight remaining `text-fit` false
rejections. Atomicity remains at zero mismatches.

## Consequences

- Valid modern `text-fit` declarations are no longer silently dropped.
- Direct range validation and computed-time math validation remain distinct.
- The property is owned by one structured grammar rather than a growing list
  of accepted strings.
