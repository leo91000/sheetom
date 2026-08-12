# ADR 0138: Own non-finite corner-shape exponents

## Status

Accepted for RC6.

## Context

Chromium accepts bare `infinity` and `-infinity` identifiers inside
`superellipse()` while the generic `<number>` parser accepts non-finite values
only through CSS math functions. Routing the corner-shape grammar directly
through that generic parser therefore rejected two valid branches across all
seventeen physical, logical and shorthand corner-shape properties.

The distinction is observable. Chromium preserves a bare infinite exponent as
`superellipse(infinity)`, but preserves a calculated exponent as
`superellipse(calc(infinity))`. Bare `NaN` and a leading `+` remain invalid,
while `calc(NaN)` is a valid deferred calculation.

## Decision

The corner-shape grammar owns an explicit exponent value with two forms:

- a normal or calculated CSS number, serialized by the typed number codec;
- a bare positive or negative infinity identifier, serialized without a
  synthetic `calc()` wrapper.

No global number parser is weakened. Other properties continue to decide
whether a bare non-finite identifier belongs to their grammar.

## Evidence

Core tests cover case-insensitive bare infinities, calculated infinities and
`NaN`, division-by-zero canonicalization, and adjacent invalid tokens. Public
tests cover shorthand expansion, synthesis, mutation, removal and atomic
invalid replacement. The versioned value corpus executes the accepted and
rejected branches against Chromium, and the process-safety suite exercises a
mixed non-finite shorthand in isolated native and public subprocesses.

The Webref-derived ratchet removes 34 mismatch cases, including all 34 false
rejections attributable to bare infinite corner shapes. Atomicity remains at
zero mismatches.

## Consequences

- Every Chromium corner-shape property accepts both non-finite branches.
- Getter and `cssText` projection retain the distinction between bare and
  calculated exponents.
- Bare non-finite identifiers remain opt-in at the property grammar boundary.
