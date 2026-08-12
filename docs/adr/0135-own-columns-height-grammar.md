# ADR 0135: Own the complete columns height grammar

## Status

Accepted for RC6.

## Context

CSS Multicolumn Layout extends `columns` from the familiar unordered
`column-width` and `column-count` pair with an optional `/ column-height`
section. Chromium expands every accepted shorthand into `column-width`,
`column-count`, `column-height`, and `column-wrap`. The previous SheetOM codec
always assigned the latter two longhands to `auto`, so every explicit height
was rejected even though the supported-property and Webref inventories exposed
the grammar.

`column-wrap` has an unusual CSSOM relationship to the shorthand. It is part of
the expanded state and must have the same priority, but its value is not emitted
by the `columns` shorthand getter. Removing `column-height` breaks shorthand
synthesis, while changing a complete group's height or wrap retains it.

## Decision

The repository-owned `columns` codec parses one required width/count section
and one optional top-level slash followed by exactly one typed `column-height`:

- width and count remain unordered and each may be omitted;
- an explicit `auto` height canonicalizes away from the shorthand getter;
- a non-`auto` height is serialized after ` / `;
- zero and CSS math use the typed `column-height` canonicalizer;
- percentages, negative lengths, intrinsic sizes, multiple heights, empty
  sections, and a second slash are rejected atomically;
- `-webkit-columns` aliases the same canonical longhand state;
- shorthand synthesis requires all four longhands and one priority, ignores the
  current `column-wrap` value, and never recreates a removed longhand.

## Evidence

Public CSSOM tests cover expansion, canonical defaults, compatible longhand
mutation, removal, priority mismatch, the WebKit alias, invalid replacement,
and reparsable round trips. The Grammar Branch Contract now records positive
height, calculated-height and unitless-zero branches with neighboring negative
forms. Native differential sequences compare the same operations with pinned
Chromium, and a subprocess case exercises nested calculations through both the
native and public boundaries.

The Webref-derived gate removes all 10 acceptance mismatches from the columns
height profile without changing observable, item-order, atomicity, or reparse
counts.

## Consequences

- Valid multicolumn height declarations are no longer silently discarded.
- Longhand mutation cannot reactivate stale shorthand text.
- Browser evidence remains separate from runtime acceptance; the codec relies
  on typed longhand grammars rather than observed literal values.
- Future multicolumn shorthand syntax must extend this codec and its reviewed
  branch contract together.
