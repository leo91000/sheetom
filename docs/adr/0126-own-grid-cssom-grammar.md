# ADR 0126: Own complete grid shorthand CSSOM behavior

## Status

Accepted for RC6.

## Context

The vendored Lightning CSS grid parser rejected valid `grid` and
`grid-template` branches when `none` was followed by a column track list, when
`subgrid` reached the top-level slash, or when template-area rows had line names
on both sides. Its normal stylesheet printer also deliberately formats template
areas across several lines, while Chromium exposes a single-line CSSOM value.

Those differences made valid declarations disappear or moved line names to the
wrong side of a track. SheetOM also needs to preserve Chromium's expanded
longhand order for the two auto-flow branches so later longhand mutation can
reconstruct the original shorthand.

## Decision

The vendored Lightning CSS source owns the grammar-level repairs:

- the `none` fast path backtracks unless it consumes the complete value;
- `subgrid` stops at an outer delimiter after parsing all valid line-name and
  repeat components;
- template-area parsing retains an explicit leading line-name slot, keeping
  trailing names after their row track;
- track lists insert the required separator before trailing line names.

Grid values expose dedicated single-line CSSOM serializers. The existing
stylesheet serializers retain their pretty multiline layout; CSSOM getters and
shorthand synthesis use the new serializers explicitly.

SheetOM records the auto-flow expansion in Chromium order and synthesizes its
row and column forms from the six longhands. Explicit template and template-area
forms continue through the typed Lightning CSS model. Invalid replacements are
validated before declaration-state mutation.

The generated Webref differential is the release authority. Every generated
`grid` and `grid-template` branch must have zero mismatches for acceptance,
observable getter value, `cssText`, indexed longhands, invalid-neighbor
atomicity and safe reparsing.

## Consequences

- All generated Chromium-supported grid shorthand branches are accepted and
  remain observable after expansion.
- Line names keep their semantic side across parse, mutation and serialization.
- CSSOM output is single-line without changing ordinary stylesheet formatting.
- The vendored parser fixes are isolated in their own commit so they can be
  proposed upstream independently.
