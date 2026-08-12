# ADR 0159: Classify border components explicitly

## Status

Accepted for RC6.

## Context

Border shorthand synthesis previously grouped records by a shared `-width`, `-style`, or `-color` suffix. That lexical shortcut admitted unrelated longhands such as `border-image-width`, obscured Chromium's distinct treatment of omitted logical-side components, and made safe output non-idempotent for default border values.

## Decision

- Classify every supported physical, logical, column-rule, and row-rule longhand through an exhaustive `Width`, `Style`, or `Color` registry.
- Reject similarly suffixed properties from that registry rather than filtering known exceptions after matching.
- Preserve semantic initial values for reparsable output while exposing `initial` for components omitted from Chromium logical-side shorthands.
- Canonicalize observable border components in width, style, color order and apply Chromium's distinct omission rules for grouped versus individual logical-side shorthands.
- Synthesize safe border provenance so reparsing default border states is idempotent.

## Consequences

The pinned Chromium corpus loses 128 getter/cssText mismatches, 104 longhand-state mismatches, and 13 safe-reparse mismatches. Future similarly named properties cannot silently enter border synthesis without an explicit registry change and regression test.
