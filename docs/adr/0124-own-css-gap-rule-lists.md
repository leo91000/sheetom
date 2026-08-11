# ADR 0124: Own CSS gap-rule lists as parallel typed structures

## Status

Accepted for RC6.

## Context

CSS Gaps rule properties are not ordinary border aliases. Their shorthands contain unordered width, style and color components inside comma lists, integer `repeat()` groups and one optional `repeat(auto, ...)` group. Chromium additionally accepts and removes a trailing comma after a final auto-repeat, while rejecting a corresponding leading comma. Mapping these properties to a single Lightning CSS border value accepted only one rule and rejected more than a thousand Chromium-supported Webref branches.

The three expanded longhands must retain identical list and repeat structure. Treating them as independent strings would accept impossible combinations, lose shorthand synthesis after mutation and make invalid replacement non-atomic.

## Decision

SheetOM owns CSS gap-rule syntax in a typed Rust module built on the vendored CSS Syntax tokenizer. One recursive list structure represents ordinary entries and integer or auto repeats. A gap-rule leaf owns typed Lightning CSS values for width, style and color plus the authored component needed for Chromium-compatible observable serialization.

The shorthand parser expands every leaf into three parallel longhand lists. `rule` and the `rule-*` shorthands duplicate the same semantic lists across row and column axes. Shorthand synthesis succeeds only when every longhand has the same list and repeat shape; removing a member makes the shorthand unobservable. Invalid counts, nested repeats, multiple auto repeats and misplaced empty sides are rejected before any declaration-state mutation.

The Webref differential remains the compatibility authority. It covers the complete generated list/repeat family, Chromium canonicalization, indexed longhand state, safe reparsing and invalid-neighbor atomicity. The runtime imports no Webref observations.

## Consequences

- CSS Gaps rules are no longer constrained to a single border-like tuple.
- Initial shorthand observability and post-longhand-mutation synthesis share one typed structure.
- A trailing auto-repeat comma follows the pinned Chromium behavior, is omitted from observable serialization and never becomes necessary to reparse SheetOM's safe output; a leading empty side remains invalid.
- Parser work is proportional to authored list size; integer repeats remain symbolic and cannot allocate by their numeric count.
