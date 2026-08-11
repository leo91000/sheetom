# ADR 0117: Type composite browser longhand grammars

## Status

Accepted for RC6.

## Context

A property-name manifest and a successful initial value do not prove that a parser owns the property's grammar. Composite longhands combine ordered and unordered keywords, axis-dependent keywords, optional components, calculations, strings, ranges, and context-sensitive non-negative constraints. Accepting only one representative value recreates the shorthand coverage gap found before RC5.

Chromium also canonicalizes these branches in property-specific ways. Examples include expanding `object-position: center` to two axes, sorting containment and touch-action components, omitting the default `proximity` scroll-snap strictness, and accepting negative `calc()` syntax where a direct negative value is rejected at parse time.

## Decision

SheetOM represents each composite branch with typed Rust state parsed from CSS Syntax components. Shared primitives cover lengths, percentages, positions, strings, and keyword sets; property-specific codecs enforce exclusivity, cardinality, ordering, default omission, and neighboring invalid values.

The checked-in Chromium contract names every supported grammar branch and pairs it with an invalid neighbor. The browser differential applies the valid value with priority, attempts the invalid replacement, compares complete CSS declaration state, verifies atomicity, removes the property, and compares the return value and empty final state.

The contract is test evidence only. A Rust test proves that every contracted property is owned by the grammar registry, while the published runtime does not import the JSON corpus.

## Consequences

- Twenty-three previously unsupported ordinary properties are owned by semantic codecs.
- One hundred and five valid branches and their invalid neighbors are pinned to Chromium 151 behavior.
- Runtime behavior is generalized by grammar structure rather than exact observed input strings.
- The remaining complex properties stay explicitly unsupported until their own branch contracts and codecs land.
