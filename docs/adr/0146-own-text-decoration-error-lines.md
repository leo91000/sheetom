# ADR 0146: Own text-decoration error lines

- Status: Accepted
- Date: 2026-08-12

## Context

Chromium exposes `spelling-error` and `grammar-error` as mutually exclusive
`text-decoration-line` values. They may be combined with shorthand thickness,
style, and color components, but not with ordinary or other error line values.
The vendored parser predates these keywords, and the structural shorthand codec
therefore dropped the complete declaration.

## Decision

SheetOM owns the complete `text-decoration-line` grammar and uses that same
typed longhand parser to validate the shorthand's accumulated line components.
The shorthand continues to classify thickness, style, and color independently,
then synthesizes all four longhands in Chromium order. Default `auto`, `solid`,
`none`, and `currentcolor` components are omitted only when another component
keeps the shorthand non-empty.

An error line is rejected when combined with an ordinary line, another error
line, `none`, duplicate styles, duplicate thicknesses, or any trailing token.
All rejection happens before mutation.

## Consequences

Error-decoration declarations retain semantic longhand state, Chromium getter
canonicalization, mutation behavior, pending substitutions, safe whole-sheet
serialization, invalid-neighbor coverage, and subprocess crash isolation.
The branch contract is a dedicated grammar profile rather than a one-off value
allowlist.
