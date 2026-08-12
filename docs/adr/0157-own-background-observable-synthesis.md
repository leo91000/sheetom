# ADR 0157: Own background observable synthesis

## Status

Accepted for RC6.

## Context

The typed background expansion already retained every Chromium longhand, but its observable codec collapsed equal origin and clip boxes, truncated two-component background sizes, and canonicalized comma-separated longhands as one declaration. That lost authored URL-function form inside `image-set()` and prevented per-layer CSS math canonicalization.

These were related projection errors: a complete semantic longhand state was being synthesized through a serializer with different shorthand omission rules.

## Decision

- Synthesize both background origin and clip whenever either box participates in the shorthand, including equal values.
- Parse background size as the longest valid one- or two-component value after `/`, rather than selecting one token.
- Project every background layer independently before joining its longhand list, so CSS-wide placeholders do not prevent typed canonicalization of neighboring layers.
- Keep the second whole-list projection out of the shorthand override because it erases per-layer provenance such as `url()` within `image-set()`.
- Differentially gate the observable shorthand, every longhand item, invalid-neighbor atomicity and safe round trip.

## Consequences

All 79 remaining background getter/cssText mismatches and all 10 background longhand mismatches in the pinned Webref cross-product are closed. Multi-layer backgrounds retain complete box, size, image and math semantics without storing an independent shorthand declaration.
