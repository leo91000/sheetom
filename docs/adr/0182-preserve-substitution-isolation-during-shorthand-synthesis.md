---
status: accepted
---

# Preserve substitution isolation during shorthand synthesis

Reparsable Stylesheet Serialization must not synthesize a shorthand from
independently authored longhands when any member contains a deferred
substitution. Combining those declarations can couple computed-value
invalidation after substitution, especially when one fallback resolves to a
CSS-wide keyword. The serializer retains the longhands so each declaration is
validated independently by the browser.

A complete Pending Shorthand Group may still serialize through its original
shorthand because the authoring mutation already established that shared
invalidation boundary. Static longhands without deferred substitutions remain
eligible for ordinary shorthand synthesis. This rule applies to every
shorthand family rather than naming `place-content`, `var()`, or
`revert-layer` as special cases.
