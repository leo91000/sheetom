# ADR 0123: Inventory formal property grammar branches

## Status

Accepted for RC6.

## Context

The complete Property Value Matrix crosses every Chromium property with a broad set of reviewed values, but a shared probe set cannot prove every branch of every property grammar. A property can accept one representative value while still rejecting another alternative, cardinality, separator, permutation or optional component. Manually extending the shared probes after each bug repeats the same blind spot at a larger scale.

Webref publishes machine-readable Value Definition Syntax for standard CSS properties. That syntax is useful for discovering candidates, but it is neither a browser-support manifest nor a property-value validator. Some Chromium properties are absent, current specifications may be ahead of the pinned browser, and recursively deep semantic types cannot be represented by a finite list of strings.

## Decision

SheetOM uses the exact pinned `@webref/css` data only to generate test candidates. A deterministic sampler covers every reachable alternative, bounded multiplier minimum/adjacent/maximum cardinalities, comma lists, pair subsets, component permutations and pairwise combinations without constructing an unbounded Cartesian product. Reviewed semantic terminals stop recursive expansion at explicit seams; every such seam remains labelled `representative` until a dedicated branch contract proves otherwise.

Pinned Chromium classifies every generated candidate and records accepted observable values, declaration serialization, indexed declaration state and an invalid neighbor. The checked-in corpus explicitly lists Chromium properties absent from Webref and hashes the property manifest, semantic terminals and exact Webref data. Webref never enters the runtime dependency graph and browser observations never become literal parser exceptions.

During RC6 development, an exact per-property/per-sample mismatch ratchet prevents regressions while focused pull requests remove known differences. Updating the ratchet requires reviewing the complete generated diff. The ratchet is not a release allowance: RC6 requires the strict checker to reach zero acceptance, observable-value, `cssText`, item-order, atomicity and reparsing mismatches.

Browser reprobe runs reuse the Chromium already installed by the browser-compatibility job. Public-runtime checking runs against the checked-in observations without launching a browser, keeping the hot CI path to a few seconds.

## Consequences

- Grammar depth is measured systematically instead of inferred from one successful value per property.
- Generated candidates are versioned evidence, not a claim that Webref and Chromium implement identical grammars.
- Missing vendor properties and finite semantic seams remain visible rather than silently disappearing from coverage totals.
- Runtime corrections must remain typed, general grammar implementations in the owned Rust engine or vendored source.
- A future Webref update fails closed when syntax, samples or browser observations drift.
