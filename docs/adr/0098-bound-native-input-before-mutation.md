---
status: accepted
---

# Bound native input before mutation

The Process Safety Contract applies within high, per-sheet Resource Budgets checked before entering or mutating native state: 64 MiB of stylesheet source, 1 MiB per declaration value, syntax depth 4,096, one million rules, and 100,000 declarations per block by default. Source and declaration sizes are UTF-8 byte counts. Callers may raise these limits explicitly without changing globals; syntax depth has an implementation maximum of 16,384 so explicit parser work stacks and their allocations remain bounded. Exceeding one raises a controlled `RangeError` before mutation rather than truncating, silently rejecting valid CSS, or risking an unbounded native allocation. Parser-only wrapper text is implementation detail and does not consume the caller's budget.
