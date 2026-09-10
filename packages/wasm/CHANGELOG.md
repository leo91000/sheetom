# @sheetom/wasm

## 0.2.0

### Minor Changes

- bde80b7: Add revision-pinned CSS mixin authoring interfaces, CSS.escape(), CSS.supports(),
  and the StyleSheet base interface. Support linear() easing and gradient color
  interpolation across property contexts; correct selector CSSOM serialization
  and namespace-aware mutation. Gate the June 2026 authoring target on native and
  WASM backends, including deep-input and invalid-mutation checks.
  
  Validate attribute namespaces and recover forgiving selector lists correctly,
  retain namespace state after detachment, and preserve inactive animation
  settings through serialization. Compare rule and declaration state on reparse.
- 5ecb1a2: Support sibling-count(), sibling-index(), and progress() in typed CSS math,
  preserving calculations that depend on element or layout context. Report
  support for the seven Baseline media state selectors. Expose
  CSSPositionTryDescriptors and enforce @position-try descriptor restrictions
  across parsing, replacement, and mutation on both native and WASM backends.
  
  Advance the dated CSS authoring target to September 9, 2026 and include
  individually Baseline compatibility branches of partially available families.

## 0.1.1

### Patch Changes

- f744336: Reduce CSS parsing overhead and the WebAssembly download size by sharing calc serialization and removing redundant allocation and reparsing paths.
- 10896ad: Reduce peak memory for large stylesheets by compacting explicit custom-property recovery state and sharing identical semantic text storage.

## 0.1.0

### Patch Changes

- d970977: Add the explicit, ESM-only `@sheetom/wasm` backend with the same private Engine Binding, parser, resource limits, and browser-shaped facade as the native package.
- 9d92e23: Add ordered declaration mutation batches, bounded shared parse reuse, and lower-peak whole-sheet serialization without changing CSSOM validation semantics.
- 1d8a026: Make the WebAssembly module graph statically analyzable while preserving independent backend instances, and gate major browser bundlers, memory, and Publisher-shaped performance.
- aa68057: Use a measured faster Binaryen profile while retaining the established runtime code generation and the complete compatibility, packaging, memory, size, and performance gates.
- 9791b89: Preserve authored pending shorthands after longhand mutations, add resilient best-effort serialization diagnostics for states CSS text cannot represent exactly, and expose `serializeStrict()` for exact-only callers.
- e92635e: Publish the root, WebAssembly, and thirteen native implementation packages from one verified lockstep artifact set.
- 8aee179: Record and verify WebAssembly browser, bundler, memory, and performance evidence in every RC7-or-later Compatibility Report.

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
