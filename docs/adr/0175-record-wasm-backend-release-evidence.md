# ADR 0175: Record WASM backend release evidence

## Decision

Compatibility Report schema version 7 adds a required `wasmBackend` dimension for RC7 and later. A reusable release-oracle job builds the exact reviewed backend, exercises direct and buffered HTTP initialization, independent instances, main-thread and module-worker execution, Vite, esbuild, Rollup and Webpack consumers in Chromium, Firefox and WebKit, a repeated-instance memory soak, and the Publisher-shaped browser workload. It emits one immutable evidence document whose execution hash and reviewed-contract hash are recorded in the Compatibility Report.

Ordinary CI runs the same contracts. Release recording executes them independently against the release pull request SHA rather than inferring evidence from a green job or copying a moving artifact. The release verifier recomputes the contract hash and rejects missing, partial or over-budget evidence.

## Rationale

Native and WebAssembly adapters share semantic Rust and JavaScript modules but differ in transport, initialization, memory ownership and consumer tooling. Treating a passing Node suite as evidence for both would hide exactly the failures introduced at that seam. One backend evidence module keeps the accepted browsers, bundlers, workload and resource limits local and machine-verifiable.
