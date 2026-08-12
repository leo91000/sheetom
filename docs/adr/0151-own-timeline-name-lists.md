# ADR 0151: Own timeline name lists

- Status: Accepted
- Date: 2026-08-12

## Context

Pinned Chromium defines each `scroll-timeline-name` and `view-timeline-name`
list entry independently as either `none` or a dashed identifier. SheetOM used
the broader dashed-identifier-list representation, which could express one
`none` or a list of names but not mixed or repeated `none` entries. Valid
timeline longhands and shorthands were therefore rejected.

The scroll-timeline shorthand parser also selected the first non-axis token as
its name without proving that the entry contained exactly one name. This made
`none none` an invalid Chromium value but a valid SheetOM mutation.

## Decision

Timeline names have a dedicated semantic list type. Every comma-separated item
is parsed as exactly one `none` keyword or dashed identifier and serialized from
that owned representation. Anchor and scope properties retain their distinct
grammar instead of sharing this timeline-specific type.

Both timeline shorthand codecs are explicit grammar profiles. A scroll
timeline entry must contain exactly one name and at most one axis. Browser
evidence covers mixed lists, repeated `none`, axes, insets, invalid adjacent
names, mutation, removal, atomicity, safe round trips, and crash isolation.

## Consequences

All pinned Chromium timeline list branches survive parsing and remain mutable.
Invalid double names and malformed comma lists remain atomic no-ops. The
runtime model no longer relies on one dashed-identifier enum to approximate
three different CSS grammars.
