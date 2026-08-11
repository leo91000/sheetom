# ADR 0128: Own offset path and motion grammar

## Status

Accepted for RC6.

## Context

The previous `offset-path` fallback represented only `none`, URLs and a narrow
subset of `ray()`. It rejected Chromium-valid coordinate boxes, unordered ray
components and SVG `path()` values. The `offset` shorthand consequently lost
valid declarations or exposed non-canonical longhand state.

CSSOM shorthand serialization has an additional semantic rule. Chromium keeps
an explicit literal zero angle in `offset-rotate: auto 0deg`, while omitting it
as the default rotation when reconstructing `offset`. A computed
`calc(0deg)` remains observable and cannot be treated as the same lexical
default.

## Decision

SheetOM owns `offset-path` as a typed grammar consisting of an optional path
component and an optional geometry box. Path components include `none`, URL,
basic shape, SVG path data and `ray()`. Ray arguments are parsed without order
dependence and retain their angle, size, `contain` flag and optional position.
SVG commands reuse the existing typed path parser instead of introducing a
second string-level implementation.

The `offset` shorthand expands position, path, distance, rotation and anchor
atomically. Its synthesis treats only a literal zero angle paired with `auto`
as the default rotation; CSS calculations are not collapsed by that CSSOM
abbreviation rule. Safe serialization remains based on the complete typed
longhand state.

The generated Webref differential is the release authority. Every generated
`offset` and `offset-path` branch must have zero mismatches for acceptance,
getter, `cssText`, indexed longhands, invalid-neighbor atomicity and safe
reparsing.

## Consequences

- Coordinate boxes can appear before or after the path component without
  changing the canonical result.
- Complete `ray()` and SVG path values no longer disappear from stylesheets.
- Invalid duplicate components and unsupported geometry boxes remain atomic
  no-ops.
- Runtime parsing remains grammar-driven and does not import browser evidence.
