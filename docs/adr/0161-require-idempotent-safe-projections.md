# ADR 0161: Require idempotent safe projections

## Status

Accepted for RC6.

## Context

SheetOM exposes Chromium-compatible live declaration text separately from the safe stylesheet projection returned by `serialize()`. Four accepted Webref branches still produced safe CSS that changed after one parse and serialization cycle: zero-length perspective aliases, distinct `place-self` values that the generic shorthand compressor collapsed, the legacy background-size alias, and `-webkit-text-stroke` values that borrowed border defaults.

The text-stroke path also classified longhands by string suffix. That let the implementation infer CSS semantics from property spelling instead of the closed browser-derived property registry.

## Decision

- Require `serialize(parse(serialize(state)))` to equal `serialize(state)` for every accepted property branch in the pinned Chromium corpus.
- Preserve a unit on zero perspective lengths because a unitless zero is not valid in the perspective grammar.
- Keep both `place-self` components in the safe projection when omission would change the second component from `auto` to the first value.
- Separate the legacy background-size observable alias spelling from its canonical safe projection.
- Give `-webkit-text-stroke` a dedicated width/color codec with authored-component tracking. Omitted longhands are observable as `initial`, and synthesis always orders width before color.
- Use exhaustive property mappings wherever a property name selects a grammar or semantic component. Suffix checks remain permitted only for lexical output handling or after membership in a closed registry has already been established.

## Consequences

All 8,369 pinned branches now have an idempotent safe projection. Acceptance and invalid-neighbor atomicity remain exact, while text-stroke getter, declaration text, and indexed longhands move closer to Chromium. Future reparse regressions fail the checked-in Webref ratchet rather than accumulating as documented exceptions.
