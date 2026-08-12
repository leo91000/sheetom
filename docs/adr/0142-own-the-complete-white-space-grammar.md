# ADR 0142: Own the complete `white-space` grammar

- Status: Accepted
- Date: 2026-08-12

## Context

The existing shorthand codec covered legacy aliases and a collapse value followed
by a wrap mode. Chromium also accepts either component alone and accepts the two
components in either order. Omitted components are stored as the CSS-wide
`initial` value, while the shorthand getter synthesizes their initial semantic
value. Rejecting those branches lost valid author CSS even though the individual
longhands were already supported.

## Decision

The native `white-space` codec now:

- accepts a collapse component or wrap mode independently;
- accepts one component from each domain in either order;
- records an omitted longhand as `initial`, matching Chromium's indexed state;
- treats `initial` as the longhand's initial semantic value only while
  synthesizing the shorthand getter;
- rejects duplicate-domain and extra components atomically;
- retains the legacy aliases and pending-substitution path.

## Consequences

Observable shorthand text, indexed longhands, mutation after expansion, and safe
whole-sheet serialization agree with Chromium across the complete reviewed
grammar. The grammar contract includes omitted, unordered, and invalid duplicate
branches instead of relying on one representative value.
