# ADR 0147: Own compound container types

- Status: Accepted
- Date: 2026-08-12

## Context

CSS Containment allows `scroll-state` to stand alone or combine, in either
authored order, with exactly one of `size` and `inline-size`. Chromium
canonicalizes the size component before `scroll-state`. The vendored Lightning
CSS enum represented only one keyword, so SheetOM rejected six accepted Webref
branches across `container-type` and the `container` shorthand.

## Decision

The vendored parser owns the complete production
`normal | [[size | inline-size] || scroll-state]`. Its semantic value has
explicit variants for each valid combination and serializes in Chromium order.
`normal` cannot combine, size modes are mutually exclusive, and duplicate
components leave the candidate unparsed so SheetOM rejects it before mutation.

The `container` shorthand continues to expand through the standard typed AST;
it does not gain an exact-value override. Positive, reordered, invalid-neighbor,
priority, longhand mutation, subprocess, and whole-sheet round-trip cases are
part of the versioned evidence.

## Consequences

All six previously rejected Webref branches retain semantic declaration state
and browser-facing canonicalization. The vendored change is isolated in its own
commit so it can be proposed upstream independently from SheetOM's evidence.
