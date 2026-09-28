# CSS Baseline audit — 28 September 2026

The official npm registry's current `web-features` release is **3.40.0**, revision
`8ddac30855da1beb8a99347eefcd344a02310709`. The downloaded
[source archive](https://registry.npmjs.org/web-features/-/web-features-3.40.0.tgz)
matches npm's SHA-512 integrity. Its `data.json` has SHA-256
`656360c41c80b95064eec068453e4d26d85668587124352ab3226b4a34ca1670`.
The cutoff is **2026-09-28**. [WebDX](https://github.com/web-platform-dx/web-features)
provides availability evidence; authoring conformance is measured separately.

## Complete target delta

Relative to the [21 September audit](css-baseline-audit-2026-09-21.md), two
compatibility branches become eligible, nine advance from Newly Available to
Widely Available, and three deferred branches disappear upstream. The new
container-name-queries family brings the inventory to 331 families, 4,218 unique
keys, 3,759 eligible records and 459 deferred records. There are no withdrawn
eligible branches or changed specification links in retained families. Upstream
also renames the existing container-queries display label to “Container queries
(size)”. Coverage
maps 2,651 authoring records to 598 surfaces and excludes 1,108 DOM/evaluation
records. Deferred entries comprise 396 not-Baseline and 63 after-cutoff records;
there are no undated records. No missing date is assigned eligibility.

| Changed branch | Disposition and evidence |
| --- | --- |
| `css.at-rules.container.container-query_optional` | Missing observable state: name-only rules parsed but exposed an empty name and the name as their query. Corrected in shared Rust; shared native/WASM contracts and browser probes cover the fields, escaped names, grammar, nested mutations and reparsing. |
| `css.properties.outline.auto` | Already supported. Added an explicit declaration probe; existing Webref outline profiles cover shorthand state and mutation. |
| `api.CSSKeyframesRule.length` | Already eligible, now high. Existing keyframe collection operations in `tests/keyframes.test.ts` cover the authoring API. |
| `css.properties.paint-order` | Already eligible, now high. Existing Webref property profiles cover its keyword ordering. |
| `css.properties.text-wrap.nowrap`, `.wrap` | Already eligible, now high. Existing Webref text-wrap profiles cover both keywords and shorthand state. |
| `css.properties.white-space-collapse`, `.break-spaces`, `.collapse`, `.preserve`, `.preserve-breaks` | Already eligible, now high. Existing Webref white-space profiles cover the keyword grammar and observable state. |

Removed deferred records are `css.properties.content.none_applies_to_elements`,
`css.properties.list-style-type.symbols`, and `css.properties.list-style.symbols`.
None was eligible; removal does not remove supported authoring behavior.

## Reproduction and contract

Fresh installed native and WASM **0.3.0** both report `containerName === ""`
and `containerQuery === "card"` for `@container card { .a { color: red; } }`.
The pinned Chromium **151.0.7922.34** reports `"card"` and `""` respectively.
The same comparison exposed incorrect partitioning of escaped names and unnamed
negated queries. The existing owned parser already validates optional-query
syntax. The shared core now partitions the validated prelude at a CSS token
boundary and permits an empty query when a name exists. It keeps the `not`
operator in the query. No vendor grammar change or browser pin update is needed.

Browser snapshots now include `containerName` and `containerQuery`; accepting a
rule or comparing only `conditionText` had missed this observable defect. The
shared backend contract covers name-only and escaped names, invalid/reserved
names, insertion failure atomicity, live child collections, detachment, and
semantic serialization/reparse. Existing September math, descriptor, alpha and
image regressions remain active; no previous in-scope gap is left unresolved.

Primary contracts are [container conditions](https://drafts.csswg.org/css-conditional-5/#typedef-container-condition),
[container rule fields](https://drafts.csswg.org/css-conditional-5/#the-csscontainerrule-interface),
and [outlines](https://drafts.csswg.org/css-ui-4/#outline).
Container selection, layout, cascade, computed values, selector matching and
mixin expansion remain outside scope. Pinned mixin/function experiments remain
separate from Baseline. [ADR 192](adr/0192-expose-name-only-container-state.md)
records the observable-state decision.

## Validation

The source-backed target and coverage generators reproduce the checked-in
inventories. Both sequentially built backends pass 25 shared modern CSS contracts
and depth checks through 4,000. Chromium/Firefox comparisons pass 324 authoring
probes, 11,985 support queries and 264 escape checks per backend with zero
mismatches. Native and WASM Webref ratchets also report zero mismatches for
acceptance, observable state, declaration text, indexed items, mutation atomicity
and reparsing.

`npm run check` passes 297 unit tests, generated manifests, documentation, runtime
artifacts and package installation checks. Native formatting, Clippy, all 237
workspace tests and vendor suites (including 189 Lightning CSS tests) pass.
`npm run wasm:check`, `npm run wasm:test`, and all 52 CI-script tests pass. The
optimized WASM is 4,596,640 raw bytes and 1,335,833 gzip bytes, within the unchanged
budgets. The platform, browser, performance and release matrices remain CI gates.
