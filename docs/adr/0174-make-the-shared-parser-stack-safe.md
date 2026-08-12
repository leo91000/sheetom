---
status: accepted
---

# Make the shared parser stack-safe

The Rust CSS Engine will replace recursive rule and value traversal with explicit owned work stacks, patching its vendored syntax sources where necessary, so native and `wasm32-unknown-unknown` backends execute one parser through the same default depth-4,096 Resource Budget. The native large-stack parser thread is removed only after both backends pass the complete deep-input, crash-safety, fuzz and compatibility suites. SheetOM will not preserve native recursion by introducing a reduced WASM grammar, a smaller silent budget, a mandatory worker, or an experimental nightly unwind toolchain.
