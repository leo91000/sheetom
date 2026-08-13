---
status: accepted
---

# ADR 0181: Optimize WASM CI without weakening evidence

## Decision

SheetOM will keep the established release profile, including LTO, symbol
stripping, one codegen unit and unwind safety, and replace Binaryen `wasm-opt -Oz`
with the measured `-O2` profile. The build fails unless Binaryen removes at least
five percent and the final engine remains at or below 10,000,000 raw bytes and
1,700,000 gzip-9 bytes.

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

The profile comparison used the current engine and identical source, toolchain,
LTO, symbol-stripping and unwind settings:

| Pipeline stage | Time | Raw bytes | gzip -9 bytes |
| --- | ---: | ---: | ---: |
| Existing release profile, before Binaryen | — | 10,248,040 | — |
| Existing release profile + `-O2` | 72.5 s | 9,616,589 | 1,604,344 |
| Size-specialized Rust profile + `-O1` | 13.0 s | 4,693,425 | 800,618 |

The size-specialized candidate was rejected despite its size and build-time gains.
In the first complete browser run, Firefox completed the Publisher workload in
25.59 seconds, close to the 30-second limit. On the post-merge run, the same
artifact exceeded that limit and failed while Chromium remained at 5.28 seconds.
The candidate therefore made a supported runtime materially slower and unstable.
The release-profile `-O2` pipeline keeps the runtime code-generation contract,
cuts the measured Binaryen stage by approximately 53%, and remains within explicit
raw and compressed size budgets.

## Consequences

The expected warm-cache critical-path reduction is roughly two minutes without
removing a supported browser, bundler, runtime, parser check, performance gate,
or release artifact. Any future optimizer or size-budget change requires the same
fixed-input size/timing comparison and at least two complete backend performance
runs before merge.

This updates the `-Oz` implementation choice in ADR 0172; its public backend and
validation contract otherwise remains in force.
