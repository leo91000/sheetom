---
status: accepted
---

# Promote RC11 without a calendar soak

## Decision

The first stable release will promote the exact RC11 source line without adding
runtime behavior. The complete release pull request validation is sufficient
for publication; `0.1.0` no longer requires seven consecutive scheduled runs or
any other calendar-duration soak.

Stable promotion still requires the immutable compatibility report, the full
platform and runtime package matrix, native and WebAssembly browser evidence,
fuzzing, process-safety checks, performance gates, and exact artifact integrity.
Any release pull request update still requires the complete matrix to pass again
on its new final commit.

## Rationale

RC10 and RC11 contain the latest reviewed fixes, and their published artifacts
have received sufficient maintainer testing for stable promotion. A mandatory
elapsed-time delay adds no necessary evidence for this release beyond the full
technical validation already enforced by CI.

This supersedes ADR 0170 and ADR 0183, and supersedes only the seven-night
first-stable requirement in ADR 0173.
