# ADR 0158: Separate mask longhand observability

## Status

Accepted for RC6.

## Context

Chromium exposes an omitted image from a static `mask` expansion as `initial`, while the mask shorthand grammar uses `none` as the semantic initial image. Chromium also repeats the initial `0%` position for every comma-separated mask layer. Treating those observable spellings as the shorthand's semantic values either lost longhand fidelity or polluted the reconstructed shorthand.

## Decision

- Derive observable `mask-image` and prefixed position-axis lists independently for every authored layer.
- Expose an omitted image as `initial` and an omitted position axis as `0%` in longhand getters and indexed declaration state.
- Translate observable `initial` back to semantic `none` only inside mask shorthand synthesis.
- Preserve `no-clip` without redundantly emitting the initial `border-box` origin.
- Gate both standard and prefixed mask forms against the complete pinned Webref cross-product.

## Consequences

The mask families no longer account for any expanded-longhand mismatches in the pinned Chromium corpus. Observable longhand fidelity and shorthand reconstruction remain separate responsibilities, preventing one representation from corrupting the other.
