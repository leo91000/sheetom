# ADR 0174: Generate static instance-scoped WASM factories

## Decision

SheetOM transforms the exact pinned `wasm-bindgen` Web output into an instance-scoped Engine Binding factory and transforms the shared bundled CSSOM facade into an Engine Binding facade factory. Both transforms validate structural sentinels and fail closed when their generated inputs drift. The published `@sheetom/wasm` JavaScript graph contains only static imports; it does not select unique module instances through query-string dynamic imports, mutate `globalThis`, evaluate generated code, or share mutable WASM state between independent facades.

The factory interface is the test seam. Direct HTTP, Vite, esbuild, Rollup and Webpack consumers exercise the same published package on a main thread and module worker in pinned Chromium, Firefox and WebKit. A repeated independent-instance memory soak and an absolute Publisher-shaped browser workload block publication alongside the functional matrix.

## Rationale

Query-parameter dynamic imports preserved per-instance wasm-bindgen state but hid the generated module graph from static bundler analysis. Keeping a singleton glue module would make explicit `createSheetOM(source)` calls share the wrong WASM instance and class identity. Generating static factories retains isolation while making bundler discovery deterministic and concentrates generated-source assumptions in two small, tested transforms.
