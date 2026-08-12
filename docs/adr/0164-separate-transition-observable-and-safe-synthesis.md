# ADR 0164: Separate transition observable and safe synthesis

## Status

Accepted for RC6.

## Context

SheetOM expanded `transition` atomically but used its reparsable shorthand serializer for the live CSSOM getter and declaration text. Chromium independently omits default duration, timing, delay, and behavior components. For example, `none 0s linear 1s normal` is observable as `none linear 1s`, even though reparsing that spelling assigns the single time to the duration rather than the delay.

The safe stylesheet projection must not copy that browser quirk because SheetOM promises idempotent reparsing there.

## Decision

- Synthesize browser-facing `transition` and `-webkit-transition` by independently omitting each default component in Chromium order.
- Retain the existing disambiguating serializer for the safe stylesheet projection.
- Exercise the distinction directly and gate every pinned transition branch through the Webref Chromium corpus.

## Consequences

All pinned standard and prefixed transition getter and declaration-text checks now match Chromium. Safe serialization remains idempotent, acceptance remains exact, and invalid neighbors remain atomic.
