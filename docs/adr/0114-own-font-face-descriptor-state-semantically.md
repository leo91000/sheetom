---
status: accepted
---

# Own font-face descriptor state semantically

Every accepted ordinary `@font-face` descriptor is retained as an owned Semantic Declaration backed by a typed vendored AST or a dedicated SheetOM descriptor value. Canonical stylesheet text and browser-observable CSSOM text are derived projections of that state, while recovered component values retain only the provenance required for fault recovery. Generic textual declaration storage cannot represent either style declarations or font-face descriptors; CSS-wide keywords, pending substitutions, and custom token streams keep their explicit semantic representations.
