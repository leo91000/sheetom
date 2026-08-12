# ADR 0141: Own the complete `initial-letter` grammar

- Status: Accepted
- Date: 2026-08-12

## Context

The RC6 Webref differential proved that accepting `initial-letter: normal` and a
single positive number was not enough. Chromium also accepts an optional sink as
a positive integer or the `drop` and `raise` keywords, permits the keyword before
the size, and defers range checks for calculated values. Treating these branches
as unrelated string exceptions would leave component ordering, math
canonicalization, invalid neighbors, and atomic mutation unowned.

## Decision

SheetOM owns `initial-letter` as a semantic native value with:

- a positive direct number for the initial-letter size;
- an optional positive direct integer sink or `drop`/`raise` keyword;
- Chromium's keyword-before-size input and size-before-sink observable output;
- number-result calculations whose range and integrality remain deferred;
- strict whole-value consumption and atomic rejection of extra components.

Pending substitutions remain token-owned by the shared substitution model. The
parser stores math semantics and authorship rather than routing compound values
through a generic scalar-number fallback.

## Consequences

Every Webref-derived Chromium branch for `initial-letter` is accepted and
canonicalized without weakening adjacent invalid values. Public tests,
differential sequences, value-capability cases, crash-safety subprocesses, and
the Webref ratchet all cover the grammar.
