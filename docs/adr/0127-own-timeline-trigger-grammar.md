# ADR 0127: Own timeline-trigger lists and CSSOM synthesis

## Status

Accepted for RC6.

## Context

The initial `timeline-trigger` fallback represented only scalar dashed timeline
names and scalar ranges. Chromium's grammar is list-valued across six expanded
longhands, accepts typed `scroll()` and `view()` sources, and allows each
shorthand item to omit default name, source, activation-range and active-range
components independently.

CSSOM serialization adds another constraint: Chromium deliberately abbreviates
some getters in ways that do not round-trip to the same expanded state. For
example, an explicitly default range end can disappear from the observable
shorthand even when reparsing the abbreviated value would infer another end.
Using that getter text for stylesheet serialization would silently change the
declaration.

## Decision

Timeline sources are parsed as lists of Lightning CSS `AnimationTimeline`
values, retaining typed named, scroll and view timelines. Activation and active
range longhands are represented as typed comma lists. Timeline trigger names
own a list model that permits the `none` placeholders Chromium creates while
expanding a mixed shorthand list.

The shorthand parser splits only top-level commas and slashes, then assigns the
optional name, source and range components in grammar order. Every item expands
into the six parallel longhand lists; unequal list lengths cannot synthesize a
shorthand.

Observable synthesis follows Chromium's abbreviated getter, including a
leading slash when only the active-range start is non-default. Safe stylesheet
synthesis always emits all six canonical components for every item. This keeps
browser-facing CSSOM fidelity separate from reparsable semantic fidelity.

The generated Webref differential is the release authority. Every generated
timeline-trigger property branch must have zero mismatches for acceptance,
getter, `cssText`, indexed longhands, invalid-neighbor atomicity and safe
reparsing.

## Consequences

- Timeline trigger source functions and comma lists no longer disappear.
- Longhand mutation can synthesize a shorthand only while all six parallel
  lists remain aligned.
- Browser-observable shorthand text can remain intentionally non-round-tripping
  without compromising `CSSStyleSheet.serialize()`.
- Runtime parsing is grammar-driven and does not import the Webref observation
  corpus.
