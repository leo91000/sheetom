# ADR 0131: Own intrinsic flex basis and `calc-size()` grammar

## Status

Accepted for RC6.

## Context

Chromium accepts intrinsic sizing keywords in `flex-basis`, `flex` and their
WebKit aliases. It also accepts `calc-size()` as a `flex-basis` longhand value,
including nested bases and calculations that refer to the `size` placeholder.
The function is not accepted as the basis component of the `flex` shorthand.

The upstream Lightning CSS snapshot represented `flex-basis` as only
`auto | <length-percentage>`. SheetOM consequently rejected 44 accepted Webref
branches across the standard and WebKit properties. A generic unparsed fallback
would lose longhand expansion, admit `calc-size()` in the shorthand, and make
invalid replacement non-atomic.

## Decision

The vendored typed grammar has a dedicated `FlexBasis` model containing:

- `auto`, `content`, intrinsic min/max/fit keywords and `stretch`, including
  their accepted legacy spellings;
- ordinary length-percentage values;
- a typed recursive `calc-size()` value for the longhand only.

The `calc-size()` parser distinguishes its basis from its calculation. The
calculation uses a dedicated dimension whose `size` placeholder participates in
addition, multiplication, division and nested CSS math without becoming an
arbitrary identifier. An `any` basis rejects calculations that reference
`size`, and number-only or dimensionally incompatible calculations are invalid.

The shorthand parser explicitly excludes `calc-size()`, matching Chromium even
though the same value is valid in `flex-basis`. SheetOM's contextual shorthand
fallback performs the same token-based exclusion so it cannot bypass the typed
grammar. Static shorthands continue to own only their three expanded longhands.

Serialization follows Chromium's observable canonical form, including ordering
ordinary dimensions before `size`, coefficient placement, and parentheses for
a scaled `size` term inside a sum. Safe whole-sheet serialization may use a
shorter equivalent shorthand spelling.

## Evidence

Vendored parser tests cover intrinsic keywords, aliases, shorthand ordering,
recursive `calc-size()` bases, math functions, canonical serialization and
invalid neighboring values. The native Chromium differential compares indexed
entries, getters, priorities, `cssText`, removal and atomic invalid replacement.
The generated Webref ratchet removes all 44 acceptance mismatches in the flex
family without adding atomicity, reparse or unrelated mismatches.

## Consequences

- Valid intrinsic flex declarations no longer disappear during mutation or
  stylesheet parsing.
- `flex-basis` can retain modern `calc-size()` values as typed semantic state.
- Invalid `calc-size()` shorthand values remain atomic no-ops.
- The parser change remains isolated in a vendored Lightning CSS commit that can
  be reviewed or proposed upstream independently.
