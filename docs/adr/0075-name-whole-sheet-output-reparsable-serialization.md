---
status: accepted
---

# Name whole-sheet output reparsable serialization

The public term for `CSSStyleSheet.serialize()` output is Reparsable Stylesheet Serialization, not safe serialization. The output repairs recoverable syntax and confines declarations and rules so the result reparses, but it does not sanitize URLs, imports, remote resources, or other valid CSS; this refines the terminology of ADR-0007 without changing its separation between browser-facing `cssText` and whole-sheet output.
