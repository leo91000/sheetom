# ADR 0120: Own geometric values and SVG paths

## Status

Accepted for RC6.

## Context

`border-shape`, `d`, `object-view-box`, and `shape-outside` were the last Chromium ordinary properties without an authoritative grammar owner. Treating them as generic token streams would accept invalid neighbors, while relying on a single parser representation would lose Chromium-observable recovery, authored gradient colors, SVG path canonicalization, and new `shape()` command semantics.

These values also combine two syntaxes. CSS Syntax defines functions, component values, calculations, images, and recovery. The string inside `path()` follows SVG path-data grammar and has different numeric and command rules. Neither prefix checks nor one permissive parser can establish both contracts.

## Decision

SheetOM exposes one deep Rust entry point, `parse_geometric_property`, backed by typed values for every supported geometric branch.

- Vendored `cssparser` owns CSS tokenization, nesting, component boundaries, and error recovery.
- SheetOM owns the property-specific grammar for basic shapes, geometry boxes, `shape()` commands, range checks, canonical omission, and CSSOM projection.
- The exactly pinned `svgtypes` parser recognizes SVG path-data commands and numbers. SheetOM converts its output into owned command state and applies Chromium's path serialization rules itself.
- Vendored Lightning CSS remains a typed image and gradient primitive. The local conic-gradient correction accepts unitless zero stop positions and carries an upstream-style regression test.
- Authored gradient color provenance is retained separately from the semantic image, including legacy `-webkit-gradient()`, so safe serialization and browser-facing getters do not erase observable color spelling.

The checked-in contract contains a valid value and invalid neighbor for every reviewed branch. A deterministic expansion additionally covers all arc-option permutations, logical-axis alternatives, SVG command families and geometry-box ordering. The browser gate compares complete declaration state, invalid-replacement atomicity, removal, SheetOM idempotence, and Chromium reparsing. Large paths, polygons, shape command sequences, nested gradients, malformed path data, and over-budget inputs execute in isolated subprocesses.

## Consequences

- Every Chromium property in the ordinary-property manifest now has an explicit grammar owner.
- The runtime stores semantic geometry rather than accepting exact strings from the browser evidence corpus.
- SVG path recognition is delegated to a narrow parser, but Chromium compatibility and serialization remain SheetOM policy and are independently tested.
- RC6 release evidence records 55 reviewed and 144 generated geometric branches, plus the exact contract, generator, and execution hashes.
