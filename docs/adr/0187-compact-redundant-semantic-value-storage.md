---
status: accepted
---

# Compact redundant semantic value storage

## Decision

After observable and reparsable projections have been derived, a custom-token
Semantic Property Value discards its Recovered Component Value Tree when every
token and block closure was explicit. The declaration retains its original
source, semantic category, parse kind, and both public projections. Values that
need implicit-closure or parse-error evidence keep the complete recovered tree.

When a declaration's canonical projection is byte-identical to its recovered
source, both fields share the same immutable `Arc<str>`. A distinct canonical
allocation remains authoritative whenever normalization or recovery changes the
text. This refines ADR 0106 without changing either serialization contract.

## Rationale

The component tree and canonical cache duplicated the largest strings in
explicit custom properties even though later operations only needed the source
and derived projections. Keeping recovery evidence only when it can affect
behavior preserves the semantic-state seam while removing redundant retained
storage.

In seven isolated Linux x64 native runs of the 12,000-rule large-serialization
workload, median peak RSS fell from 315,543,552 to 265,666,560 bytes. The output
remained byte-identical at 24,876,890 bytes.

Seven isolated Node WebAssembly runs of the same workload reduced median RSS
after state construction from 324,599,808 to 276,049,920 bytes and median peak
RSS after serialization from 353,361,920 to 304,533,504 bytes.

A clean artifact comparison increased the native addon by 4,096 bytes and the
optimized WebAssembly engine by 357 raw bytes and 62 gzip bytes. Five interleaved
native samples put the small stress-workload mutation median about 10% slower,
while Publisher mutation was about 13% faster; the repository's official
performance comparator passed. The size and microbenchmark tradeoffs are
accepted for the substantially lower peak-memory requirement.

## Consequences

- Browser-facing CSSOM and Reparsable Stylesheet Serialization remain unchanged.
- Explicit custom values no longer expose a retained component tree internally.
- Malformed, implicitly closed, pending-substitution, and shorthand recovery
  paths keep their structural evidence.
- Storage sharing stays local to one declaration; this does not introduce a
  global string interner or unbounded cross-sheet retention.
