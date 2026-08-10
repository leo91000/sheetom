---
status: accepted
---

# Own a vendored Rust CSS engine

SheetOM will replace the npm Lightning CSS binding with a repository-owned Rust CSS Engine behind a narrow N-API boundary. Every file tracked by the upstream Lightning CSS repository except its `.git` directory will be imported as ordinary files in one isolated commit, built through local Cargo paths and `[patch]` entries, and then patched in focused commits whenever the pinned browser grammar or process-safety contract requires it; no Git dependency, subtree, submodule, or JavaScript AST roundtrip will remain. Rust owns syntax, declaration state, shorthand expansion and serialization, while JavaScript retains the public WebIDL facade and live rule identity. Vendored MPL files and modifications remain available under MPL-2.0, with their upstream revision and notices preserved.

The native boundary accepts only strings and validated primitives, returns owned DTOs, uses recoverable `Result` errors for expected failures, and is protected by unwind-capable builds plus subprocess crash testing. General Lightning parser corrections should remain separable for upstream contribution, while Chromium precedence, CSSOM ordering and diagnostics remain SheetOM policy.
