---
status: accepted
---

# Preserve pending shorthand semantics during serialization

Pending-substitution shorthands retain shared, immutable provenance after a
longhand mutation breaks their CSSOM shorthand getter. Reparsable Stylesheet
Serialization emits the authored shorthand before the current longhand
overrides. This preserves the browser's deferred substitution semantics without
changing observable `cssText`.

CSS text exactly represents equal-priority overrides and an important longhand
over a normal shorthand. It cannot exactly represent a normal longhand winning
imperatively over an important shorthand, nor a removed member inheriting from
the cascade while the remaining members retain pending shorthand semantics.

`serialize()` remains total for accepted user CSS. In an unrepresentable state,
it emits the authored shorthand and minimally promotes conflicting surviving
longhands so the explicit local values win. With diagnostics enabled it records
`UNREPRESENTABLE_PENDING_SHORTHAND`, the shorthand, and the affected longhands.
This is a documented best-effort projection, not an exact round trip.

`serializeStrict()` uses the same projection but throws a typed
`SheetOMSerializationError` before returning CSS when exact semantics cannot be
preserved. It exists for validators and migration tools; production stylesheet
generation should use the resilient default. Independent pending longhands
remain independent as required by ADR 0182.
