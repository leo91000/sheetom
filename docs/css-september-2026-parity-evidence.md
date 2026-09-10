# September 2026 CSS authoring target

The current target includes individually Baseline CSS compatibility branches
through **2026-09-09**, including branches of families that are not completely
Baseline. The source remains web-features 3.37.0, with revision and data hash in
[the target manifest](../compatibility/css-feature-target.json).

The selection contains 328 families, 3,755 eligible compatibility key records,
and 475 deferred records. [The evidence map](../compatibility/css-authoring-coverage.json)
assigns 2,648 authoring key records to 603 surfaces and excludes 1,107 records
concerning DOM or evaluation. This mapping is not proof of every possible grammar
combination or CSSOM operation.

## Added authoring behavior

- `sibling-count()` and `sibling-index()` remain typed numeric expressions that
  can participate in calculations across compatible dimensions. They are not
  evaluated against a DOM by SheetOM.
- `progress()` validates three consistently typed argument calculations. Its
  argument dimension is independent of the enclosing property. Numbers,
  lengths, percentages, times, angles, frequencies, and resolutions are parsed;
  relative expressions remain unresolved for browser evaluation.
- `CSS.supports("selector(...)")` recognizes `:playing`, `:paused`, `:seeking`,
  `:buffering`, `:stalled`, `:muted`, and `:volume-locked`.
- `CSSPositionTryRule.style` returns a live `CSSPositionTryDescriptors` instance
  with the specified descriptor accessors. Its shared engine context filters
  unsupported properties, custom properties, and parsed important declarations.
  As in the reference browser, setProperty can set an important descriptor even
  though declaration-block parsing discards important descriptors.

These changes close the four gap groups in [the pre-implementation audit](css-baseline-gap-audit-2026-09-09.md).
Existing experimental mixin support remains separate from Baseline eligibility.
The project still does not evaluate cascade, computed styles, selector matching,
layout, or mixin expansion.

## Validation

`tests/baseline-september.test.ts` exercises valid and invalid math, mutation
atomicity, semantic round trips, descriptor interfaces, all mutation entry
points, detachment, and media capability queries. The packaged native and WASM
facades execute the same test source through `test-modern-css-backends.ts`.

`check-css-authoring-target.ts` compares authored state and support queries with
Chromium 151.0.7922.34. Explicitly marked media selector probes use Firefox 153.0,
because Chromium 151 predates their Baseline availability. The selected oracle
versions appear in the generated report. Existing Webref grammar probes and
native/browser/WASM suites continue to apply.

```sh
npm run native:build
npm run build
npm run wasm:build
npm run css:target:check
npm run native:webref-property-branches
node scripts/check-webref-property-branches.ts --wasm
npm run check
npm run native:check
npm run wasm:test
```

Primary references: [CSS Values 5](https://drafts.csswg.org/css-values-5/),
[CSS Anchor Positioning interfaces](https://drafts.csswg.org/css-anchor-position-1/#csspositiontryrule),
[WebDX sibling functions](https://web-platform-dx.github.io/web-features-explorer/features/sibling-count/),
[WebDX progress](https://web-platform-dx.github.io/web-features-explorer/features/progress-function/),
and [WebDX media selectors](https://web-platform-dx.github.io/web-features-explorer/features/media-pseudos/).
