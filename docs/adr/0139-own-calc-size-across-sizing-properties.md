# ADR 0139: Own calc-size across sizing properties

## Status

Accepted for RC6.

## Context

The vendored engine already parsed `calc-size()` for `flex-basis`, but all box
sizing properties rejected it. A single missing grammar branch therefore
caused false rejections across physical, logical and legacy WebKit aliases.

The grammar is property-sensitive. Preferred and minimum sizes accept `auto`
as a basis; maximum sizes do not. Both families accept intrinsic keywords,
length-percentage and anchor-size calculations, `any`, and nested
`calc-size()` values whose basis is valid in that context. The calculation can
refer to the `size` placeholder unless the basis is `any`.

Chromium also recovers after a complete second argument by ignoring subsequent
component values inside the function. That recovery includes commas and
semicolons, so it must remain confined to the function block and must never
create an adjacent declaration.

## Decision

The vendored engine owns a reusable typed `CalcSize<V, D>` module. A containing
property supplies its valid basis grammar and typed length dimension. The
module owns arithmetic, nested functions, the `size` placeholder, `any`
restrictions, canonical serialization and Chromium-compatible tail recovery.

Preferred/minimum `Size` and maximum `MaxSize` expose separate aliases and
basis filters. `flex-basis` retains its established property-specific basis
type while sharing the same confined tail-recovery rule. The generic
`IntoOwned` implementation is explicit because the vendored derive macro does
not support this generic recursive shape in every production feature set.

## Evidence

Vendored tests cover keyword, numeric, anchor and nested bases; arithmetic,
rounding, sign and percentage calculations; preferred-versus-maximum basis
differences; invalid result types; and recovered tails. Public CSSOM tests
cover twelve physical, logical and legacy properties, canonical getters,
atomic replacement, pending substitutions, alias projection and idempotent
whole-sheet serialization.

The versioned Chromium corpus adds accepted and rejected `calc-size()`
branches. Native differentials cover preferred, minimum, maximum, nested,
invalid and recovery sequences. A semicolon recovery case proves that the
function cannot inject a following declaration, and process-safety executes a
nested anchor/mathematical value in isolated native and public subprocesses.

The Webref-derived ratchet removes twelve false-rejection cases. Atomicity
remains at zero mismatches.

## Consequences

- Sizing properties no longer drop valid modern `calc-size()` declarations.
- Maximum sizes cannot accidentally acquire the preferred-only `auto` basis.
- Nested and anchor-relative size calculations retain typed semantic state.
- Chromium's permissive function-tail recovery is reproduced without weakening
  the surrounding declaration parser.
