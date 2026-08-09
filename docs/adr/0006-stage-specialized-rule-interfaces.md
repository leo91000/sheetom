---
status: superseded by ADR-0043
---

# Stage specialized rule interfaces

The first complete surface will implement `CSSStyleSheet`, `CSSRuleList`, `CSSStyleRule`, `CSSStyleDeclaration`, `insertRule`, and `deleteRule`. Other parsed rules will remain serializable and accessible as Generic Rules until specialized browser-compatible interfaces are added, preventing incomplete API coverage from discarding valid stylesheet content.
