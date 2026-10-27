# CSS Baseline audit — 27 October 2026

The cutoff is **2026-10-27**. The official npm registry reports **web-features
3.44.0**, source revision `2c24bbfc45cfb8b7fea90bd1618b0ad5027bec5d`.
The [downloaded archive](https://registry.npmjs.org/web-features/-/web-features-3.44.0.tgz)
matches the registry SHA-512 integrity. Its actual `data.json` bytes have SHA-256
`72577e8df9c8abe8a8af86a03bc161ac06be21488d76857f157c7f83f017f706`.
[WebDX](https://github.com/web-platform-dx/web-features) supplies eligibility;
executable comparisons independently establish authored CSSOM support.
The baseline repository revision is `99843ba7436409ea12a7b1c04b6d12ad4c47fa99`.

## Complete inventory delta

Relative to the [19 October audit](css-baseline-audit-2026-10-19.md), this refresh
adds 29 eligible records, removes 18 eligible records through alignment key
reorganization, and adds two deferred records. Twenty-four retained records
change status or date. No family is added or removed; no retained family changes
its name, family availability date, or specification links. The target contains
333 families, 4,363 unique compatibility keys, 3,885 eligible records and 478
deferred records. The evidence map assigns 2,772 authoring records to 598
surfaces and excludes 1,113 DOM/evaluation records.

Eligibility uses individual low/high branches dated at or before the cutoff,
including branches in families that are not wholly Baseline. The deferred records
are 415 not-Baseline and 63 after-cutoff records, with no missing status objects.
The two new SVG records have baseline false and no availability date; no dates
are invented. Pinned mixin/function experiments and earlier withdrawn Baseline
claims remain separate. The September authoring gaps remain closed by the
[September implementation](css-september-2026-parity-evidence.md); the mask and
all mutation corrections remain covered by the October contracts.

Every changed compatibility record is accounted for below. Existing property
profiles provide valid/invalid grammar coverage; the added `baseline-october-27.*`
probes exercise the affected values through both public facades and browser state,
including invalid-write atomicity and semantic serialization/reparse. A root
mapping alone is not evidence. Layout-context subkeys share property authoring
syntax; positioning, flex/grid alignment, glyph shaping, geometry and form-control
rendering are outside SheetOM's authoring contract.

| Family | Compatibility record | Change | Disposition/evidence |
| --- | --- | --- | --- |
| absolute-positioning | `css.properties.justify-self.context_position_absolute` | Added eligible key | Existing property authoring: October 27 alignment probes and Webref profiles; layout context outside scope. |
| absolute-positioning | `css.properties.place-self.context_position_absolute` | Added eligible key | Existing property authoring: October 27 alignment probes and Webref profiles; layout context outside scope. |
| absolute-positioning | `css.properties.place-self.position_absolute_context` | Removed eligible key | Alignment compatibility-key reorganization; existing authoring syntax retained. |
| absolute-positioning | `css.properties.justify-self.position_absolute_context` | Removed eligible key | Alignment compatibility-key reorganization; existing authoring syntax retained. |
| align-content-block | `css.properties.align-content.context_block` | low → high | Existing property authoring: October 27 probes and Webref profiles; evaluation outside scope. |
| flexbox | `css.properties.align-content.flex-end` | Added eligible key | Existing property authoring: October 27 value probes and Webref profiles. |
| flexbox | `css.properties.align-content.flex-start` | Added eligible key | Existing property authoring: October 27 value probes and Webref profiles. |
| flexbox | `css.properties.align-self.flex-end` | Added eligible key | Existing property authoring: October 27 value probes and Webref profiles. |
| flexbox | `css.properties.align-self.flex-start` | Added eligible key | Existing property authoring: October 27 value probes and Webref profiles. |
| flexbox | `css.properties.justify-content.context_flex` | Added eligible key | Existing property authoring: October 27 alignment probes and Webref profiles; layout context outside scope. |
| flexbox | `css.properties.justify-content.end` | Added eligible key | Existing property authoring: October 27 value probes and Webref profiles. |
| flexbox | `css.properties.justify-content.left` | 2020-01-15 → 2022-03-14 | Existing property authoring: October 27 probes and Webref profiles; evaluation outside scope. |
| flexbox | `css.properties.justify-content.right` | 2020-01-15 → 2022-03-14 | Existing property authoring: October 27 probes and Webref profiles; evaluation outside scope. |
| flexbox | `css.properties.justify-content.safe` | Added eligible key | Existing property authoring: October 27 value probes and Webref profiles. |
| flexbox | `css.properties.justify-content.space-evenly` | Added eligible key | Existing property authoring: October 27 value probes and Webref profiles. |
| flexbox | `css.properties.justify-content.start` | Added eligible key | Existing property authoring: October 27 value probes and Webref profiles. |
| flexbox | `css.properties.justify-content.stretch` | Added eligible key | Existing property authoring: October 27 value probes and Webref profiles. |
| flexbox | `css.properties.justify-content.unsafe` | Added eligible key | Existing property authoring: October 27 value probes and Webref profiles. |
| flexbox | `css.properties.justify-items.context_flex` | Added eligible key | Existing property authoring: October 27 alignment probes and Webref profiles; layout context outside scope. |
| flexbox | `css.properties.place-content.context_flex` | Added eligible key | Existing property authoring: October 27 alignment probes and Webref profiles; layout context outside scope. |
| flexbox | `css.properties.place-items.context_flex` | Added eligible key | Existing property authoring: October 27 alignment probes and Webref profiles; layout context outside scope. |
| flexbox | `css.properties.place-self.context_flex` | Added eligible key | Existing property authoring: October 27 alignment probes and Webref profiles; layout context outside scope. |
| flexbox | `css.properties.place-self.flex_context` | Removed eligible key | Alignment compatibility-key reorganization; existing authoring syntax retained. |
| flexbox | `css.properties.justify-content.flex_context.start_end` | Removed eligible key | Alignment compatibility-key reorganization; existing authoring syntax retained. |
| flexbox | `css.properties.justify-content.flex_context.left_right` | Removed eligible key | Alignment compatibility-key reorganization; existing authoring syntax retained. |
| flexbox | `css.properties.justify-content.flex_context` | Removed eligible key | Alignment compatibility-key reorganization; existing authoring syntax retained. |
| flexbox | `css.properties.justify-content.flex_context.stretch` | Removed eligible key | Alignment compatibility-key reorganization; existing authoring syntax retained. |
| flexbox | `css.properties.place-content.flex_context` | Removed eligible key | Alignment compatibility-key reorganization; existing authoring syntax retained. |
| flexbox | `css.properties.place-items.flex_context` | Removed eligible key | Alignment compatibility-key reorganization; existing authoring syntax retained. |
| flexbox | `css.properties.justify-content.flex_context.safe_unsafe` | Removed eligible key | Alignment compatibility-key reorganization; existing authoring syntax retained. |
| flexbox | `css.properties.justify-items.flex_context` | Removed eligible key | Alignment compatibility-key reorganization; existing authoring syntax retained. |
| flexbox | `css.properties.justify-content.flex_context.space-evenly` | Removed eligible key | Alignment compatibility-key reorganization; existing authoring syntax retained. |
| font-variant-caps | `css.properties.font-variant.all-petite-caps` | Added eligible key | Existing property authoring: October 27 value probes and Webref profiles. |
| font-variant-caps | `css.properties.font-variant.all-small-caps` | Added eligible key | Existing property authoring: October 27 value probes and Webref profiles. |
| font-variant-caps | `css.properties.font-variant.petite-caps` | Added eligible key | Existing property authoring: October 27 value probes and Webref profiles. |
| font-variant-caps | `css.properties.font-variant.small-caps` | Added eligible key | Existing property authoring: October 27 value probes and Webref profiles. |
| font-variant-caps | `css.properties.font-variant.titling-caps` | Added eligible key | Existing property authoring: October 27 value probes and Webref profiles. |
| font-variant-caps | `css.properties.font-variant.unicase` | Added eligible key | Existing property authoring: October 27 value probes and Webref profiles. |
| grid | `css.properties.justify-content.context_grid` | Added eligible key | Existing property authoring: October 27 alignment probes and Webref profiles; layout context outside scope. |
| grid | `css.properties.justify-items.context_grid` | Added eligible key | Existing property authoring: October 27 alignment probes and Webref profiles; layout context outside scope. |
| grid | `css.properties.justify-self.context_grid` | Added eligible key | Existing property authoring: October 27 alignment probes and Webref profiles; layout context outside scope. |
| grid | `css.properties.place-content.context_grid` | Added eligible key | Existing property authoring: October 27 alignment probes and Webref profiles; layout context outside scope. |
| grid | `css.properties.place-items.context_grid` | Added eligible key | Existing property authoring: October 27 alignment probes and Webref profiles; layout context outside scope. |
| grid | `css.properties.place-self.context_grid` | Added eligible key | Existing property authoring: October 27 alignment probes and Webref profiles; layout context outside scope. |
| grid | `css.properties.justify-items.grid_context` | Removed eligible key | Alignment compatibility-key reorganization; existing authoring syntax retained. |
| grid | `css.properties.place-items.grid_context` | Removed eligible key | Alignment compatibility-key reorganization; existing authoring syntax retained. |
| grid | `css.properties.justify-self.grid_context` | Removed eligible key | Alignment compatibility-key reorganization; existing authoring syntax retained. |
| grid | `css.properties.place-self.grid_context` | Removed eligible key | Alignment compatibility-key reorganization; existing authoring syntax retained. |
| grid | `css.properties.justify-content.grid_context` | Removed eligible key | Alignment compatibility-key reorganization; existing authoring syntax retained. |
| grid | `css.properties.place-content.grid_context` | Removed eligible key | Alignment compatibility-key reorganization; existing authoring syntax retained. |
| popover | `api.HTMLButtonElement.popoverTargetAction` | low → high | DOM association/events/HTML behavior outside scope; no authoring gap. |
| popover | `api.HTMLButtonElement.popoverTargetElement` | low → high | DOM association/events/HTML behavior outside scope; no authoring gap. |
| popover | `api.HTMLElement.beforetoggle_event` | low → high | DOM association/events/HTML behavior outside scope; no authoring gap. |
| popover | `api.HTMLElement.beforetoggle_event.popover_elements` | low → high | DOM association/events/HTML behavior outside scope; no authoring gap. |
| popover | `api.HTMLElement.hidePopover` | low → high | DOM association/events/HTML behavior outside scope; no authoring gap. |
| popover | `api.HTMLElement.showPopover` | low → high | DOM association/events/HTML behavior outside scope; no authoring gap. |
| popover | `api.HTMLElement.togglePopover` | low → high | DOM association/events/HTML behavior outside scope; no authoring gap. |
| popover | `api.HTMLElement.togglePopover.returns_boolean` | low → high | DOM association/events/HTML behavior outside scope; no authoring gap. |
| popover | `api.HTMLElement.toggle_event.popover_elements` | low → high | DOM association/events/HTML behavior outside scope; no authoring gap. |
| popover | `api.HTMLInputElement.popoverTargetAction` | low → high | DOM association/events/HTML behavior outside scope; no authoring gap. |
| popover | `api.HTMLInputElement.popoverTargetElement` | low → high | DOM association/events/HTML behavior outside scope; no authoring gap. |
| popover | `css.selectors.backdrop.popover` | low → high | Existing selector probes, mutation and reparse; matching and popover behavior outside scope. |
| popover | `css.selectors.popover-open` | low → high | Existing selector probes, mutation and reparse; matching and popover behavior outside scope. |
| popover | `html.elements.button.popovertarget` | low → high | DOM association/events/HTML behavior outside scope; no authoring gap. |
| popover | `html.elements.button.popovertargetaction` | low → high | DOM association/events/HTML behavior outside scope; no authoring gap. |
| popover | `html.elements.input.popovertarget` | low → high | DOM association/events/HTML behavior outside scope; no authoring gap. |
| popover | `html.elements.input.popovertargetaction` | low → high | DOM association/events/HTML behavior outside scope; no authoring gap. |
| popover | `html.global_attributes.popover` | low → high | DOM association/events/HTML behavior outside scope; no authoring gap. |
| svg | `svg.elements.use.omit_external_fragment` | Added deferred key | Not yet eligible: baseline false, no date; SVG DOM/rendering is outside scope. |
| svg | `svg.global_attributes.transform.svg_root` | Added deferred key | Not yet eligible: baseline false, no date; SVG DOM/rendering is outside scope. |
| transform-box | `css.properties.transform-box.content-box` | low → high | Existing property authoring: October 27 probes and Webref profiles; evaluation outside scope. |
| transform-box | `css.properties.transform-box.stroke-box` | low → high | Existing property authoring: October 27 probes and Webref profiles; evaluation outside scope. |
| vertical-form-controls | `css.properties.writing-mode.vertical_oriented_form_controls` | low → high | Existing property authoring: October 27 probes and Webref profiles; evaluation outside scope. |

The alignment context keys move from `flex_context`, `grid_context` and
`position_absolute_context` to `context_flex`, `context_grid` and
`context_position_absolute`; several compound justify-content keys become
individual keyword keys. The `left` and `right` dates change from 2020-01-15 to
2022-03-14 without changing eligibility. The six newly catalogued font-variant
caps branches already use the owned font grammar. Low-to-high promotions do not
introduce new authoring syntax. The popover family itself remains low even though
individual branches are now high; family status is not substituted for branch
status.

Primary contracts and browser support evidence consulted:
[box alignment](https://drafts.csswg.org/css-align-3/),
[font caps](https://drafts.csswg.org/css-fonts-4/#font-variant-caps-prop),
[transform boxes](https://drafts.csswg.org/css-transforms-1/#transform-box),
[vertical modes](https://drafts.csswg.org/css-writing-modes-4/#vertical-modes),
[popover](https://html.spec.whatwg.org/multipage/popover.html), and
[BCD alignment data](https://github.com/mdn/browser-compat-data/blob/main/css/properties/justify-content.json).
The executable browser oracles below resolve authored state, independently of
those eligibility records. Cascade, computed styles, selector matching, layout,
painting, DOM event execution and mixin expansion remain outside scope.

## Validation and delivery

Validation results and exact delivery checkpoints are recorded as gates complete.
The current change updates inventory and executable evidence; no public runtime
behavior or package version is changed, and no Changeset is manufactured. The
repository CI classifier and Release workflow determine the applicable jobs and
whether there is a release candidate.

A fresh registry install of `sheetom@0.4.1` and `@sheetom/wasm@0.4.1` passes the
repository's maintained `check-installed-packages.ts` runner with its original
sources/corpus/assertions. Both backends pass 27 shared modern CSS contracts and
depth checks through 4,000; 491 browser authoring probes, 12,295 support queries
and 264 escape checks per backend have zero mismatches. Both Webref ratchets cover
667 properties, 370 profiles, 8,549 branches and 11,783 cases, with zero acceptance,
observable-state, declaration-text, indexed-item, atomicity or reparse mismatches.
The oracles are Chromium 151.0.7922.34, Firefox 153.0 and Baseline Chromium
153.0.8010.12. The added probes therefore require no oracle pin change.
`npm audit signatures` verifies registry signatures and attestations for all three
installed packages (root, WASM and the host native package).

Source-backed target regeneration and rechecking, coverage regeneration, script
typechecking, 52 CI-script tests, documentation, and the broad check through its
300 unit tests, conformance, generated catalog/types, build and runtime checks
pass. The package test initially used the host's npm 12.2.1 and failed to read its
pack result; it passes with repository-pinned npm 11.16.0. No assertions or package
scripts were changed. Native and WASM release builds run sequentially using Rust
1.88.0, wasm-bindgen 0.2.127 and the repository's size budgets. Node is 24.22.1;
the node exposes 24 CPUs and approximately 94 GiB RAM (58 GiB available at setup),
so Cargo uses eight workers. Build caches remain in the managed workspace/home.

The rebuilt native/WASM target checks, both Webref ratchets, `native:check`,
`wasm:check` and `wasm:test` also pass. Remote delivery gates are recorded in the
durable run checkpoint/evidence. An open PR, a successful subset of CI, or the existing
0.4.1 publication is not presented as evidence of a new release. The final run
report records the exact PR/head/merge SHAs and actual Release automation outcome.
