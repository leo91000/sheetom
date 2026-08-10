---
status: accepted
---

# Bound native input before mutation

The Process Safety Contract applies within high, per-sheet Resource Budgets checked before entering or mutating native state: 64 MiB of stylesheet source, 1 MiB per declaration value, syntax depth 4,096, one million rules, and 100,000 declarations per block by default. Callers may raise these limits explicitly without changing globals; exceeding one raises a controlled `RangeError` before mutation rather than truncating, silently rejecting valid CSS, or risking an unbounded native allocation.
