# @sheetom/wasm

## 0.1.0-rc.11

### Patch Changes

- 9791b89: Preserve authored pending shorthands after longhand mutations, add resilient best-effort serialization diagnostics for states CSS text cannot represent exactly, and expose `serializeStrict()` for exact-only callers.

## 0.1.0-rc.10

### Patch Changes

- 9d92e23: Add ordered declaration mutation batches, bounded shared parse reuse, and lower-peak whole-sheet serialization without changing CSSOM validation semantics.

## 0.1.0-rc.9

### Patch Changes

- aa68057: Use a measured faster Binaryen profile while retaining the established runtime code generation and the complete compatibility, packaging, memory, size, and performance gates.

## 0.1.0-rc.8

## 0.1.0-rc.7

### Patch Changes

- d970977: Add the explicit, ESM-only `@sheetom/wasm` backend with the same private Engine Binding, parser, resource limits, and browser-shaped facade as the native package.
- 1d8a026: Make the WebAssembly module graph statically analyzable while preserving independent backend instances, and gate major browser bundlers, memory, and Publisher-shaped performance.
- e92635e: Publish the root, WebAssembly, and thirteen native implementation packages from one verified lockstep artifact set.
- 8aee179: Record and verify WebAssembly browser, bundler, memory, and performance evidence in every RC7-or-later Compatibility Report.
