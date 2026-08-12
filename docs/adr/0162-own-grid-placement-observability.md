# ADR 0162: Own grid placement observability

## Status

Accepted for RC6.

## Context

`grid-area`, `grid-row`, and `grid-column` already expanded atomically, but their ordinary values retained authored token order instead of Chromium's CSSOM ordering. The SheetOM contextual-math synthesizer was marked authoritative even when no contextual math was present, so the vendored typed Grid AST never serialized the shorthand.

The vendored `GridLine` parser also interpreted `1 span` as line number `1` with the custom name `span`. Chromium treats it as a span and serializes it as `span 1`. The `span` grammar permits the keyword before or after its contiguous integer/custom-ident group, with only positive span counts.

## Decision

- Use the typed Grid shorthand serializer for ordinary placement values and retain the SheetOM contextual path only when its recovery evidence is present.
- Project expanded grid-line longhands from their typed canonical values instead of recovering authored ordering.
- Patch the vendored `GridLine` parser to recognize the Chromium-supported before/after span group orders, reject separated groups, reject bare or negative spans, and reserve `auto` and `span` from custom line names.
- Keep the vendored parser correction and its permutation tests in a dedicated commit suitable for upstreaming.

## Consequences

All pinned `grid-area`, `grid-row`, and `grid-column` getter, declaration-text, indexed-longhand, atomicity, and safe-reparse checks now match Chromium. The full corpus remains acceptance-exact and reparse-idempotent.
