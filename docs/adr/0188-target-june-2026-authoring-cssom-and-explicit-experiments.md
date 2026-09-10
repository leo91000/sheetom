---
status: accepted
---

# Target June 2026 authoring CSSOM and explicit experiments

The modern CSS parity effort targets the cumulative CSS feature set that reached
Web Platform Baseline Newly available by June 30, 2026, plus native CSS mixins and
custom functions as explicitly documented experimental features. The scope is
authoring CSSOM, including specialized editable rule interfaces; it retains
[ADR 0001](./0001-limit-the-product-to-authoring-cssom.md)'s exclusion of cascade,
computed styles, and rendering, and does not add mixin expansion.

A fixed, pinned feature inventory makes this target reviewable, while explicitly
including mixins and custom functions addresses desired authoring capabilities
that a Baseline-only selection might omit. Choosing Newly available instead of
Widely available avoids adding a 30-month adoption delay. We do not select every
experimental feature or continuously enlarge the target as browsers ship changes.

This is an accepted scope decision, not a claim of achieved conformance.
Experimental revision and exposure policy are settled by
[ADR 0189](./0189-expose-pinned-draft-mixin-interfaces-by-default.md); the accepted
completion criteria are recorded in the
[design interview](../css-june-2026-parity-design.md).
Existing [browser precedence](./0008-use-chromium-as-the-final-divergence-fallback.md)
and [versioned grammar evidence](./0081-bound-modern-value-compatibility-to-a-versioned-corpus.md)
remain in force.

References: [Web Platform Baseline](https://web.dev/baseline),
[CSS Custom Functions and Mixins draft](https://drafts.csswg.org/css-mixins/).
