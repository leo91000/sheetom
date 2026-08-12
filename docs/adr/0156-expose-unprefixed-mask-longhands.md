# ADR 0156: Expose unprefixed longhands from prefixed shorthands

## Status

Accepted for RC6.

## Context

Lightning CSS compared every shorthand vendor prefix with the prefix bitset of the requested longhand. Unprefixed longhands such as `mask-mode` and `mask-composite` intentionally have an empty prefix bitset, so the comparison rejected them even for the standard `mask` shorthand. SheetOM then replaced their authored values with defaults. This affected both `mask` and Chromium's canonical CSSOM state for `-webkit-mask`.

## Decision

- Apply a shorthand prefix compatibility check only when the requested longhand carries a vendor-prefix dimension.
- Use prefix intersection rather than exact bitset equality so a shorthand can expose a compatible member from a multi-prefix property id.
- Retain direct vendored-engine tests for `mask-mode` and `mask-composite`, plus SheetOM tests for both standard and prefixed shorthand entry points.
- Keep the Chromium/Webref acceptance and atomicity ratchets at zero while lowering the observable mismatch ratchet.

## Consequences

Mask mode and composite values survive typed shorthand expansion and participate in longhand mutation, shorthand synthesis and safe serialization. The generic fix applies to any future mixed-prefix shorthand without adding property-specific extraction code.
