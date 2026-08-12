# ADR 0145: Own `text-box` component partitioning

- Status: Accepted
- Date: 2026-08-12

## Context

The `text-box` shorthand combines `text-box-trim` and `text-box-edge` with
either component omitted and with the two component groups in either order.
The previous codec assumed a leading trim followed by a mandatory edge. It
therefore rejected Chromium-valid `trim-start`, `auto`, `auto none`, and
`cap alphabetic trim-end` values.

## Decision

SheetOM partitions `text-box` through its complete typed longhand grammars:

1. test the complete source as a trim-only value;
2. test the complete source as an edge-only value;
3. otherwise test the first and last top-level component as the trim while
   requiring the remaining contiguous components to be one complete edge.

Omitted trim and edge values become `trim-both` and `auto` respectively.
Every candidate must consume the complete corresponding longhand grammar, so
interleaved edge tokens, duplicate trims, and multiple edges remain atomic
no-ops.

## Consequences

The shorthand now covers omitted, reversed, paired-edge, normal-component,
longhand mutation, pending-substitution, and whole-sheet round-trip branches.
The parser remains bounded to three top-level components and does not duplicate
the longhand keyword grammar.
