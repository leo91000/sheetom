# ADR 0116: Own browser keyword longhands in a Rust registry

## Status

Accepted for RC6.

## Context

Chromium 151 recognizes ordinary properties whose complete keyword branches are not represented by the vendored Lightning CSS property model. Treating those declarations as unsupported creates silent no-ops for browser-valid CSS. Keeping a parallel JSON runtime override would make conformance evidence an accidental parser and would duplicate the grammar authority.

Legacy `-webkit-column-break-*` aliases also demonstrate that property aliases are not always plain renames. Chromium stores the canonical property name, hides ordinary semantic values through the alias, preserves CSS-wide keywords, and keeps pending substitutions observable only through the alias that received the write.

## Decision

SheetOM owns complete finite-keyword longhand grammars in one reviewed Rust registry. The registry generates parser dispatch and inventory membership, while the Chromium-versioned JSON contract remains test evidence only. Each registered keyword is parsed from recovered CSS Syntax components and stored as semantic state; unknown neighboring keywords are rejected atomically.

Alias-specific observability is retained as provenance on the canonical declaration record. It does not create a second declaration or a second mutable source of semantic truth.

The browser differential executes every registered keyword, ASCII case folding, list branch, invalid neighbor, alias query, priority, pending substitution, and removal sequence against pinned Chromium. Full CSS declaration state is compared after every operation.

## Consequences

- Finite browser grammars have one typed runtime authority and cannot drift from dispatch.
- Browser evidence cannot increase the runtime bundle or become an exact-value allowlist.
- Alias quirks survive mutation and serialization without storing duplicate declarations.
- Composite grammars remain outside this registry and require dedicated typed codecs and branch contracts.
