# ADR 0166: Own radius, outline, and list observability

## Status

Accepted for RC6.

## Context

Three established shorthand families still borrowed either authored shorthand text or semantic default longhands for live CSSOM state.

- `border-radius` retained redundant slash sections and non-canonical math because its complete typed expansion was not reconstructed.
- `outline` exposed omitted components as `currentcolor`, `none`, and `medium` rather than `initial`.
- `list-style` exposed `outside`, `none`, and `disc` for omissions, retained authored component order, and did not preserve Chromium's contextual assignment of the ambiguous `none` keyword.

## Decision

- Reconstruct standard and WebKit border-radius shorthands from their complete typed corner records.
- Classify authored outline components through each exact longhand grammar, expose omissions as `initial`, and synthesize observable components in color-style-width order.
- Classify list position, image, type, and ambiguous `none` occurrences through exact longhand grammars; expose omissions as `initial`; synthesize in position-image-type order.
- Keep safe synthesis based on semantic defaults, independently from the browser-facing omission markers.

## Consequences

Every pinned radius, outline, and list-style getter, declaration-text, and indexed-longhand check now matches Chromium. Acceptance and invalid-neighbor atomicity remain exact, and all safe projections remain idempotent.
