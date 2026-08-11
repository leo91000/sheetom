# ADR 0121: Own unitless numbers and cross-dimension values

## Status

Accepted for RC6.

## Context

Lightning CSS accepted every unitless number as a length, so declarations such as `width: 1` survived even though Chromium rejects them in standards mode. Globally rejecting nonzero unitless lengths fixed that false-positive family, but it also exposed legitimate property grammars where a number is distinct from a length: SVG geometry, opacity, integer properties, `line-height`, image-slice dimensions, and the legacy `-webkit-perspective` alias.

These properties also differ in range, percentage support, cardinality, unit preservation, and browser-facing serialization. Treating all of them as one `length | number` parser would either restore invalid declarations or erase observable distinctions such as `stroke-width: 0` versus `stroke-width: 0px`.

## Decision

Vendored Lightning CSS accepts a unitless value as `Length` only when it is zero. The change stays in an isolated commit with its upstream-style regression test.

SheetOM owns the intentional exceptions through typed Rust profiles:

- number and percentage properties declare their range, integer constraint, percentage support, and percentage-to-number serialization;
- length/percentage/number properties declare whether negative values and percentages are allowed and whether a unitless zero is exposed as `0` or `0px`;
- `stroke-dasharray` owns comma/space list parsing and preserves math-function provenance per item;
- image width and outset values own one-to-four component cardinality, with `auto` limited to width;
- `-webkit-perspective` has a dedicated legacy codec, so unitless positive numbers become pixels without weakening the unprefixed `perspective` grammar.

Numeric-looking input is authoritative for these profiles. An invalid numeric candidate cannot fall back to Lightning CSS after the owned grammar rejects it. Non-numeric branches remain available to the standard parser.

The checked-in Numeric Property contract selects 57 properties and 11 grammar-oriented probes from the full Chromium Property Value evidence. The public binding must match Chromium for acceptance, observable value, `cssText`, indexed names, and invalid-mutation atomicity across the complete selected cross product. The evidence files are test-only and are not imported by the runtime.

## Consequences

- Standard length properties no longer accept nonzero unitless numbers accidentally.
- Intentional number-capable properties remain accepted with property-specific range and serialization behavior.
- Safe serialization and CSSOM-observable serialization share semantic state without conflating units or math provenance.
- Future numeric properties must join a reviewed profile and pass the Chromium contract rather than inheriting a permissive global fallback.
