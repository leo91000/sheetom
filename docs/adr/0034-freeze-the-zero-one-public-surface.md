---
status: superseded by ADR-0043
---

# Freeze the 0.1 public surface

Version 0.1 will expose browser-shaped `CSSStyleSheet`, `CSSRuleList`, `CSSRule`, `CSSStyleRule`, and `CSSStyleDeclaration` interfaces with live identity, standard mutation methods, indexed and named declaration access, and Generic Rules. Its only initial non-browser extensions are `parseStyleSheet`, safe `serialize`, and pull-based `takeDiagnostics`; additional conveniences and specialized rule interfaces wait for later releases.
