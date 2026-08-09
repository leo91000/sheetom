---
status: superseded
superseded-by: 0093
---

# Pin the Syntax Engine Set

SheetOM will pin the exact runtime versions of Lightning CSS, css-tree, cssstyle, and the csstools tokenizer, record all four in every Compatibility Baseline, and treat consumer overrides as outside that baseline. Future parser versions remain an informative compatibility signal until SheetOM deliberately upgrades and releases a newly measured set; this chooses reproducible observable behavior over package-manager deduplication and supersedes ADR 0015.
