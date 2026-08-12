---
status: accepted
---

# Require an engine ABI handshake

Every N-API and WebAssembly engine exports a generated Engine ABI Identity containing its binding ABI version, SheetOM version and Syntax Engine Set hash. The JavaScript facade compares that identity before constructing public state and rejects any mismatch with a stable binding error, even when exact optional dependencies should normally prevent it. This makes package-manager overrides, corrupt caches, copied binaries and partial multi-package releases fail closed rather than invoking an incompatible native layout.
