# ADR 0150: Own overscroll chaining

- Status: Accepted
- Date: 2026-08-12

## Context

Pinned Chromium accepts the `chain` keyword for physical and logical
overscroll-behavior longhands and in either component of the shorthand.
SheetOM's browser grammar retained only `auto`, `contain`, and `none`, so valid
author CSS using the newer keyword was dropped atomically.

## Decision

The shared overscroll keyword grammar includes `chain` for all four longhands.
The existing two-value shorthand codec validates and expands it exactly like
the other overscroll keywords. Browser evidence covers every keyword on every
longhand, one- and two-component shorthand forms, mutation and removal, invalid
atomic replacement, and safe round trips.

## Consequences

All pinned Chromium overscroll chaining branches survive parsing and remain
fully mutable. No exact-value runtime exception is introduced.
