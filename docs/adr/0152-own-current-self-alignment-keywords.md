# ADR 0152: Own current self-alignment keywords

- Status: Accepted
- Date: 2026-08-12

## Context

Pinned Chromium accepts `anchor-center`, optionally preceded by `safe` or
`unsafe`, for `align-self` and `justify-self`. It also accepts bare `legacy` for
`justify-items`, in addition to the ordered or reordered legacy side forms.
The vendored standard AST predated both branches, so SheetOM rejected valid
longhands and their `place-self` or `place-items` shorthand forms.

These keywords do not belong to every alignment property. Chromium rejects
`anchor-center` for `align-items` and `justify-items`, and rejects `legacy` for
self alignment. A generic keyword fallback would therefore weaken adjacent
grammars.

## Decision

The vendored alignment AST and parser explicitly represent anchored self
alignment, including its optional overflow position, and bare legacy item
justification. Shorthand copying and synthesis preserve those semantic variants.
The existing alignment types continue to reject the keywords on properties
where Chromium does not support them.

Browser evidence covers every new longhand branch, reordered legacy syntax,
both place shorthands, invalid property boundaries, mutation, removal,
atomicity, safe round trips, and crash isolation. Vendored source changes remain
in a dedicated commit so they can be evaluated for upstreaming.

## Consequences

All pinned Chromium anchored self-alignment and bare legacy branches survive
parsing without exact-value exceptions. The parser retains strict distinctions
between self alignment, item alignment, and shorthand component grammars.
