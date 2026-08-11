# ADR 0132: Own the rule-inset grammar and CSSOM state model

## Status

Accepted for RC6.

## Context

Chromium exposes a family of `rule-inset` properties for the cap and junction
ends of row and column rules. The three full shorthands accept one or two cap
values, optionally followed by `/` and one or two junction values. The component
shorthands then project those values across an axis or both axes.

The vendored Lightning CSS snapshot has no typed representation for this family.
SheetOM's former structural fallback treated each shorthand as an ordinary
space-separated one-to-four-value property. It rejected valid slash-separated
values and `overlap-join`, accepted invalid cardinalities through neighboring
fallbacks, and could not reproduce Chromium's expanded longhand state or
canonical shorthand getter. This left 71 Webref acceptance mismatches across
the full family.

## Decision

SheetOM owns a dedicated rule-inset grammar in its native CSSOM layer:

- each physical longhand accepts `overlap-join` or a typed
  `<length-percentage>`, including negative values and CSS math;
- `column-rule-inset`, `row-rule-inset` and `rule-inset` parse one or two cap
  values plus an optional slash and one or two junction values;
- omitting the slash copies the cap pair into the junction pair;
- cap and junction component shorthands accept one or two values, while start
  and end component shorthands accept exactly one value;
- every static shorthand is stored only as its canonical physical longhands;
  substitution-bearing shorthands continue to use pending-substitution groups;
- synthesis compresses a full shorthand to one value only when all four
  components are identical. Otherwise it emits both cap values, `/`, and both
  junction values, matching Chromium's observable serialization;
- invalid syntax, extra components, extra slashes, comma lists and values from
  another property domain are atomic no-ops.

This grammar is deliberately implemented above Lightning CSS. It depends on
SheetOM's token-aware syntax helpers and typed longhand validators, and does not
round-trip an external JavaScript AST through the native parser.

## Evidence

Rust tests cover longhand domains, negative values, CSS math, `overlap-join`,
all shorthand cardinalities, mutation, removal, priorities, pending
substitutions and invalid replacement. Public API tests cover indexed order,
getters, `cssText` and idempotent safe serialization. The native Chromium
differential compares representative state sequences. The generated Webref
ratchet removes all 71 acceptance mismatches in the family without introducing
atomicity or reparse mismatches.

## Consequences

- Valid rule-inset declarations are no longer dropped from parsed stylesheets.
- Longhand mutation cannot reactivate stale shorthand state.
- CSSOM getters and declaration serialization follow Chromium's canonical
  slash form rather than preserving a noncanonical authored spelling.
- Future additions to this family must extend the dedicated grammar and its
  differential evidence instead of falling back to generic cardinality logic.
