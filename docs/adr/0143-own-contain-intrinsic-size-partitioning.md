# ADR 0143: Own `contain-intrinsic-size` partitioning

- Status: Accepted
- Date: 2026-08-12

## Context

Each axis of `contain-intrinsic-size` accepts either `none`, a length, or the
compound `auto none` / `auto <length>` form. A whitespace-token cardinality
codec cannot distinguish one compound axis value from two axis values and
therefore rejected Chromium-valid values such as `auto none` and
`auto none auto 1px`.

## Decision

SheetOM parses the shorthand as one or two complete longhand values:

1. validate the complete source as one `contain-intrinsic-width` value and
   duplicate it when valid;
2. otherwise test top-level component boundaries for one complete width value
   followed by one complete height value;
3. require complete typed validation on both sides and reject if no partition
   exists.

The algorithm is bounded to four top-level components, preserves calculated
range deferral, and never uses token prefixes as a validity decision.

## Consequences

Compound axis values, mixed simple/compound axes, longhand mutation, pending
substitutions, and invalid neighboring partitions are covered by Chromium
differentials and public state tests. The generic cardinality codec remains
small and does not acquire property-specific ambiguity.
