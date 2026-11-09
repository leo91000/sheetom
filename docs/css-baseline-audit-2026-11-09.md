# CSS Baseline audit — 9 November 2026

The cutoff is **2026-11-09**. The official npm registry reports **web-features
3.45.0**, source revision `45644301624f58b3e13817a2fc9ce067f882dd4a`.
The [downloaded archive](https://registry.npmjs.org/web-features/-/web-features-3.45.0.tgz)
matches registry SHA-512 integrity
`sha512-Atp/v77aPEuVIPsw7tYz9IrID8CztDcIleh8n/Z8r7HEkvNt7eYWHF8s4hlWQ0QskFzTPDjq81vjubXXCnBMSg==`.
The actual `data.json` bytes have SHA-256
`bcda8f1fd702fd2c7cc71779d35430022cf57b56dcd9415f3bb8ee14486eb3b4`.
The repository baseline is `a8286df17a6d4596c04225c2b26efed1ae7bf5fe`.

## Complete inventory delta

Relative to the [27 October audit](css-baseline-audit-2026-10-27.md), this refresh
adds one family, `scroll-driven-animations`, 103 compatibility records (100
eligible and three deferred), and makes the retained `animation-duration.auto`
branch eligible. There are no removed families or records. No retained family
changes its name, family availability date, or specification links, and no other
retained branch changes its status or date. The new family has a Newly Available
date of 2026-10-27. The resulting inventory contains **334 families, 4,466 unique
compatibility keys, 3,986 eligible records and 480 deferred records**. The evidence
map assigns **2,854 authoring records to 610 surfaces**, excluding **1,132
DOM/evaluation records**.

Eligibility uses individual Newly Available (`low`) and Widely Available (`high`)
branches dated at or before the cutoff, including branches in families that are
not wholly Baseline. Deferred entries comprise 417 not-Baseline and 63 after-cutoff
records; no status objects are missing. The three new deferred branches have
`baseline: false` and no availability date: animation shorthand inclusion of
`animation-timeline`, `timeline-scope: all`, and SVG view-fragment transform
parameters. They receive no invented dates or Baseline claim. Existing syntax is
preserved. Pinned mixin/function experiments and withdrawn historical Baseline
claims remain separate. The four September authoring gap groups remain covered
by the [shared September contracts](css-september-2026-parity-evidence.md); the
October mask and reset contracts also remain active.

## Contract and primary references

[WebDX](https://github.com/web-platform-dx/web-features) supplies availability;
[the pinned scroll-driven animation definition](https://github.com/web-platform-dx/web-features/blob/45644301624f58b3e13817a2fc9ce067f882dd4a/features/scroll-driven-animations.yml)
links the [Scroll-driven Animations specification](https://drafts.csswg.org/scroll-animations-1/).
Its recorded browser support starts at Chrome/Edge 115, Firefox 159, and Safari 26.
[Current browser compatibility data](https://github.com/mdn/browser-compat-data/blob/main/css/properties/animation-timeline.json)
corroborates those versions. The pinned Chromium 151 authoring oracle already
implements the selected CSS syntax, so no browser pin change is needed. Firefox
153 is retained only for the existing explicitly selected media-selector probes;
it is not evidence for Firefox 159 scroll-driven execution.

[CSS Box Alignment](https://drafts.csswg.org/css-align-3/) and
[CSS Flexbox](https://drafts.csswg.org/css-flexbox-1/) define the newly catalogued
alignment and `flex: content` syntax. All eligible property branches are tested
as authored CSSOM: parsing, support queries, observable longhands, mutation,
invalid-write atomicity, removal, and semantic serialization/reparse.
`ScrollTimeline`/`ViewTimeline` constructors, source/subject DOM associations,
computed offsets, scroll progress, animation execution and layout are outside
this contract. SVG path-length attributes and view-fragment parameters are DOM
or rendering behavior, not CSS declarations.

Every changed record is accounted for below. `baseline-november-09.<property>.*`
means the actual declaration probes in
[the browser corpus](../compatibility/css-authoring-probes.json), alongside the
existing Webref grammar profiles. `tests/baseline-november.test.ts` adds shared
native/WASM state, invalid-grammar, priority, removal, longhand expansion, typed
length-percentage expression, and reset contracts through
`scripts/test-modern-css-backends.ts`. A root mapping or arbitrary-token acceptance
alone is not evidence.

| Family | Compatibility record | Change | Disposition/evidence |
| --- | --- | --- | --- |
| animations-css | `css.properties.animation-duration.auto` | not-Baseline → Newly Available (2026-10-27) | Existing authoring support: `baseline-november-09.animation-duration.*` and Webref grammar/state profiles. |
| flexbox | `css.properties.flex.content` | Added eligible key | Existing authoring support: `baseline-november-09.flex.*` and Webref grammar/state profiles. |
| flexbox | `css.properties.justify-content.center` | Added eligible key | Existing authoring support: `baseline-november-09.justify-content.*` and Webref grammar/state profiles. |
| flexbox | `css.properties.justify-content.flex-end` | Added eligible key | Existing authoring support: `baseline-november-09.justify-content.*` and Webref grammar/state profiles. |
| flexbox | `css.properties.justify-content.flex-start` | Added eligible key | Existing authoring support: `baseline-november-09.justify-content.*` and Webref grammar/state profiles. |
| flexbox | `css.properties.justify-content.space-around` | Added eligible key | Existing authoring support: `baseline-november-09.justify-content.*` and Webref grammar/state profiles. |
| flexbox | `css.properties.justify-content.space-between` | Added eligible key | Existing authoring support: `baseline-november-09.justify-content.*` and Webref grammar/state profiles. |
| flexbox | `css.properties.justify-items.baseline` | Added eligible key | Existing authoring support: `baseline-november-09.justify-items.*` and Webref grammar/state profiles. |
| flexbox | `css.properties.justify-items.end` | Added eligible key | Existing authoring support: `baseline-november-09.justify-items.*` and Webref grammar/state profiles. |
| flexbox | `css.properties.justify-items.flex-end` | Added eligible key | Existing authoring support: `baseline-november-09.justify-items.*` and Webref grammar/state profiles. |
| flexbox | `css.properties.justify-items.flex-start` | Added eligible key | Existing authoring support: `baseline-november-09.justify-items.*` and Webref grammar/state profiles. |
| flexbox | `css.properties.justify-items.normal` | Added eligible key | Existing authoring support: `baseline-november-09.justify-items.*` and Webref grammar/state profiles. |
| flexbox | `css.properties.justify-items.self-end` | Added eligible key | Existing authoring support: `baseline-november-09.justify-items.*` and Webref grammar/state profiles. |
| flexbox | `css.properties.justify-items.self-start` | Added eligible key | Existing authoring support: `baseline-november-09.justify-items.*` and Webref grammar/state profiles. |
| flexbox | `css.properties.justify-items.start` | Added eligible key | Existing authoring support: `baseline-november-09.justify-items.*` and Webref grammar/state profiles. |
| flexbox | `css.properties.justify-items.stretch` | Added eligible key | Existing authoring support: `baseline-november-09.justify-items.*` and Webref grammar/state profiles. |
| flexbox | `css.properties.place-content.baseline` | Added eligible key | Existing authoring support: `baseline-november-09.place-content.*` and Webref grammar/state profiles. |
| flexbox | `css.properties.place-content.center` | Added eligible key | Existing authoring support: `baseline-november-09.place-content.*` and Webref grammar/state profiles. |
| flexbox | `css.properties.place-content.end` | Added eligible key | Existing authoring support: `baseline-november-09.place-content.*` and Webref grammar/state profiles. |
| flexbox | `css.properties.place-content.flex-end` | Added eligible key | Existing authoring support: `baseline-november-09.place-content.*` and Webref grammar/state profiles. |
| flexbox | `css.properties.place-content.flex-start` | Added eligible key | Existing authoring support: `baseline-november-09.place-content.*` and Webref grammar/state profiles. |
| flexbox | `css.properties.place-content.normal` | Added eligible key | Existing authoring support: `baseline-november-09.place-content.*` and Webref grammar/state profiles. |
| flexbox | `css.properties.place-content.space-around` | Added eligible key | Existing authoring support: `baseline-november-09.place-content.*` and Webref grammar/state profiles. |
| flexbox | `css.properties.place-content.space-between` | Added eligible key | Existing authoring support: `baseline-november-09.place-content.*` and Webref grammar/state profiles. |
| flexbox | `css.properties.place-content.space-evenly` | Added eligible key | Existing authoring support: `baseline-november-09.place-content.*` and Webref grammar/state profiles. |
| flexbox | `css.properties.place-content.start` | Added eligible key | Existing authoring support: `baseline-november-09.place-content.*` and Webref grammar/state profiles. |
| flexbox | `css.properties.place-content.stretch` | Added eligible key | Existing authoring support: `baseline-november-09.place-content.*` and Webref grammar/state profiles. |
| flexbox | `css.properties.place-items.baseline` | Added eligible key | Existing authoring support: `baseline-november-09.place-items.*` and Webref grammar/state profiles. |
| flexbox | `css.properties.place-items.center` | Added eligible key | Existing authoring support: `baseline-november-09.place-items.*` and Webref grammar/state profiles. |
| flexbox | `css.properties.place-items.end` | Added eligible key | Existing authoring support: `baseline-november-09.place-items.*` and Webref grammar/state profiles. |
| flexbox | `css.properties.place-items.flex-end` | Added eligible key | Existing authoring support: `baseline-november-09.place-items.*` and Webref grammar/state profiles. |
| flexbox | `css.properties.place-items.flex-start` | Added eligible key | Existing authoring support: `baseline-november-09.place-items.*` and Webref grammar/state profiles. |
| flexbox | `css.properties.place-items.normal` | Added eligible key | Existing authoring support: `baseline-november-09.place-items.*` and Webref grammar/state profiles. |
| flexbox | `css.properties.place-items.self-end` | Added eligible key | Existing authoring support: `baseline-november-09.place-items.*` and Webref grammar/state profiles. |
| flexbox | `css.properties.place-items.self-start` | Added eligible key | Existing authoring support: `baseline-november-09.place-items.*` and Webref grammar/state profiles. |
| flexbox | `css.properties.place-items.start` | Added eligible key | Existing authoring support: `baseline-november-09.place-items.*` and Webref grammar/state profiles. |
| flexbox | `css.properties.place-items.stretch` | Added eligible key | Existing authoring support: `baseline-november-09.place-items.*` and Webref grammar/state profiles. |
| flexbox | `css.properties.place-self.auto` | Added eligible key | Existing authoring support: `baseline-november-09.place-self.*` and Webref grammar/state profiles. |
| flexbox | `css.properties.place-self.baseline` | Added eligible key | Existing authoring support: `baseline-november-09.place-self.*` and Webref grammar/state profiles. |
| flexbox | `css.properties.place-self.center` | Added eligible key | Existing authoring support: `baseline-november-09.place-self.*` and Webref grammar/state profiles. |
| flexbox | `css.properties.place-self.end` | Added eligible key | Existing authoring support: `baseline-november-09.place-self.*` and Webref grammar/state profiles. |
| flexbox | `css.properties.place-self.flex-end` | Added eligible key | Existing authoring support: `baseline-november-09.place-self.*` and Webref grammar/state profiles. |
| flexbox | `css.properties.place-self.flex-start` | Added eligible key | Existing authoring support: `baseline-november-09.place-self.*` and Webref grammar/state profiles. |
| flexbox | `css.properties.place-self.normal` | Added eligible key | Existing authoring support: `baseline-november-09.place-self.*` and Webref grammar/state profiles. |
| flexbox | `css.properties.place-self.self-end` | Added eligible key | Existing authoring support: `baseline-november-09.place-self.*` and Webref grammar/state profiles. |
| flexbox | `css.properties.place-self.self-start` | Added eligible key | Existing authoring support: `baseline-november-09.place-self.*` and Webref grammar/state profiles. |
| flexbox | `css.properties.place-self.start` | Added eligible key | Existing authoring support: `baseline-november-09.place-self.*` and Webref grammar/state profiles. |
| flexbox | `css.properties.place-self.stretch` | Added eligible key | Existing authoring support: `baseline-november-09.place-self.*` and Webref grammar/state profiles. |
| grid | `css.properties.justify-self.baseline` | Added eligible key | Existing authoring support: `baseline-november-09.justify-self.*` and Webref grammar/state profiles. |
| grid | `css.properties.justify-self.center` | Added eligible key | Existing authoring support: `baseline-november-09.justify-self.*` and Webref grammar/state profiles. |
| grid | `css.properties.justify-self.end` | Added eligible key | Existing authoring support: `baseline-november-09.justify-self.*` and Webref grammar/state profiles. |
| grid | `css.properties.justify-self.flex-end` | Added eligible key | Existing authoring support: `baseline-november-09.justify-self.*` and Webref grammar/state profiles. |
| grid | `css.properties.justify-self.flex-start` | Added eligible key | Existing authoring support: `baseline-november-09.justify-self.*` and Webref grammar/state profiles. |
| grid | `css.properties.justify-self.self-end` | Added eligible key | Existing authoring support: `baseline-november-09.justify-self.*` and Webref grammar/state profiles. |
| grid | `css.properties.justify-self.self-start` | Added eligible key | Existing authoring support: `baseline-november-09.justify-self.*` and Webref grammar/state profiles. |
| grid | `css.properties.justify-self.start` | Added eligible key | Existing authoring support: `baseline-november-09.justify-self.*` and Webref grammar/state profiles. |
| scroll-driven-animations | `api.ScrollTimeline` | Added eligible key | DOM/evaluation outside scope: timeline execution and document/element association. |
| scroll-driven-animations | `api.ScrollTimeline.ScrollTimeline` | Added eligible key | DOM/evaluation outside scope: timeline execution and document/element association. |
| scroll-driven-animations | `api.ScrollTimeline.axis` | Added eligible key | DOM/evaluation outside scope: timeline execution and document/element association. |
| scroll-driven-animations | `api.ScrollTimeline.source` | Added eligible key | DOM/evaluation outside scope: timeline execution and document/element association. |
| scroll-driven-animations | `api.ViewTimeline` | Added eligible key | DOM/evaluation outside scope: timeline execution and document/element association. |
| scroll-driven-animations | `api.ViewTimeline.ViewTimeline` | Added eligible key | DOM/evaluation outside scope: timeline execution and document/element association. |
| scroll-driven-animations | `api.ViewTimeline.endOffset` | Added eligible key | DOM/evaluation outside scope: timeline execution and document/element association. |
| scroll-driven-animations | `api.ViewTimeline.startOffset` | Added eligible key | DOM/evaluation outside scope: timeline execution and document/element association. |
| scroll-driven-animations | `api.ViewTimeline.subject` | Added eligible key | DOM/evaluation outside scope: timeline execution and document/element association. |
| scroll-driven-animations | `css.properties.animation-range` | Added eligible key | Existing authoring support: `baseline-november-09.animation-range.*` and Webref grammar/state profiles. |
| scroll-driven-animations | `css.properties.animation-range-end` | Added eligible key | Existing authoring support: `baseline-november-09.animation-range-end.*` and Webref grammar/state profiles. |
| scroll-driven-animations | `css.properties.animation-range-end.normal` | Added eligible key | Existing authoring support: `baseline-november-09.animation-range-end.*` and Webref grammar/state profiles. |
| scroll-driven-animations | `css.properties.animation-range-start` | Added eligible key | Existing authoring support: `baseline-november-09.animation-range-start.*` and Webref grammar/state profiles. |
| scroll-driven-animations | `css.properties.animation-range-start.normal` | Added eligible key | Existing authoring support: `baseline-november-09.animation-range-start.*` and Webref grammar/state profiles. |
| scroll-driven-animations | `css.properties.animation-timeline` | Added eligible key | Existing authoring support: `baseline-november-09.animation-timeline.*` and Webref grammar/state profiles. |
| scroll-driven-animations | `css.properties.animation-timeline.scroll` | Added eligible key | Existing authoring support: `baseline-november-09.animation-timeline.*` and Webref grammar/state profiles. |
| scroll-driven-animations | `css.properties.animation-timeline.view` | Added eligible key | Existing authoring support: `baseline-november-09.animation-timeline.*` and Webref grammar/state profiles. |
| scroll-driven-animations | `css.properties.animation.animation-timeline_included` | Added deferred key | Not yet eligible: false status without an availability date; no Baseline claim. |
| scroll-driven-animations | `css.properties.scroll-timeline` | Added eligible key | Authoring correction: reject axis-before-name; `baseline-november-09.scroll-timeline.*`, shared November regression, and Webref profiles. |
| scroll-driven-animations | `css.properties.scroll-timeline-axis` | Added eligible key | Existing authoring support: `baseline-november-09.scroll-timeline-axis.*` and Webref grammar/state profiles. |
| scroll-driven-animations | `css.properties.scroll-timeline-axis.block` | Added eligible key | Existing authoring support: `baseline-november-09.scroll-timeline-axis.*` and Webref grammar/state profiles. |
| scroll-driven-animations | `css.properties.scroll-timeline-axis.inline` | Added eligible key | Existing authoring support: `baseline-november-09.scroll-timeline-axis.*` and Webref grammar/state profiles. |
| scroll-driven-animations | `css.properties.scroll-timeline-axis.x` | Added eligible key | Existing authoring support: `baseline-november-09.scroll-timeline-axis.*` and Webref grammar/state profiles. |
| scroll-driven-animations | `css.properties.scroll-timeline-axis.y` | Added eligible key | Existing authoring support: `baseline-november-09.scroll-timeline-axis.*` and Webref grammar/state profiles. |
| scroll-driven-animations | `css.properties.scroll-timeline-name` | Added eligible key | Existing authoring support: `baseline-november-09.scroll-timeline-name.*` and Webref grammar/state profiles. |
| scroll-driven-animations | `css.properties.timeline-scope` | Added eligible key | Existing authoring support: `baseline-november-09.timeline-scope.*` and Webref grammar/state profiles. |
| scroll-driven-animations | `css.properties.timeline-scope.all` | Added deferred key | Not yet eligible: false status without an availability date; no Baseline claim. |
| scroll-driven-animations | `css.properties.timeline-scope.none` | Added eligible key | Existing authoring support: `baseline-november-09.timeline-scope.*` and Webref grammar/state profiles. |
| scroll-driven-animations | `css.properties.view-timeline` | Added eligible key | Existing authoring support: `baseline-november-09.view-timeline.*` and Webref grammar/state profiles. |
| scroll-driven-animations | `css.properties.view-timeline-axis` | Added eligible key | Existing authoring support: `baseline-november-09.view-timeline-axis.*` and Webref grammar/state profiles. |
| scroll-driven-animations | `css.properties.view-timeline-axis.block` | Added eligible key | Existing authoring support: `baseline-november-09.view-timeline-axis.*` and Webref grammar/state profiles. |
| scroll-driven-animations | `css.properties.view-timeline-axis.inline` | Added eligible key | Existing authoring support: `baseline-november-09.view-timeline-axis.*` and Webref grammar/state profiles. |
| scroll-driven-animations | `css.properties.view-timeline-axis.x` | Added eligible key | Existing authoring support: `baseline-november-09.view-timeline-axis.*` and Webref grammar/state profiles. |
| scroll-driven-animations | `css.properties.view-timeline-axis.y` | Added eligible key | Existing authoring support: `baseline-november-09.view-timeline-axis.*` and Webref grammar/state profiles. |
| scroll-driven-animations | `css.properties.view-timeline-inset` | Added eligible key | Existing authoring support: `baseline-november-09.view-timeline-inset.*` and Webref grammar/state profiles. |
| scroll-driven-animations | `css.properties.view-timeline-inset.auto` | Added eligible key | Existing authoring support: `baseline-november-09.view-timeline-inset.*` and Webref grammar/state profiles. |
| scroll-driven-animations | `css.properties.view-timeline-name` | Added eligible key | Existing authoring support: `baseline-november-09.view-timeline-name.*` and Webref grammar/state profiles. |
| svg | `svg.elements.circle.pathLength` | Added eligible key | DOM/rendering outside scope: SVG attribute or view-fragment behavior. |
| svg | `svg.elements.ellipse.pathLength` | Added eligible key | DOM/rendering outside scope: SVG attribute or view-fragment behavior. |
| svg | `svg.elements.line.pathLength` | Added eligible key | DOM/rendering outside scope: SVG attribute or view-fragment behavior. |
| svg | `svg.elements.path.pathLength` | Added eligible key | DOM/rendering outside scope: SVG attribute or view-fragment behavior. |
| svg | `svg.elements.polygon.pathLength` | Added eligible key | DOM/rendering outside scope: SVG attribute or view-fragment behavior. |
| svg | `svg.elements.polyline.pathLength` | Added eligible key | DOM/rendering outside scope: SVG attribute or view-fragment behavior. |
| svg | `svg.elements.rect.pathLength` | Added eligible key | DOM/rendering outside scope: SVG attribute or view-fragment behavior. |
| svg | `svg.elements.view.svgView` | Added eligible key | DOM/rendering outside scope: SVG attribute or view-fragment behavior. |
| svg | `svg.elements.view.svgView.preserveAspectRatio_parameter` | Added eligible key | DOM/rendering outside scope: SVG attribute or view-fragment behavior. |
| svg | `svg.elements.view.svgView.transform_parameter` | Added deferred key | Not yet eligible: false status without an availability date; no Baseline claim. |
| svg | `svg.elements.view.svgView.viewBox_parameter` | Added eligible key | DOM/rendering outside scope: SVG attribute or view-fragment behavior. |

## Implemented authoring correction

The fresh published 0.4.1 native and WASM installs accepted
`scroll-timeline: inline --progress`; Chromium 151 rejected it in support queries
and left authored state unchanged. The specification's
[name-then-optional-axis grammar](https://drafts.csswg.org/scroll-animations-1/#scroll-timeline-shorthand)
confirms this is invalid, rather than an oracle predating new syntax. The shared
`sheetom-core` shorthand parser now validates that order in every comma-separated
entry. An invalid later entry rejects the entire write. The regression covers
parsing, insertRule, setProperty, the IDL property setter, cssText, both forms of
CSS.supports, priority retention, valid lists and serialization/reparse.

The correction changes public invalid-input behavior and receives a patch
Changeset. Engine revision advances to `lightningcss-1.33.0-c6a0c3ce-sheetom.83` and
engine ABI identity is regenerated. No vendor source is changed. Existing dependency Changesets remain owned by release automation.
Other newly eligible syntax was already implemented and receives executable
evidence without a fabricated new-support version bump.

## Measured serialization behavior

Chromium 151 and published SheetOM 0.4.1 expose the same declaration order after
writing an `animation` shorthand over pre-existing `animation-range` and
`animation-timeline` declarations. Serializing and reparsing that shorthand moves
reset-only longhands to their shorthand expansion order in both implementations.
The shared reset contract therefore compares the complete name/value/priority set
for that sequence, checks rule topology and stable serialization, and independently
asserts every reset value. Individual declaration and initial shorthand round trips
continue to require exact declaration order using the maintained roundtrip helper.
No assertions or existing corpus are removed.
[ADR 0193](adr/0193-validate-scroll-timeline-authoring-state.md) records the
shared grammar and the measured reset-order comparison policy.

## Validation and delivery

The initial fresh registry install of `sheetom@0.4.1` and `@sheetom/wasm@0.4.1`
reproduced the axis-before-name defect on both backends. The initial 629-probe
browser comparison reported four mismatches, all state/support observations of
that one invalid shorthand. The added focused regression fails on 0.4.1 as
expected. Registry signatures and attestations verified for all three packages
installed on this host. That baseline failure is retained as evidence, not
presented as successful validation of the correction.

After the correction, all **633 browser probes**, **12,469 support checks** and
**264 escaping checks per backend** pass with zero mismatches. The shared runner
passes **30 modern CSS contracts per backend**, including all three November
regressions and depth 256/1024/4000 checks. Both native and WASM Webref ratchets
pass **11,815 checks** over 667 properties, 370 grammar profiles and 8,575
branches, with zero acceptance, state, item-order, atomicity or reparse mismatches.
The original test sources, assertions and Webref corpus remain in the repository;
installed-package validation uses the maintained `check-installed-packages.ts`
runner with an explicit fresh package prefix and durable report directory.

Required local gates pass: sequential `native:build`, `build` and `wasm:build`;
`css:target:check`; both Webref ratchets; `check` (303 unit tests, generated files,
documentation, runtime artifacts and a fresh package tarball smoke test);
`native:check` (workspace formatting/Clippy/tests and all vendored parser suites);
`wasm:check`; `wasm:test`; and all 52 automation-script tests. The generator also
reproduces the target against the downloaded source bytes.

Final builds and broad gates use the repository's Node 24.12.0 pin, npm 11.16.0,
Rust 1.88.0 and wasm-bindgen 0.2.127, with the pinned Playwright browser cohort.
The node exposes three CPUs and approximately 18 GiB RAM, so builds use at most
three Cargo workers, with two while an independent single-worker optimizer runs.
Final native and WASM artifact builds run sequentially. WASM is **4,598,255 raw
bytes** and **1,336,550 gzip bytes**, within the unchanged repository budgets.

A patch Changeset records the public grammar correction; no package version is
manually assigned. The existing dependency Changesets remain in place for the
same automated release cohort. Exact implementation/release PR heads, merge SHAs,
workflow IDs, the automation-selected candidate and publication observations are
recorded in the durable delivery checkpoint and final run evidence. Local success
and the existing 0.4.1 publication do not replace the implementation CI, main CI,
final Changesets head, publication workflow or cold published-package gates.
