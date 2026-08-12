# ADR 0168: Close the indexed declaration state

## Status

Accepted for RC6.

## Context

The exhaustive Chromium Webref matrix had reached zero divergence for acceptance, mutation atomicity, and safe reparsing, but fifteen accepted branches still exposed the wrong expanded longhands through `length`, `item()`, and longhand getters. They belonged to six reusable grammar families rather than independent exceptions: SVG marker replication, mask-position axes, table spacing axes, negative animation time assignment, legacy prefixed background-size lists, and cursor hotspots.

Some of these relationships were absent from Lightning CSS's typed shorthand API. Falling back to initial longhand values made a declaration appear accepted while exposing the wrong CSSOM state.

## Decision

- Extend the vendored Lightning CSS typed API for `marker` and `mask-position`, with focused vendor tests suitable for upstreaming.
- Reject negative `animation-duration` values in the typed grammar so a negative shorthand time binds to `animation-delay`.
- Expand `border-spacing` into Chromium's observable horizontal and vertical aliases.
- Reproduce the legacy `-webkit-background-size` rule that duplicates a single component only in the final layer.
- Truncate finite cursor hotspot coordinates toward zero in the browser-facing projection while retaining safe semantic serialization.
- Gate all 8,369 pinned Webref branches with zero indexed-state divergence.

## Consequences

Every Chromium-accepted branch in the pinned corpus now exposes the same declaration count, indexed property names, longhand values, and priorities in SheetOM. The vendored changes remain isolated in their own commit. Browser-facing legacy projections do not weaken parsing, atomicity, or reparsable stylesheet output.
