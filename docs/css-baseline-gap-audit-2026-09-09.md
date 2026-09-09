# CSS Baseline gap audit — 9 September 2026

Pre-implementation audit. See [the implementation contract](css-september-2026-parity-evidence.md) for the subsequent fixes and validation.

Four authoring gap groups are confirmed in both native and WASM builds of current GitHub main (`bde80b7848503fd55b73032804761375c2ea9832`). This audit uses Baseline newly available or widely available by 2026-09-09, including individually eligible branches of feature families that are not entirely Baseline.

Scope is CSS parsing, mutation, authoring interfaces, capability reporting, and serialization. DOM evaluation, selector matching, cascade, and layout are outside SheetOM's scope. Inventory completeness and passing sampled inputs do not establish exhaustive grammar conformance.

## Confirmed implementation gaps

| Feature | Baseline newly available | Observed SheetOM behavior | Required authoring behavior |
|---|---|---|---|
| `sibling-count()` and `sibling-index()` | 2026-08-18 | Valid declarations are discarded; `CSS.supports()` returns false. | Accept the numeric functions directly and inside compatible `calc()` expressions, preserve them through mutation/serialization, and report support. |
| `progress()` | 2026-09-01 | Valid numeric, dimensioned, and relative examples are discarded; `CSS.supports()` returns false. | Accept valid progress expressions and their use in math expressions, serialize appropriately, and report support. |
| Seven media state pseudo-classes | 2026-08-28 | Selectors survive authoring, but `CSS.supports("selector(...)")` returns false. | Report support for `:playing`, `:paused`, `:seeking`, `:buffering`, `:stalled`, `:muted`, and `:volume-locked`. |
| `CSSPositionTryDescriptors` and `@position-try` descriptor filtering | 2026-01-13 for the relevant individual API branches | Interface export is absent; `CSSPositionTryRule.style` is a generic `CSSStyleDeclaration`, accepting forbidden declarations such as `color` and `display`. | Expose the descriptor interface, use it for the rule's style, and enforce allowed descriptors during parsing and mutation. |

