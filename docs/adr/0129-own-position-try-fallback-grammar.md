# ADR 0129: Own position-try fallback grammar and synthesis

## Status

Accepted for RC6.

## Context

The first `position-try-fallbacks` model recognized named fallbacks and three
logical tactics, but required the dashed name to precede every tactic. Chromium
also accepts the physical `flip-x` and `flip-y` tactics and allows a dashed name
to appear in any order relative to distinct tactics.

The declaration state separately retained the authored `normal` order in the
`position-try` shorthand. Chromium expands `normal` into the longhand default
and reconstructs the getter from the canonical fallback. Conversely, a
non-default order paired with `none` must retain the explicit `none` because an
order alone is not a valid shorthand.

## Decision

Each fallback-list item is parsed as one of two exclusive forms:

- a complete typed position area; or
- an unordered dashed name and distinct set of `flip-block`, `flip-inline`,
  `flip-start`, `flip-x` and `flip-y` tactics.

The parser first attempts the complete position-area grammar, then parses the
unordered named form component by component. Duplicate names, duplicate
tactics, mixed position areas and tactics, and unknown identifiers are rejected
atomically.

Static `position-try` groups always synthesize from their two longhands.
`normal` is omitted, position areas are canonicalized, and a non-default order
with no fallbacks serializes as `<order> none`.

The generated Webref differential is the release authority. Every generated
`position-try` and `position-try-fallbacks` branch must have zero mismatches for
acceptance, getter, `cssText`, indexed longhands, invalid-neighbor atomicity and
safe reparsing.

## Consequences

- Physical and logical tactics can be combined without source-order
  restrictions.
- Position-area aliases and component order follow Chromium CSSOM
  canonicalization.
- Shorthand getters no longer expose a redundant leading `normal`.
- Runtime parsing remains grammar-driven and independent of browser fixtures.
