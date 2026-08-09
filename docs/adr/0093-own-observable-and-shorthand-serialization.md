---
status: accepted
---

# Own observable and shorthand serialization

SheetOM will not depend on `cssstyle`. The Observable Value Codec owns the
property-aware text exposed through CSSOM getters, and every manifested static
shorthand maps to an explicit codec that expands into canonical longhand state.
Lightning CSS, css-tree, and the csstools tokenizer remain the pinned syntax
engine set. Compatibility Baselines record only those runtime engines from
this decision onward, while older immutable baselines may retain their
historical `cssstyle` version. This supersedes ADR 0077 and narrows the helper
allowances in ADRs 0079 and 0088 to zero.
