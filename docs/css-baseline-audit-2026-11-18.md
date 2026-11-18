# CSS Baseline audit for 18 November 2026

The cutoff is **2026-11-18**. The official npm registry reported **web-features
3.46.0**, source revision `8af8ff8d71f0b81bb8d637060ada22cf3c903112`.
The downloaded archive matches npm integrity
`sha512-UXJz08ycij5sVoOL97e8kvzhnC/3kbo3SCGC1j84G5pqx7XXStbqPVNVli+BHwo8Gcb6SquKUGk5TJYsl0oRYw==`.
Its `data.json` bytes have SHA-256
`38d73859f967972a3170a774ba5fe2669877ea6b7ffd53a14d7b53daf1366a11`.
The SheetOM audit started from `8c04d6e2f00037b62277d6efc470758e5d5dbca5`.
[WebDX revision](https://github.com/web-platform-dx/web-features/commit/8af8ff8d71f0b81bb8d637060ada22cf3c903112)
· [npm archive](https://registry.npmjs.org/web-features/-/web-features-3.46.0.tgz).

## Inventory delta

Compared with the [9 November audit](css-baseline-audit-2026-11-09.md), the
target adds 29 Widely Available keys and one undated deferred key, and drops two
previously deferred keys. No retained family or branch changes availability
status, dates, or specification links. No Newly Available key enters the target.
The refreshed snapshot has 334 families, 4,494 unique keys, 4,015 eligible
records, and 479 deferred records. Of the eligible records, 2,883 refer to 610
authoring surfaces and 1,132 concern DOM association or evaluation. Of the
deferred records, 416 lack Baseline status and 63 have after-cutoff dates.

The table accounts for all new eligible keys. Each property row refers to the
declaration probes with the corresponding `baseline-november-18.<property>.*`
IDs in the [browser corpus](../compatibility/css-authoring-probes.json). The
color branch uses the `baseline-november-18.color.*` probes. The probes
compare support queries, authored declarations including shorthand longhands
and priority, rejection of invalid grammar, atomicity, removal state, and
semantic serialization/reparse on both backends.

| Family | Compatibility records | Disposition |
| --- | --- | --- |
| Filters | `css.properties.filter.none` | Existing authoring support. [Filter Effects](https://drafts.csswg.org/filter-effects-1/#propdef-filter). |
| Flexbox | `css.properties.flex-flow.column`, `.column-reverse`, `.nowrap`, `.row`, `.row-reverse`, `.wrap`, `.wrap-reverse` | Existing shorthand authoring and longhand expansion. [CSS Flexible Box Layout](https://drafts.csswg.org/css-flexbox-1/#flex-flow-property). |
| Flexbox | `css.properties.flex.auto`, `.fit-content`, `.max-content`, `.min-content` | Existing flex-basis keyword support through the shorthand. [CSS Flexible Box Layout](https://drafts.csswg.org/css-flexbox-1/#flex-property) and [CSS Sizing](https://drafts.csswg.org/css-sizing-3/#sizing-values). |
| Grid | `css.properties.grid-area.auto`, `css.properties.grid-row.auto`, `css.properties.grid-column.auto`, `css.properties.grid-row-start.auto`, `css.properties.grid-row-end.auto`, `css.properties.grid-column-start.auto`, `css.properties.grid-column-end.auto` | Existing grid placement authoring and longhand expansion. [CSS Grid Layout](https://drafts.csswg.org/css-grid-2/#placement-shorthands). |
| Grid | `css.properties.grid-auto-columns.auto`, `.max-content`, `.min-content`; `css.properties.grid-auto-rows.auto`, `.max-content`, `.min-content` | Existing implicit track grammar. [CSS Grid Layout](https://drafts.csswg.org/css-grid-2/#auto-tracks). |
| Grid | `css.properties.grid.none` | Existing shorthand and reset state. [CSS Grid Layout](https://drafts.csswg.org/css-grid-2/#grid-shorthand). |
| HSL | `css.types.color.hsla` | Authoring correction for legacy syntax and alpha serialization. [CSS Color](https://drafts.csswg.org/css-color-4/#the-hsl-notation). |
| Masks | `css.properties.mask.none`, `css.properties.mask-image.none` | Existing shorthand expansion and longhand authoring. [CSS Masking](https://drafts.csswg.org/css-masking-1/#positioned-masks). |

WebDX uses [Browser Compatibility Data 8.1.9](https://github.com/mdn/browser-compat-data/tree/v8.1.9).
The new `flex-basis.stretch` key is undated with `baseline: false`. Its
compatibility data records Chromium 138 and Safari 27, with Firefox support
unavailable. The two removed `overlay` keys for `overflow-block` and
`overflow-inline` had been undated and ineligible. Their removal from WebDX's
grouping makes no change to the existing property grammar. Three additional
CSS families appear outside the cutoff target. Their keys for
`:navigation-source`, `scroll-snap-type: pair`, and the `polygon()` `round`
keyword have `baseline: false` and no eligibility dates. WebDX also updates
browser-version metadata for five retained ineligible branches. Chromium 157
is recorded for `position-visibility: anchor-valid` and `anchor-visible`.
Chromium 108 corrects the version for `break-after: avoid-page`,
`break-after: avoid-page` in paged contexts, and `break-before: avoid-page`.
All five retain `baseline: false` and no eligibility date.
[Flex basis](https://github.com/mdn/browser-compat-data/blob/v8.1.9/css/properties/flex-basis.json)
· [CSS Overflow](https://drafts.csswg.org/css-overflow-3/#valdef-overflow-overlay).

The earlier September, October, and 9 November authoring contracts remain part
of the shared test run. Pinned draft mixin and function evidence remains
separate. Grid and flex layout, filter and mask rendering, and computed color
evaluation remain outside the authoring contract.

## HSL authoring correction

At the starting revision, SheetOM accepted bare numeric saturation and
lightness in legacy comma syntax, accepted a legacy `none` alpha, exposed a
50% alpha as `0.502`, and could expose the author's HSL function unchanged
for an opaque color. Chromium 151.0.7922.34, Firefox 153.0 and Chromium
153.0.8010.12 and WebKit 26.5 rejected the invalid grammar and exposed `0.5` for the 50%
case, `0.52` for the measured 52.3% case, and `rgb(0, 128, 0)` for the
opaque case. Space syntax still accepts numeric channels. The browser
comparison checks the state after shorthand removal independently of the
return string. Chromium 151 returned an empty string for the measured
shorthand removals, and SheetOM returned the previous shorthand value.

[ADR 0194](adr/0194-validate-legacy-hsl-and-alpha.md) documents the shared
parser and serializer decision. The native and WASM facades execute the same
HSL tests in [the November suite](../tests/baseline-november.test.ts), including
legacy and modern forms, percentages calculated with `calc()`, escaped
function names, invalid mutations, and safe serialization. The engine
revision advances to `lightningcss-1.33.0-c6a0c3ce-sheetom.84`. The Changeset
classifies the corrected behavior as a patch.

Local validation passed 664 browser probes, 12,500 support comparisons and
264 escape checks per backend, with zero mismatches. Native and WASM each
passed 31 shared modern CSS contracts. The maintained installed-package runner
also passed from a fresh prefix containing the local tarballs, including 11,815
Webref declaration cases per backend across 667 properties with zero
mismatches.
