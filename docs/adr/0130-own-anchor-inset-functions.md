# ADR 0130: Own anchor inset functions in the typed property grammar

## Status

Accepted for RC6.

## Context

Chromium accepts `anchor()` in every physical and logical inset longhand and in
the `inset`, `inset-block` and `inset-inline` shorthands. The function accepts
an optional dashed anchor name and a required side in either order, plus an
optional fallback. That fallback can itself contain `anchor()`,
`anchor-size()` or either function inside CSS math.

The upstream Lightning CSS snapshot did not parse `anchor()`. SheetOM therefore
rejected 119 accepted Webref branches across the inset family. Treating the
function as an unrestricted token fallback would also accept it in margins,
padding and sizing properties where Chromium rejects it, and would prevent
typed shorthand expansion.

## Decision

The vendored typed value model has distinct length domains:

- sizing and margin values accept ordinary lengths and `anchor-size()`;
- inset values additionally accept `anchor()`;
- the recursive fallback of `anchor()` is boxed and uses the inset length
  domain, so nested anchor functions remain finite in memory and typed;
- ordinary dimensions sort before anchor functions in additive CSS math to
  match Chromium CSSOM serialization.

`anchor()` is parsed only by the physical and logical inset longhands. Their
existing shorthand handlers consequently expand one-to-four values without a
parallel shorthand record. Substitution-bearing values remain pending
substitution groups and retain their authored observable spelling.

The vendored parser tests cover valid ordering, percentages, recursive
fallbacks, math, invalid neighbors and rejection outside inset grammar. The
native Chromium differential additionally compares priority, indexed entries,
getter, `cssText`, removal and atomic invalid replacement. The generated Webref
gate must retain zero inset-family mismatches in every measured dimension.

## Consequences

- Static anchor insets participate in normal longhand mutation and removal.
- Recursive fallbacks are accepted without an untyped escape hatch.
- `anchor()` remains invalid outside inset properties while `anchor-size()`
  keeps its broader accepted-property domain.
- The grammar fix is isolated in the vendored Lightning CSS commit so it can be
  proposed upstream independently of SheetOM's conformance evidence.
