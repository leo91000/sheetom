# ADR 0167: Own scalar and composite CSSOM projections

## Status

Accepted for RC6.

## Context

Several typed values were semantically correct but used a general-purpose printer rather than Chromium's CSSOM projection rules. The remaining cases formed reusable grammar categories: number-percentage zero aliases, equal optional pairs, zero-valued optional clip margins, zero-angle font styles, and color-first text shadows.

Applying the same rewrite to safe stylesheet output would be incorrect for browser serializations that are not round-trip preserving, so the projection remains browser-facing only.

## Decision

- Serialize zero percentages as unitless zero only when the exact property grammar accepts both numbers and percentages.
- Compress equal pairs for the closed `view-timeline-inset` and `scroll-snap-align` grammars.
- Omit a zero clip-margin length only when paired with its visual-box component.
- Canonicalize an explicit zero-angle oblique font style to `normal`.
- Parse text-shadow layers structurally, project colors and lengths through their typed grammars, and serialize color before geometry.

## Consequences

Every pinned case in these scalar and composite families now matches Chromium in getters, declaration text, and indexed state. Length percentages outside number-percentage grammars retain their units, and safe output remains idempotent.
