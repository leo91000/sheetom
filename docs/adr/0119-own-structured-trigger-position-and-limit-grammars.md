# ADR 0119: Own structured trigger, position, and limit grammars

## Status

Accepted for RC6.

## Context

`clip`, `dynamic-range-limit`, `animation-trigger`, and `position-area` were the last non-shape Chromium longhands without a standard parser owner. Their grammars are not interchangeable token lists: `clip` has a legacy quirks-sensitive separator form, dynamic-range mixes recurse and attach bounded percentages, animation triggers combine dashed names with one or two behaviors, and position areas validate and reorder compatible axes.

A declaration value can also consume the entire configured syntax-depth budget while detached from its surrounding rule. Accepting that value would make the subsequently serialized stylesheet exceed the same budget and break SheetOM's safe round-trip contract.

## Decision

SheetOM owns all four grammars as typed Rust values and canonical serializers.

- `clip` distinguishes `auto` from four typed components, accepts either all commas or all spaces, applies the standards-mode unitless rule, and preserves authored math-function shape.
- `dynamic-range-limit` stores recursive mixes, preserves math functions, enforces literal percentage bounds, and rejects an all-literal-zero mix.
- `animation-trigger` stores comma-separated `none` or dashed-name attachments with the complete Chromium behavior vocabulary.
- `position-area` classifies every keyword by axis, validates compatible pairs, restores grammar order, and applies Chromium's repeated/default-value elision.

The browser differential checks 204 reviewed branches with atomic invalid neighbors. It additionally compares all 2,551 one- and two-keyword `position-area` sequences against Chromium 151, including rejected pairs.

Declaration mutations reserve the syntax depth occupied by their live rule ancestry before native parsing. The same reservation applies to `cssText` replacement, so accepted declaration state remains serializable within its stylesheet's configured budget.

## Consequences

- The four shared geometric properties remain the final separately owned grammar family in the ordinary-property catalog.
- Quirks-mode-only nonzero unitless `clip` lengths are not accepted by SheetOM's document-independent standards profile.
- Deep recursive values are exercised in isolated native and public subprocesses at the configured resource boundary.
- A mutation may now throw the documented nesting `RangeError` earlier when its containing rule blocks consume part of the configured syntax-depth budget.
