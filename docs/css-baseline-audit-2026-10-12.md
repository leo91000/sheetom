# CSS Baseline audit — 12 October 2026

The official npm registry's current `web-features` release is **3.42.0**, revision
`fab686752db54ca26f9b55c99181ab2485fda0c2`. The downloaded
[source archive](https://registry.npmjs.org/web-features/-/web-features-3.42.0.tgz)
matches npm's SHA-512 integrity. Its actual `data.json` bytes have SHA-256
`4eb8992b18f0037182be93a1088b650bece21862bc6697e83b76c4726faa4728`.
The cutoff is **2026-10-12**. [WebDX](https://github.com/web-platform-dx/web-features)
records availability; executable authoring evidence is assessed separately.

## Complete target delta

Relative to the [5 October audit](css-baseline-audit-2026-10-05.md), the target
adds one family, 39 eligible key records and 20 deferred records, and removes
27 eligible records and 21 deferred records. Most changes rename or split BCD
keys. No retained key changes status or date, no existing family changes its
name or specification links, and no family is removed. The target now contains
332 families, 4,257 unique keys, 3,798 eligible records and 459 deferred records.
Coverage assigns 2,685 authoring records to 598 surfaces and excludes 1,113
DOM/evaluation records. Deferred records comprise 396 not-Baseline and 63
after-cutoff entries; there are no undated entries without a status object.
Not-Baseline entries with null availability dates remain ineligible.

The table accounts for all 39 added eligible records. Suffixes in each row are
relative to the stated compatibility root.

| Added eligible compatibility records | Disposition and executable evidence |
| --- | --- |
| `css.properties.align-self.context_position_absolute` | Renames `position_absolute_context`. Authored alignment is already supported by Webref profile `webref-profile.2a2c754f3ebde0ca` and the new `baseline-october-12.align-self.*` probes; absolute-position layout is outside scope. |
| `css.properties.align-content.context_block` | Renames `block_context`. Authored alignment is already supported by Webref profile `webref-profile.a7dc1b92d6e164be` and the new `baseline-october-12.align-content.*` probes; block layout is outside scope. |
| `css.properties.align-content`: `.baseline`, `.context_flex`, `.end`, `.first_baseline`, `.safe`, `.space-evenly`, `.start`, `.stretch`, `.unsafe` | Reorganizes the old `.flex_context` subtree, splitting `.start_end` and `.safe_unsafe`. Already supported by the align-content profile and explicit probes for each authored value, including `safe center` and `unsafe center`. |
| `css.properties.align-items`: `.baseline`, `.center`, `.context_flex`, `.context_flex.end_in_flex`, `.context_flex.start_in_flex`, `.end`, `.first_baseline`, `.last_baseline`, `.safe`, `.start`, `.unsafe` | Reorganizes the old `.flex_context` subtree and `.grid_context.start_end`. Already supported by Webref profile `webref-profile.44693ee1e7bd1a99` and explicit align-items probes. Context-specific layout effects are outside scope. |
| `css.properties.align-self`: `.baseline`, `.context_flex`, `.context_flex.stretch_in_flex`, `.end`, `.first_baseline`, `.last_baseline`, `.safe`, `.start`, `.unsafe` | Reorganizes the old `.flex_context` subtree, splitting `.start_end` and `.safe_unsafe`. Already supported by the align-self profile and explicit probes. Context-specific layout effects are outside scope. |
| `css.properties.align-content.context_grid`, `css.properties.align-items.context_grid`, `css.properties.align-self.context_grid` | Rename the three `.grid_context` keys. The same alignment profiles and probes cover authored state; grid layout is outside scope. |
| `css.properties.border-image-source.none` | Already supported by Webref profile `webref-profile.225b467b0a16657d` and explicit `baseline-october-12.border-image-source.none` probe. |
| `css.properties.columns.auto` | Already supported by Webref profile `webref-profile.05014b0208c966ca` and explicit `baseline-october-12.columns.auto` probe, including observable shorthand state. |
| `css.properties.color-interpolation.linearRGB` | Already supported by Webref profile `webref-profile.f0b2d8082d1db3c3` and explicit `baseline-october-12.color-interpolation.linearRGB` probe. Color evaluation and painting are outside scope. |
| `api.AnimationEvent.animation`, `api.TransitionEvent.animation` | New family `animations-css-animation-accessors`, Newly Available since 2026-09-14. Outside scope: event objects and live animation execution, not authored CSSOM state. |

All removed alignment keys are accounted for by the renames, subtree flattening
and split keywords above: the two block/absolute context keys, 20 eligible flex
subkeys, and four eligible grid subkeys. This is inventory reorganization, not
withdrawal of alignment syntax. The remaining removed eligible record is
`css.types.color.system-color.accentcolor_accentcolortext`. Upstream replaces it
with separate `.accentcolor` and `.accentcolortext` records, both not Baseline
and without availability dates. Their previous July availability claim is
withdrawn; existing authoring support is retained without calling it Baseline.

All deferred changes are also accounted for: 16 `css.at-rules.page` margin-box
keys acquire an `at_` prefix; `css.properties.align-content.multicol_context`
becomes `.context_multicol`; `.align-content.flex_context.last_baseline` becomes
`.align-content.last_baseline`; all remain not Baseline. Three not-Baseline
`api.CSSPseudoElement` records (the interface, `.element`, and `.type`) leave the
CSSOM family. The two accent system-color records described above are newly
deferred. No absent date is assigned eligibility.

The 29 added declaration probes exercise important authored values, invalid-write
atomicity, public state, rule attachment/detachment, support queries and semantic
serialization/reparse on both facades against the pinned Chromium oracle.
Existing Webref profiles additionally check positive and negative grammar,
indexed declarations and mutation state. A mapping or arbitrary-token acceptance
is not used as proof. No previously unresolved in-scope authoring gap is hidden
or reclassified. The [September implementation evidence](css-september-2026-parity-evidence.md)
remains applicable; pinned mixin/function experiments and withdrawn selector
statuses remain separate from Baseline claims.

Primary contracts are [box alignment](https://drafts.csswg.org/css-align-3/),
[absolute positioning](https://drafts.csswg.org/css-position-3/#abspos-insets),
[border image sources](https://drafts.csswg.org/css-backgrounds-3/#border-image-source),
[columns](https://drafts.csswg.org/css-multicol-2/#propdef-columns),
[SVG color interpolation](https://svgwg.org/svg2-draft/painting.html#ColorInterpolationProperty),
[animation accessors](https://drafts.csswg.org/css-animations-2/#dom-animationevent-animation),
[page margin boxes](https://drafts.csswg.org/css-page-3/#margin-boxes), and
[system colors](https://drafts.csswg.org/css-color-4/#css-system-colors).
Cascade, computed styles, selector matching, layout, event execution and mixin
expansion remain outside the authoring contract.

## Delivery decision

This refresh updates source pins, the dated inventory, evidence mapping and
executable probes. It requires no engine, public API or package behavior change,
so no Changeset or new release is warranted. Existing published **0.4.0** packages
provide the cold native/WASM evidence, without workspace-linked artifacts.

## Validation

Source-backed target regeneration and the coverage generator reproduce the
checked-in files. Dependencies were installed with the pinned npm 11.16.0 using
`npm ci --include=dev`; validation uses Node 24.12.0. Script typechecking and all
52 CI-script tests pass.

A fresh registry installation of `sheetom@0.4.0` and `@sheetom/wasm@0.4.0` passes
`check-installed-packages.ts` with the repository's test sources and corpus.
Both backends pass 25 shared modern CSS contracts and depth checks through 4,000.
Chromium 151.0.7922.34, Firefox 153.0 and the existing alpha oracle Chromium
153.0.8010.12 compare 377 authoring probes, 12,038 support queries and 264 escape
checks per backend with zero mismatches. Native and WASM Webref ratchets each
check 667 properties, 370 profiles and 8,415 branches through 11,640 cases with
zero acceptance, observable-state, declaration-text, indexed-item, atomicity or
reparse mismatches. Focused checks compare 18 invalid grammar values with
Chromium and reject them atomically on each installed backend (36 backend cases),
covering all six changed property roots.

The existing [stable 0.4.0 release](https://github.com/leo91000/sheetom/releases/tag/v0.4.0)
is non-draft and non-prerelease; its tag resolves to
`6cc1224b4cf010f199ed015fa7102c2173dd002f`. All 15 cohort packages match the release
manifest's versions and integrity, have `latest` set to 0.4.0 in both registry
endpoints, and expose attestation metadata. `npm audit signatures` verifies
registry signatures and attestations for the three installed packages. This
distinguishes cohort-wide metadata presence from cryptographic verification of
the installed platform subset.
