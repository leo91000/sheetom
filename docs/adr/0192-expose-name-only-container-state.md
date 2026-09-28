---
status: accepted
---

# Expose name-only container state

WebDX 3.40.0 adds the individually Baseline optional container-query branch.
The owned parser already accepts a container name without a query, but the core
prelude adapter rejected the empty query and caused both facades to fall back to
incorrect public fields. Accept an empty query after a validated name and use a
CSS identifier token boundary to separate it from an optional condition. Keep
escaped whitespace inside the name and the `not` operator inside the query.
Grammar validation remains in the owned parser; no DOM query is evaluated.

Include containerName and containerQuery in shared authoring snapshots. Native,
WASM and the existing pinned Chromium must agree on these fields, nested rule
state and serialization/reparse. Parsing arbitrary tokens or observing only the
conditionText string is insufficient evidence for the CSSContainerRule API.
