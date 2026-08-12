# ADR 0148: Own font palette mixing

- Status: Accepted
- Date: 2026-08-12

## Context

The vendored Lightning CSS parser modeled `font-palette` as only a dashed
identifier. It therefore rejected the `light` and `dark` keywords and the
recursive `palette-mix()` grammar accepted by Chromium and CSS Fonts 4.

## Decision

Lightning CSS now owns a recursive semantic `FontPalette` AST. The parser
requires an explicit color interpolation method, canonicalizes the `xyz` alias
to `xyz-d65`, permits Chromium's shorter, longer, increasing, and decreasing
hue interpolation only for polar spaces, accepts exactly two palette operands, supports a percentage
before or after either operand, and recursively accepts palette keywords,
dashed identifiers, and nested mixes.

Direct percentages are range checked. Two direct zero weights are invalid;
complementary weights are canonicalized like Chromium. Calculated percentages
remain deferred, including values whose eventual result may be outside the
direct range. SheetOM removes its former `normal` keyword fallback so the
vendored AST is the sole static grammar authority.

The capability corpus covers every Chromium interpolation color-space keyword.
It rejects custom interpolation spaces and `specified hue`, which are not
accepted by the pinned Chromium oracle even though Lightning CSS exposes the
latter in a more general color-interpolation enum.

## Consequences

Font palette values retain semantic state, Chromium CSSOM canonicalization,
atomic invalid replacement, safe round trips, and subprocess crash isolation.
The vendored parser change is isolated for a possible upstream contribution.
