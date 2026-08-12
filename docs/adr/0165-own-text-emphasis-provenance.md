# ADR 0165: Own text emphasis provenance

## Status

Accepted for RC6.

## Context

The typed text-emphasis AST stores semantic values but erases whether an author explicitly supplied default keywords. Chromium's CSSOM retains explicit `filled` in `text-emphasis-style` and explicit `right` in `text-emphasis-position`, while canonicalizing their order.

The `text-emphasis` shorthand also exposes omitted longhands as `initial` in its indexed declaration state. Expanding directly from semantic defaults instead exposed `none` and `currentcolor`, and generic shorthand serialization could retain authored component order.

## Decision

- Derive style and position observability from the recovered component structure, retaining explicit default keywords in canonical order.
- Partition shorthand components through the typed color grammar, then expose omitted style or color longhands as `initial` without changing their safe semantic values.
- Synthesize the live shorthand in canonical style-then-color order from those observable longhands.
- Keep the proof corpus outside runtime and gate standard plus prefixed aliases through the pinned Chromium Webref observations.

## Consequences

Every pinned `text-emphasis`, `text-emphasis-style`, and `text-emphasis-position` getter, declaration-text, and indexed-longhand check now matches Chromium, including their WebKit aliases. Acceptance, invalid-neighbor atomicity, and idempotent safe serialization remain unchanged.
