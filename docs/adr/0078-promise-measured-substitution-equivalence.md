---
status: accepted
---

# Promise measured substitution equivalence

When Reparsable Stylesheet Serialization closes malformed custom-property token structures, SheetOM promises only the substitution behavior measured for named consuming properties in the release Compatibility Baseline, not identical computed custom-property text. Exact computed values remain beyond the Authoring CSSOM boundary, while the narrower measured promise preserves a useful browser-backed contract without claiming that every repaired token stream is universally equivalent.
