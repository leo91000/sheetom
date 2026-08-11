# ADR 0122: Enforce browser property domains after typed parsing

## Status

Accepted for RC6.

## Context

A parser can produce a typed CSS value while still accepting a value outside the domain of the property that owns it. Shared value types intentionally permit combinations needed by other properties, older grammar editions, or calculations whose validity is deferred until computed-value time. Treating a typed AST as sufficient proof therefore created false positives for negative sizes, invalid keyword branches, compound origins and legacy aliases. Conversely, newer Chromium branches could be rejected by the pinned upstream grammar even though SheetOM already owned the required semantic value family.

## Decision

SheetOM validates property-specific domains before declaration mutation. A
browser-capability constraint runs on the authored property identity and its
structured CSS Syntax components after substitution classification but before
the standard and extension parser alternatives. Typed-domain constraints then
inspect the parsed property and canonical longhands. This ordering prevents a
legacy alias or fallback grammar from accepting a value rejected by the pinned
browser while preserving pending-substitution behavior.

Validators consume CSS Syntax tokens or typed values; they do not use raw
prefix matching, scan intermediate serialized declarations, or import browser
observations into runtime tables. Direct values with statically invalid ranges
are rejected, while context-dependent calculations remain accepted when their
sign or result cannot be decided at parse time.

Newer general grammar branches are implemented in the vendored Lightning CSS source when they extend an existing typed grammar. SheetOM-owned browser grammar fallbacks are allowed only as reviewed typed productions attempted after the standard parser rejects a value. They must parse the complete input, include invalid-neighbor tests, and may not be generated from exact observed values.

Static shorthands may commit only a complete canonical longhand set. Shorthand expansion rejects `Unparsed` and `Custom` Lightning variants, preserves invalid-replacement atomicity, and derives observable synthesis from that longhand state.

## Consequences

- A typed AST is necessary but not independently sufficient evidence of Chromium acceptance.
- A standards-valid branch not implemented by the pinned Chromium baseline is
  represented as a property-family capability constraint, never an
  exact-observed-value exception.
- Browser-specific policy remains outside upstreamable Lightning CSS grammar changes.
- The complete Property Value Matrix gates acceptance, observable value, declaration serialization, item order and atomicity with zero unexplained differences.
- Future browser drift must become a reviewed grammar or domain change rather than a literal runtime exception.
