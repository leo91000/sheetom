# CSS Baseline audit — 19 October 2026

The cutoff is **2026-10-19**. The official npm registry reports **web-features
3.43.0**, source revision `3c7e5ff9ed136e4fcf76b4ee54fdffab7c31ab69`.
The [downloaded archive](https://registry.npmjs.org/web-features/-/web-features-3.43.0.tgz)
matches the registry's SHA-512 integrity. Its actual `data.json` bytes have SHA-256
`0c093e865d7277a787267e1e0c02a88c175c8a81bde3572e4c707f1c3c2df9d9`.
[WebDX](https://github.com/web-platform-dx/web-features) supplies eligibility;
executable evidence separately establishes authored CSSOM support.
The baseline repository revision is `445ec7bc143b844a36126d71d852eea7654c743a`.

## Complete inventory delta

Relative to the [12 October audit](css-baseline-audit-2026-10-12.md), this refresh
adds one family (`revert-rule`), 78 eligible and 18 deferred key records, and
removes one eligible and two deferred records. Two retained font-feature-settings
records have corrected dates; transition-behavior loses eligibility. No family
is removed, and no retained family changes its name or specification links.
The generated target contains 333 families, 4,350 unique compatibility keys,
3,874 eligible records and 476 deferred records. The evidence map assigns
2,761 authoring records to 598 surfaces and excludes 1,113 DOM/evaluation records.
Selection includes individual low/high branches dated at or before the cutoff,
including branches in families that are not wholly Baseline.

Every added eligible record is listed below. Property roots link to the executable
Webref profiles in the generated coverage map, supplemented by the
`baseline-october-19.*` probes. Root mappings alone are not support evidence.

| Added eligible records | Disposition |
| --- | --- |
| `css.properties.animation.paused`, `css.properties.animation.running` | Already supported by executable property/type probes added in this audit and the existing Webref grammar profile. Rendering and evaluation are outside scope. |
| `css.types.line-width` | Already supported by executable property/type probes added in this audit and the existing Webref grammar profile. Rendering and evaluation are outside scope. |
| `css.properties.content.content_replacement` | Already supported by executable property/type probes added in this audit and the existing Webref grammar profile. Rendering and evaluation are outside scope. |
| `css.types.color.currentcolor` | Already supported by executable property/type probes added in this audit and the existing Webref grammar profile. Rendering and evaluation are outside scope. |
| `css.properties.align-content.center`, `css.properties.align-content.space-around`, `css.properties.align-content.space-between`, `css.properties.align-items.normal`, `css.properties.align-items.self-end`, `css.properties.align-items.self-start`, `css.properties.align-items.stretch`, `css.properties.align-self.center`, `css.properties.align-self.self-end`, `css.properties.align-self.self-start` | Already supported by executable property/type probes added in this audit and the existing Webref grammar profile. Rendering and evaluation are outside scope. |
| `css.properties.font-variant.full-width`, `css.properties.font-variant.jis04`, `css.properties.font-variant.jis78`, `css.properties.font-variant.jis83`, `css.properties.font-variant.jis90`, `css.properties.font-variant.proportional-width`, `css.properties.font-variant.ruby`, `css.properties.font-variant.simplified`, `css.properties.font-variant.traditional` | Already supported by executable property/type probes added in this audit and the existing Webref grammar profile. Rendering and evaluation are outside scope. |
| `css.properties.font-variant.common-ligatures`, `css.properties.font-variant.contextual`, `css.properties.font-variant.discretionary-ligatures`, `css.properties.font-variant.historical-ligatures`, `css.properties.font-variant.no-common-ligatures`, `css.properties.font-variant.no-contextual`, `css.properties.font-variant.no-discretionary-ligatures`, `css.properties.font-variant.no-historical-ligatures` | Already supported by executable property/type probes added in this audit and the existing Webref grammar profile. Rendering and evaluation are outside scope. |
| `css.properties.font-variant.diagonal-fractions`, `css.properties.font-variant.lining-nums`, `css.properties.font-variant.oldstyle-nums`, `css.properties.font-variant.ordinal`, `css.properties.font-variant.proportional-nums`, `css.properties.font-variant.slashed-zero`, `css.properties.font-variant.stacked-fractions`, `css.properties.font-variant.tabular-nums` | Already supported by executable property/type probes added in this audit and the existing Webref grammar profile. Rendering and evaluation are outside scope. |
| `css.properties.mask-position.bottom`, `css.properties.mask-position.center`, `css.properties.mask-position.left`, `css.properties.mask-position.right`, `css.properties.mask-position.top`, `css.properties.mask-repeat.no-repeat`, `css.properties.mask-repeat.repeat`, `css.properties.mask-repeat.repeat-x`, `css.properties.mask-repeat.repeat-y`, `css.properties.mask-repeat.round`, `css.properties.mask-repeat.space`, `css.properties.mask-size.auto`, `css.properties.mask-size.contain`, `css.properties.mask-size.cover`, `css.properties.mask.add`, `css.properties.mask.alpha`, `css.properties.mask.bottom`, `css.properties.mask.center`, `css.properties.mask.exclude`, `css.properties.mask.intersect`, `css.properties.mask.left`, `css.properties.mask.match-source`, `css.properties.mask.no-repeat`, `css.properties.mask.repeat`, `css.properties.mask.repeat-x`, `css.properties.mask.repeat-y`, `css.properties.mask.right`, `css.properties.mask.round`, `css.properties.mask.space`, `css.properties.mask.subtract`, `css.properties.mask.top` | Existing grammar and browser state are supported. New mask/mask-position probes find and fix safe serialization of omitted images and position keywords. Painting and geometry remain outside scope. |
| `css.types.global_keywords.revert-rule` | Existing typed CSS-wide parsing is supported. The new all mutation probe finds and fixes replacement of earlier affected declarations. Cascade rollback evaluation is outside scope. |
| `css.properties.vector-effect.non-scaling-stroke`, `css.properties.vector-effect.none` | Already supported by executable property/type probes added in this audit and the existing Webref grammar profile. Rendering and evaluation are outside scope. |
| `css.properties.backface-visibility.hidden`, `css.properties.backface-visibility.visible`, `css.properties.transform-style.flat`, `css.properties.transform-style.preserve-3d` | Already supported by executable property/type probes added in this audit and the existing Webref grammar profile. Rendering and evaluation are outside scope. |

`css.properties.content.element_replacement` is renamed to
`css.properties.content.content_replacement`; authored image replacement syntax
is retained. Font-feature-settings and its `.normal` branch move from 2017-04-05
to 2016-03-21, remaining Widely Available. Explicit normal/tagged font probes
recheck both spellings.

`css.properties.transition-behavior` changes from low (2024-08-06) to not Baseline
with no availability date. It remains in the eligible transitions family's
inventory as deferred; existing authoring support is retained without a Baseline
claim. The two already-deferred accentcolor/accentcolortext system-color records
leave the system-color family. This is no newly withdrawn runtime syntax.

The 18 newly deferred mask branches are mask-clip's border-box, content-box,
fill-box, no-clip, padding-box, stroke-box and view-box; mask-origin's border-box,
content-box and padding-box; and mask's border-box, content-box, fill-box,
luminance, no-clip, padding-box, stroke-box and view-box. All have baseline false
and no availability date; none is promoted using its containing family's status.
The resulting deferred inventory has 413 not-Baseline and 63 after-cutoff
records, with no missing status objects; absent dates are never invented. Pinned mixin/function experiments and media-selector status
withdrawals remain separate from Baseline claims. The four September authoring
gap groups remain closed by the [September implementation](css-september-2026-parity-evidence.md).

## Reproduced gaps and implementation

A fresh registry installation of 0.4.0, selected with the maintained installed
package runner, passes the existing shared modern contracts but fails 40 new
native/WASM authoring comparisons. Thirty-eight are semantic roundtrip state
differences for mask/mask-position; two are the all reset mutation. Browser state
and valid syntax acceptance otherwise agree on the new probes.

Safe mask serialization now synthesizes from the owned longhand state, preserving
position keywords and the distinction between an omitted image (observable
`initial`) and an explicit `none`. An otherwise empty omitted-image layer uses
`add`, a valid default component, so reparsing retains the same longhand state.
Explicit image layers remain explicit. Safe mask-position serialization retains
its independently validated axis values. Observable cssText still follows the
browser's shorthand projection; safe output independently preserves reparsable
state. Existing scalar canonicalization, topology, indexed declarations and
priority assertions remain in force.

An all setter now removes affected earlier declarations only after grammar,
priority and resource validation succeeds, including earlier important writes.
Custom properties, direction and unicode-bidi survive, matching Chromium.
Affected unoverridden property getters expose the compact all value and priority,
including shorthand queries.
This correction concerns setter transactions; it does not claim exhaustive
browser equivalence for every all/longhand interleaving or browser-specific
enumeration strategy. Chromium compresses all, while Firefox enumerates expanded
longhands. The audit uses the repository's Chromium authoring precedence.

The shared regression suite adds reset/invalid-write cases for all six CSS-wide
keywords, preserved exceptions, and single/multiple mask layers. The same test
source executes through native and WASM in `test-modern-css-backends.ts`.
The browser corpus adds 87 declaration probes, including revert-rule on typed,
shorthand, all and custom properties. Existing support-query and mutation checks
and the Webref positive/negative corpus are retained without relaxed assertions.
This is a runtime correction, so a patch Changeset requests the existing fixed
package cohort; repository automation owns the eventual version.

Primary contracts: [rule rollback](https://drafts.csswg.org/css-cascade-5/#revert-rule-keyword),
[masking](https://drafts.csswg.org/css-masking-1/),
[font variants](https://drafts.csswg.org/css-fonts-4/#font-variant-prop),
[box alignment](https://drafts.csswg.org/css-align-3/),
[line widths](https://drafts.csswg.org/css-backgrounds-3/#line-width),
[currentcolor](https://drafts.csswg.org/css-color-4/#currentcolor-color),
[content](https://drafts.csswg.org/css-content-3/#content-property),
and [transforms](https://drafts.csswg.org/css-transforms-2/).
Cascade, computed styles, selector matching, layout, painting, event execution
and mixin expansion remain outside scope.

## Validation and delivery

Validation results and exact delivery checkpoints are recorded below as gates
complete. The cold 0.4.0 failure evidence is retained separately from the eventual
published-package result. Dependencies use npm 11.16.0 with dev dependencies,
Node 24.12.0, Rust 1.88.0 and wasm-bindgen 0.2.127. The node exposes 12 CPUs and
approximately 31 GiB RAM; Cargo uses six workers with sequential native/WASM
builds. Existing Playwright consumer and pinned oracle binaries are installed.
Source-backed target regeneration, coverage regeneration, script typechecking and
all 52 CI-script tests pass.

Both rebuilt backends pass 464 browser authoring probes, 12,125 support queries
and 264 escape checks per backend with zero mismatches. The oracles are Chromium
151.0.7922.34, Firefox 153.0 and the existing Baseline Chromium 153.0.8010.12.
Each facade passes 27 shared modern CSS contracts and depth checks through 4,000.
Each Webref ratchet checks 667 properties, 370 profiles and 8,415 branches through
11,640 cases, with zero acceptance, observable-state, declaration-text,
indexed-item, atomicity or reparse mismatches. `npm run check` passes, including
299 unit tests, typechecking, documentation, conformance, runtime and package
checks. The compatibility-report test's expected engine revision advances with
the engine to `lightningcss-1.33.0-c6a0c3ce-sheetom.82`; historical release reports
remain unchanged.

Final checks after the getter correction also pass: `npm run css:target:check`,
both Webref ratchets, `npm run check`, `npm run native:core-check`,
`npm run wasm:check`, `npm run wasm:test`, and 52 CI-script tests. The unchanged
vendored engines passed `npm run native:vendor-check` as part of the earlier
`native:check`; no vendor inputs changed afterward. The rebuilt WASM artifact
remains within the maintained size budgets. Full PR/main/release CI and cold
published-package validation remain delivery gates; they are not inferred from
these local results.
