# ADR 0163: Own animation range observability

## Status

Accepted for RC6.

## Context

`animation-range` parsed and expanded atomically, but the vendored Lightning CSS property API did not expose its two parallel longhand lists through `Property::longhand`. SheetOM therefore retained authored shorthand text instead of Chromium's canonical getter and declaration text. The same gap made the indexed longhand view incomplete.

Lightning CSS also parsed an explicit shorthand end with the start grammar. A bare named range such as the `cover` in `normal cover` consequently received the start default offset of zero instead of the end default offset of one hundred percent.

## Decision

- Expose `animation-range-start` and `animation-range-end` as parallel lists from the vendored typed shorthand AST.
- Parse an explicit shorthand end with `AnimationRangeEnd`, preserving its end-specific default offset.
- Synthesize the observable and safe shorthand from the two canonical longhand lists, omitting an end only when the start grammar implies the same value.
- Keep both vendored corrections and their focused tests in a dedicated commit suitable for upstreaming.

## Consequences

All pinned `animation-range` getter, declaration-text, indexed-longhand, atomicity, and safe-reparse checks now match Chromium. The Webref corpus remains acceptance-exact and reparse-idempotent, while its mismatch ratchet drops by thirty cases.
