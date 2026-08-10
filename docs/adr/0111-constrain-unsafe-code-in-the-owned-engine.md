---
status: accepted
---

# Constrain unsafe code in the owned engine

SheetOM-owned Rust crates forbid unsafe code. Existing upstream unsafe code may remain unchanged in Vendored CSS Syntax Source and Vendored Lightning Source, but any local edit touching an unsafe block requires an isolated commit, an explicit safety rationale, targeted regression tests and coverage-guided fuzzing before merge. Rewriting upstream unsafe code merely to remove the keyword is rejected because it would enlarge the fork and replace mature invariants without compatibility evidence.
