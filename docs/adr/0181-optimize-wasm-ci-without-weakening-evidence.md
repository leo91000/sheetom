---
status: accepted
---

# ADR 0181: Optimize WASM CI without weakening evidence

## Decision

SheetOM will compile the complete WebAssembly dependency graph through a dedicated
`wasm-release` Cargo profile that inherits release LTO, symbol stripping, one
codegen unit and unwind safety while selecting `opt-level = "z"`. Pinned Binaryen
then runs the lightweight `wasm-opt -O1` profile. The build fails unless Binaryen
removes at least three percent and the final engine remains at or below 5,000,000
raw bytes and 850,000 gzip-9 bytes. Those absolute budgets make the complete
artifact smaller than the former `-Oz` output even though the local Binaryen pass
has a lower relative threshold.

The exact Binaryen version and feature flags and the complete native, browser,
bundler, memory, Publisher-performance, packaging, and release-evidence gates
remain unchanged.

The `wasm-quality` job will restore a content-addressed Playwright browser cache
keyed by the operating system and exact lockfile. It will continue to run
`playwright install --with-deps` so a cache hit skips only browser downloads and
never skips operating-system dependency validation.

Host-target Rust workspace tests remain owned by `native-quality`, whose
`native:core-check` runs `cargo test --workspace`. `wasm-quality` retains the
WASM-target Clippy build and every executable WebAssembly contract, but no longer
recompiles the zero-test `sheetom-wasm` host harness serially.

## Evidence

Four full GitHub Actions runs put `wasm-quality` between 5m49s and 6m08s. The
dominant step was `wasm:build` at 153-174 seconds. Its Rust release compilation
took only 6.7 seconds with 214 sccache hits and zero misses; `wasm-opt -Oz`
consumed the remaining approximately 155 seconds.

The first profile-only comparison used the current engine and identical source,
toolchain, LTO, symbol-stripping and unwind settings:

| Pipeline stage | Time | Raw bytes | gzip -9 bytes |
| --- | ---: | ---: | ---: |
| Existing release profile, before Binaryen | — | 10,248,040 | — |
| Existing release profile + `-O2` | 72.5 s | 9,616,589 | 1,604,344 |
| Size-specialized profile, before Binaryen | — | 4,911,252 | 815,265 |
| Size-specialized profile + `-O1` | 13.0 s | 4,693,425 | 800,618 |

The size-specialized pipeline cuts approximately 51% from the compressed transport
and 51% from installed engine bytes while making the Binaryen stage roughly 5.6
times faster than `-O2` on the same machine. `-O2` after the size-specialized
profile reduced raw bytes further but increased gzip output to 896,066 bytes and
took 37.0 seconds, so it was rejected. Runtime acceptance depends on the full
backend matrix rather than either optimizer profile name.

## Consequences

The expected warm-cache critical-path reduction is roughly two minutes without
removing a supported browser, bundler, runtime, parser check, performance gate,
or release artifact. The dedicated profile has its own Rust-cache cohort. Any
future optimizer or size-budget change requires the same fixed-input size/timing
comparison and complete backend validation.

This updates the `-Oz` implementation choice in ADR 0172; its public backend and
validation contract otherwise remains in force.
