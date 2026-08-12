# ADR 0153: Own current keyword and ordered-set grammars

## Status

Accepted for RC6.

## Context

The pinned Chromium/Webref cross-product exposed several standard grammar branches that the vendored Lightning CSS revision either rejected or serialized differently: `revert-rule`, math display, `auto-phrase`, pixel-preserving image rendering, `preserve-3d`, a standalone dense grid flow, ordered scroll-marker purposes, reordered scrollbar-gutter components, and legacy break aliases.

Accepting only the observed literals would leave adjacent invalid values unchecked and would not reproduce legacy CSSOM alias projection. In particular, `page-break-before: always` stores `break-before: page` while its legacy getter returns `always`; `-webkit-column-break-before: always` stores `break-before: column` and remains write-only.

## Decision

- Patch the vendored typed AST for standard keyword branches and `grid-auto-flow` rather than adding runtime allow-list exceptions.
- Require `all` to consume the complete declaration value before accepting a CSS-wide keyword.
- Reserve `revert-rule` everywhere that consumes a CSS-wide keyword or `<custom-ident>`.
- Own `scroll-marker-group` and `scrollbar-gutter` as complete token grammars, including ordering, duplicate rejection, and canonical serialization.
- Translate legacy break inputs at the alias boundary and project their getters from canonical semantic state.
- Keep accepted branches and invalid neighbors in the browser differential contracts and Webref ratchet.

## Consequences

The runtime accepts the complete reviewed branches without weakening neighboring grammar, legacy aliases remain observable like Chromium, and the vendored changes stay isolated for possible upstreaming. Future keywords must extend the typed grammar and differential evidence together.
