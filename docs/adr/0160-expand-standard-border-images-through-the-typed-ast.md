# ADR 0160: Expand standard border images through the typed AST

## Status

Accepted for RC6.

## Context

The standard `border-image` grammar was accepted by the vendored Lightning CSS parser, but `Property::longhand()` rejected all five standard longhands because the shorthand carries a vendor-prefix bitset and its longhand identifiers do not. SheetOM then fell back to a partial hand-written slash parser that missed repeat keywords after width, empty slash sections, image sets, and other valid branches.

## Decision

- Patch the vendored Lightning CSS `Property::longhand()` boundary to expose all five semantic border-image longhands before its generic prefix guard.
- Keep the vendor correction and its regression test in a dedicated commit suitable for upstreaming.
- Route every branch understood by the vendored parser through the typed AST rather than the partial legacy codec.
- Retain a narrow SheetOM parser fallback only for the versioned Chromium `sign()` capability branches that the vendored parser cannot type yet. The fallback still validates every resulting longhand and is never consulted after a successful typed parse.
- Retain authored longhand provenance after semantic matching, including explicit zero dimensions such as `0px` that the safe serializer canonicalizes to `0`.
- Continue to isolate every native AST interaction behind owned Rust values; no JavaScript AST round trip is introduced.

## Consequences

All 29 pinned Chromium `border-image` branches now match the shorthand getter, declaration text, five expanded longhands, atomic invalid-neighbor behavior, and safe round trip. The standard grammar has one typed authority, with a closed compatibility extension for the separately versioned contextual-math evidence.