Baseline dates: [sibling functions](https://web-platform-dx.github.io/web-features-explorer/features/sibling-count/), [progress](https://web-platform-dx.github.io/web-features-explorer/features/progress-function/), [media pseudo-classes](https://web-platform-dx.github.io/web-features-explorer/features/media-pseudos/). The descriptor interface and rule contract are defined by [CSS Anchor Positioning](https://drafts.csswg.org/css-anchor-position-1/#csspositiontryrule). Its individual Baseline dates come from the pinned WebDX data below; the whole anchor-positioning family is still marked limited availability.

Reproducing declarations:

```css
.example {
  order: sibling-index();
  width: calc(100% / sibling-count());
  animation-delay: calc(0.1s * sibling-index());
  opacity: progress(50px, 0px, 100px);
  height: calc(100px * progress(50vw, 0px, 1000px));
}

@position-try --try {
  top: 10px;
  width: 20px;
  color: red;     /* Browsers discard this descriptor. SheetOM retains it. */
  display: block; /* Browsers discard this descriptor. SheetOM retains it. */
}
```

For each media selector, assigning `sheet.cssRules[0].selectorText = "video:playing"` retains the selector, while `CSS.supports("selector(video:playing)")` returns false. This is a capability-reporting gap, not an absent selector parser. Raw function tokens surviving inside custom properties do not establish support for sibling or progress functions in typed declarations.

## Inventory gaps

The June target generator first requires a whole feature family to be Baseline, then selects its eligible compatibility branches. This hides individually Baseline branches when other parts of their family remain unavailable. Advancing only the cutoff adds three whole families (311 to 314) but does not fix that selection problem.

The independent per-key audit finds 374 unique omitted compatibility keys across 18 affected families: 365 authoring-oriented keys and nine DOM/SVG integration keys outside scope. Of the authoring keys, 341 were already eligible by June and 24 became eligible afterward. These are compatibility data entries, not 365 missing implementations.

| Affected family | Omitted authoring keys | Audit disposition |
|---|---:|---|
| alignment-baseline | 7 | Sampled eligible keywords accepted, including July additions. |
| Anchor positioning | 311 | Descriptor interface/filtering gap above; sampled anchor values accepted. Includes 74 CSSOM API keys, which do not represent 74 separately missing APIs. |
| attr() | 2 | Sampled content fallback syntax accepted; typed attr is not implied by these eligible branches. |
| background-attachment | 4 | Sampled scroll/local/multiple-background syntax accepted. |
| Column breaks | 3 | Sampled break-inside syntax accepted. |
| Cursor styles | 5 | Sampled eligible keywords and URL syntax accepted. |
| display: contents | 1 | Accepted. |
| display-mode media query | 2 | Sampled browser query accepted. |
| font-variant-position | 2 | Sampled normal value accepted. |
| Fullscreen API CSS selector | 1 | `::backdrop` accepted. |
| Media pseudo-classes | 7 | Capability-reporting gap above. |
| Page selectors | 1 | `@page :first` accepted. |
| path() | 3 | Sampled clip-path and offset-path values accepted. |
| progress() | 1 | Function gap above. |
| Sibling functions | 2 | Both functions missing above. |
| System colors | 1 | `AccentColor` and `AccentColorText` accepted. |
| text-autospace | 3 | Sampled eligible values accepted. |
| text-box | 9 | Sampled eligible properties/keywords accepted, including normal, auto, and trim variants. This does not make every text-box-edge branch Baseline. |

The nine excluded integration keys concern eight implicit HTML/DOM anchor relationships and one SVG cursor attribute. The table records sampled authoring results; it does not claim every semantic condition described by each compatibility entry was exercised.

Four generated anchor samples were rejected by SheetOM and Chromium alike. Three used `anchor-center` on align-items/justify-items/place-items; the current anchor specification removed its use on the items properties. The fourth used the compatibility branch label `position-area` as a literal position-try-fallbacks value; actual area keywords such as block-start were accepted. These shared rejections are not counted as implementation gaps or positive syntax coverage.

The inventory should select eligible individual branches independently of parent-family eligibility. Its evidence mapping should continue distinguishing syntax/API checks from DOM and rendering behavior. The Chromium 151 reference also needs refreshing for September eligibility: Chromium support for the media selectors arrived in 152. Neither change was made during this read-only implementation audit.

## Evidence and source pins

- Current published WebDX `web-features`: **3.37.0**, revision `60e8982caeaa28c4913f45c2cb20b5f6114de510`, verified from npm on the audit date. This is the same data snapshot used by the June inventory, evaluated with a later cutoff and corrected branch selection.
- [Source tarball](https://registry.npmjs.org/web-features/-/web-features-3.37.0.tgz); extracted `data.json` SHA-256: `7633fe15d2ac393c69682150cd2e20b1164f4b8396fd9ecbc8226e25777dcac0`.
- Runtime tested from worktree commit `510a20b`, verified to have the same tree as merged main `bde80b7`. Native and WASM engine identity: ABI 5, revision suffix `.79`, hash `0cdb7affa3110506b1e8c759d5fb9cef1e46f0baaab6e9767768d83d456cdd5c`.
- Browser comparisons used Chromium **151.0.7922.34** and Firefox **153.0**. Chromium verifies the sibling/progress acceptance gaps; Firefox verifies current media selector behavior. Both verify the position-try descriptor behavior. These installed versions are not a claim to test every current Baseline browser.
- **294 fresh differential cases** across the omitted surfaces: native and WASM agree throughout; 17 cases differ from Chromium (six sibling cases, three progress cases, seven media selector cases, one descriptor case). The seven media cases reflect Chromium 151's older support as described above; Firefox supplies the positive reference.
- **293 round-trip cases per backend** have no semantic round-trip mismatch. The known invalid position-try descriptor fixture was excluded. Round-tripping an empty result after rejection does not establish function support; acceptance and support-query evidence above remain decisive.
- Fresh `npm run css:target:check` passed: 259 browser probes, 11,881 supports checks and 264 escape checks per backend, plus 17 shared modern-CSS contracts and deep nesting checks through depth 4,000 per backend.
- Fresh Webref probes passed on both backends: 666 properties, 371 profiles, 8,369 branches, 11,590 checks per backend, zero acceptance/observable/cssText/items/atomicity/reparse mismatches.

Session reproduction scripts and detailed results are at `/tmp/sheetom-inventory-delta.mjs`, `/tmp/sheetom-baseline-delta.json`, `/tmp/sheetom-baseline-runtime-audit.mjs`, `/tmp/sheetom-baseline-runtime-results.json`, `/tmp/sheetom-baseline-roundtrip-audit.mjs`, and `/tmp/sheetom-baseline-roundtrip-results.json`. These temporary artifacts are not permanent repository fixtures.

## Limits and deferred features

This is the full confirmed gap list from this inventory audit and the described probes, not proof that all other Baseline grammar combinations and CSSOM operations conform. Existing regression coverage has its own documented boundaries in [the June evidence report](css-june-2026-parity-evidence.md).

The current WebDX snapshot does not make whole families such as native mixins, custom functions, if(), random(), calc-size(), interpolate-size, scroll-driven animations, or style-query range syntax Baseline. They should not be added to this list solely because they are recent CSS features. Existing experimental authoring support is separate from the Baseline contract.
