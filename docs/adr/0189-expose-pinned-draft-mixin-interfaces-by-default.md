---
status: accepted
---

# Expose pinned draft mixin interfaces by default

Native CSS mixin authoring follows an exact revision of the current CSSWG draft,
including its specialized rule interfaces, and is available by default without
an experimental runtime switch. Chromium's experimental interfaces differ from
the draft; implementing those instead would expose a different public contract.
We choose the draft contract, label it experimental, and use browser results as
supporting evidence with explicit discrepancy records. This narrows
[ADR 0043](./0043-complete-mutable-rule-authoring-before-zero-one.md)'s generic-rule
policy for the explicitly selected mixin family; other future rules retain the
existing policy. Stable CSS continues to follow
[ADR 0008](./0008-use-chromium-as-the-final-divergence-fallback.md).

Existing custom functions remain available by default and are audited under
their existing compatibility policy. This decision does not silently replace
already published function behavior with a draft-only contract.

The source selected for the mixin contract is
[`css-mixins-1/Overview.bs` at CSSWG revision
`5e68d5c1ca6656dd7a4b32f821d187aa9652ad5d`](https://github.com/w3c/csswg-drafts/blob/5e68d5c1ca6656dd7a4b32f821d187aa9652ad5d/css-mixins-1/Overview.bs),
SHA-256 `4967c197dfd1b86cb321b835ebcdb2577974bc80d7097294294cd75a8249a76c`.
Implementation must record any internally inconsistent or underspecified draft
behavior as an explicit resolution, not claim browser consensus where none was
measured. Updating the draft contract requires a reviewed change and renewed
evidence.
