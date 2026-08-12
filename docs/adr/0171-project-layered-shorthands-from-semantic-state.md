---
status: accepted
---

# Project layered shorthands from semantic state

Expanded `background`, `mask`, and `-webkit-mask` longhands derive from the typed layer state. Their observable position and size views use the same values, while the Recovered Component Value Tree records only whether those components were explicitly authored. A shared layered shorthand projection owns that boundary; it does not split or reparse source or serialized strings, and it keeps Vendored Lightning Source unchanged. This accepts a focused internal refactor so compact separators, multi-layer values, and future lexical spellings cannot silently replace valid semantic components with defaults.

The refactor is limited to the shared layered-shorthand profile. Other observable codecs remain separate until browser evidence justifies a family-specific change, and an architectural test prevents the layered codec from acquiring a textual parsing path.

The projection exists only while expanding a shorthand mutation. It emits complete semantic longhand records and disposable position and size views, then is discarded; the expanded longhands remain the only persistent shorthand authority. When the recovered tree proves that an authored observable spelling cannot be reproduced by the semantic printer, such as `url()` inside `image-set()`, that evidence is attached to the affected longhand rather than to parallel shorthand state. Subsequent longhand mutations therefore require no layered state to invalidate and do not erase that CSSOM spelling.
