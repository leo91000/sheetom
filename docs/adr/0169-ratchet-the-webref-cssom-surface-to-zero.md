# ADR 0169: Ratchet the Webref CSSOM surface to zero

## Status

Accepted for RC6.

## Context

After closing parsing, mutation, safe serialization, and indexed-state differences, 43 of 8,369 pinned Webref branches still differed only in the browser-facing shorthand text. The cases shared canonical synthesis rules: equal-value compression, positional axis reconstruction, normalized typed calculations, canonical component ordering, and optional keyword elision.

Keeping the authored shorthand text in a static provenance group bypassed the longhand codecs that already held the correct Chromium-observable state. Adding isolated string rewrites would duplicate grammar knowledge and fail after longhand mutation.

## Decision

- Classify the remaining shorthand families as synthesized observable provenance.
- Reconstruct their getters and declaration text from the same typed longhand records used by mutation and indexed access.
- Generalize position-list synthesis across background and mask axes.
- Use closed structural cardinality codecs for two-value and four-side families.
- Own `font-synthesis` component order explicitly from its three longhands.
- Ratchet every dimension of the pinned 8,369-branch Webref corpus to zero mismatches.

## Consequences

SheetOM and pinned Chromium now agree on acceptance, getter values, declaration text, indexed declaration state, invalid-mutation atomicity, and safe reparsing for every generated Webref branch. Future changes that create even one mismatch fail CI until backed by a reviewed browser-baseline update or a corrective implementation.
