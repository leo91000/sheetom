# ADR 0149: Own rule partition grammars

- Status: Accepted
- Date: 2026-08-12

## Context

Chromium accepts `intersection` for the row and column rule-break longhands and
accepts `between` and `around` for the corresponding visibility longhands.
SheetOM's reviewed browser grammar omitted those keywords. Its structural
shorthand fallback also implemented both shorthands as two-value grammars and
retained the obsolete `spanning-item` and `none` alternatives. Chromium instead
accepts exactly one keyword and repeats it across the row and column longhands.

## Decision

The browser-longhand registry owns the complete pinned Chromium keyword sets.
`rule-break` and `rule-visibility-items` accept exactly one component and expand
it atomically to both observed longhands. Multiple components and obsolete
neighbors are rejected without changing existing state.

The versioned evidence covers every longhand keyword, shorthand expansion,
invalid atomic replacement, longhand mutation and removal, item ordering, and a
safe parse-serialize-parse round trip.

## Consequences

The runtime no longer drops valid rule-partition declarations or accepts
obsolete alternatives. The fix reduces the Webref acceptance ratchet through a
shared grammar rather than exact-value exceptions.
