# CSS Baseline audit — 21 September 2026

The official npm registry's current `web-features` is **3.39.0**, revision
`a9cc987771757d5604fa37cb76f40f5e9b3808f1`. The downloaded
[source archive](https://registry.npmjs.org/web-features/-/web-features-3.39.0.tgz)
contains `data.json` with SHA-256
`b9e65236833bf78451fbd05ac4c587b643247a6d5ee4f0e8e28daaa13b7247cd`.
The cutoff is **2026-09-21**. Availability comes from
[WebDX](https://github.com/web-platform-dx/web-features); executable authoring
coverage is a separate gate.

## Complete target delta

Relative to the 14 September target, nine compatibility records become eligible,
seven are withdrawn, and nine previously non-Baseline records disappear upstream.
Three families enter and the media-pseudos family leaves: 330 families, 4,219
unique keys, 3,757 eligible records and 462 deferred records remain (399 not
Baseline, 63 after cutoff, zero undated). There are no
changed specification links in retained families. Coverage maps 2,649 authoring
records to 598 surfaces and excludes 1,108 DOM/evaluation records.

| Branch | Disposition |
| --- | --- |
| `css.types.color.alpha` | Partial existing support; unresolved origins were rejected. Implemented typed relative alpha, including currentColor, system colors and nested colors. |
| `css.types.color.light-dark.image_value` | Missing image-valued function; implemented typed light/dark image-or-none branches. |
| `css.properties.position-anchor` and `.auto`, `.none`, `.normal` | Already supported; executable checks include the three keywords and dashed identifiers. |
| `css.properties.overflow-anchor` and `.auto`, `.none` | Already supported; authoring checks cover both keywords. Scroll anchoring itself is outside scope. |
| `css.selectors.buffering`, `.muted`, `.paused`, `.playing`, `.seeking`, `.stalled`, `.volume-locked` | Withdrawn: current data has baseline false and no Chrome support. Retain existing authoring support and Firefox probes without a Baseline claim. |

Removed deferred records: `css.types.attr.type_function.transform-function`,
`css.types.attr.type_function.url`, `css.properties.display.contents.focusable_elements`,
`css.selectors.empty.matches_whitespace`, `css.properties.font-variant.greek_accented_characters`,
`css.properties.font-variant.uppercase_eszett`, `css.types.basic-shape.path.shape-outside`,
`css.properties.text-autospace.punctuation`, and `css.properties.text-autospace.replace`.
None was eligible; removal does not remove an authoring API. Undated or false
statuses are never silently treated as Baseline.

## Reproduction and implementation

Fresh installed native and WASM 0.2.1 packages both reject image light-dark and
`alpha(from currentColor / .5)`. Both browsers in the existing pinned cohort
accept image light-dark and the anchor properties. Both reject alpha colors;
Chromium 153.0.8010.12 from the new exact `playwright-baseline` 1.63.0 alias accepts
them and supplies explicit alpha probes. Existing browsers remain unchanged.

The shared relative-color AST now retains the origin and optional alpha
expression without resolving it. The image AST retains two validated branches.
Tests cover valid/invalid grammar, support queries, property contexts, important
state, invalid-write atomicity, serialization and semantic reparsing through
both public backends. The browser corpus includes new feature-specific probes;
existing Webref checks remain the anchor-property grammar evidence.

Primary contracts: [relative alpha](https://drafts.csswg.org/css-color-5/#relative-alpha),
[scheme images](https://drafts.csswg.org/css-color-5/#typedef-light-dark-image),
[position-anchor](https://drafts.csswg.org/css-anchor-position-1/#position-anchor),
and [overflow-anchor](https://drafts.csswg.org/css-scroll-anchoring-1/#exclusion-api).
[ADR 191](adr/0191-retain-relative-alpha-and-scheme-images.md) records the shared
AST and browser decisions. Historical September gap groups remain covered by
shared regression tests. Cascade, computed values, matching, DOM associations,
layout and mixin expansion remain outside scope. Pinned mixin/function support
remains experimental, separate from Baseline.

## Validation

The source archive matches npm's SHA-512 integrity, and target and coverage
regeneration reproduce from the pinned data. Both finalized backends pass 24
shared modern CSS contracts and depth checks through 4,000. Browser comparisons
pass 314 probes, 11,956 support queries and 264 escape checks per backend with
zero mismatches. Native and WASM Webref ratchets report zero mismatches across
acceptance, observable state, declaration text, indexed items, mutation atomicity
and reparsing.

`npm run check` passes 295 unit tests plus generated manifests, documentation,
runtime artifacts and package installation checks. Native checks pass formatting,
Clippy, 237 workspace tests and the vendor suites (including 189 Lightning CSS
tests). WASM checks, the WASM backend suite and all 52 CI script tests pass.
Native and WASM artifacts were built sequentially with the pinned toolchains.
WASM remains within the unchanged budgets: 4,596,442 raw bytes and 1,335,395 gzip
bytes. Removing the obsolete eager-alpha evaluator avoided raising those limits.
The platform, browser, performance and release matrices remain CI gates.
