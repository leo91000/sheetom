# June 2026 authoring target: implementation and evidence

This is the unreleased authoring target for features Newly available by
2026-06-30, plus explicitly selected experimental mixins and existing custom
functions. It does not implement cascade, mixin expansion, selector matching,
layout, or other DOM/runtime evaluation.

## Target and reproducibility

The [target manifest](../compatibility/css-feature-target.json) pins WebDX
`web-features@3.37.0`, its source hash, and each compatibility key's date.
[The coverage mapping](../compatibility/css-authoring-coverage.json) assigns all
311 selected families: 2,283 eligible key records map to 572 authoring surfaces;
1,098 concern DOM or evaluation operations; 364 are outside the dated target.
An early family date never admits its later compatibility subkeys.

The mapping links property surfaces to actual executable Webref grammar profile
IDs, selector/type/rule surfaces to operation probe IDs, and public interfaces
to operation suites. It is a surface-level evidence index, not a claim that each
BCD subkey has an independent assertion. Rendering subkeys inherit their
applicable authored syntax without acquiring rendering support. The target
includes the 18 families first reaching Baseline during January–June 2026.
Existing later grammar remains supported and covered by regression suites.

Build the same private engine ABI for both public facades, then run:

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
npm run wasm:test:browsers
npm run wasm:test:memory
npm run wasm:test:performance
```

`check-css-authoring-target.ts` compares native and WASM snapshots with Chromium
151.0.7922.34, including declaration order, values, priority, selector state,
rejected-mutation atomicity, parentage, live lists, and safe serialization
reparsing. Reparse checks also compare rule topology, selector/descriptor state,
declaration names/order and priorities, and independently canonicalized longhand
values. This permits equivalent scalar spellings while detecting dropped rules,
declarations, or shorthand settings; it does not rely solely on serializing the
already-reparsed sheet. Deliberately corrupted copies exercise these assertions.
Its report is `target/css-authoring-target-report.json`.
`test-modern-css-backends.ts` executes the same mixin and CSS namespace unit
contracts through both packaged facades, in separate subprocesses, and probes
nested conditions, selectors, and mixin bodies at depths 256, 1024, and 4000.
CI and release oracle jobs run the applicable backend gates.

The existing property corpus covers 666 properties, 371 grammar profiles,
8,369 branches and 11,590 mutations. The new operation probes supplement it:
that corpus alone missed `linear()` and interpolation methods in gradients
outside `shape-outside`. Those gaps are now implemented in the shared typed
parser, with longhand and shorthand probes and vendored regression tests.
Observed gradient projection reuses the existing authored-gradient machinery.
Selector projection now uses CSSOM pseudo-element/An+B/relative-selector
spelling and handles wildcard and named namespaces during parsing and mutation.
Unknown namespace prefixes reject insertion or selector replacement atomically;
namespace insertion/deletion rejects changes while ordinary rules exist.
Late namespace declarations are discarded, including their bindings, following
the pinned Chromium recovery behavior. The browser differential suite covers
both undeclared prefixes and late declarations.
Attribute prefixes receive the same validation as type-selector prefixes.
`:is()` and `:where()` discard invalid namespace branches while `:not()`,
`:has()`, and `:nth-child(... of ...)` reject an invalid list. Capability checks
reject invalid branches even in forgiving lists. Parsed selectors retain their
normalized namespace state when detached. Authored `:is()` wrappers remain
observable, including when recovery leaves one or zero branches.

The stronger round-trip check exposed inactive animation settings being dropped
by the compiler serializer when the animation name was `none`. Authoring output
now preserves those settings; changing `animation-name` after reparse retains the
original duration and easing. Compiler serialization defaults remain unchanged.

## Experimental mixin resolution

The authority is [CSSWG revision
5e68d5c1ca6656dd7a4b32f821d187aa9652ad5d](https://github.com/w3c/csswg-drafts/blob/5e68d5c1ca6656dd7a4b32f821d187aa9652ad5d/css-mixins-1/Overview.bs),
SHA-256 `4967c197dfd1b86cb321b835ebcdb2577974bc80d7097294294cd75a8249a76c`.
[ADR 0189](./adr/0189-expose-pinned-draft-mixin-interfaces-by-default.md) selects
its five interfaces by default. Parameter records and argument arrays are fresh
snapshots; grouping lists and declaration blocks are live. Calls remain authored
and unresolved. Unsupported or invalid parameter defaults, duplicate parameter
names, misplaced rules, and invalid free-form arguments are rejected.

[Recorded flagged Chromium observations](../compatibility/css-experimental-observations.json)
are reproduced with
`node scripts/check-css-experimental-observations.ts`. Chromium 151 with
`--enable-blink-features=CSSMixins` exposes `CSSApplyMixinRule`,
`CSSContentsMixinRule`, and `CSSResultRule`; it drops direct mixin declarations
that the selected draft allows. It is evidence of a different experimental
contract, not the oracle for SheetOM's draft interfaces.

The draft has these explicit resolutions:

| Issue | Authoring behavior |
| --- | --- |
| Old references to an `@contents` parameter, despite grammar allowing any mixin to receive a contents block | `CSSMixinRule.contents` is true; parameters contain only declared dashed parameters. |
| Old `@result` examples alongside the current direct-body grammar | Follow direct-body grammar; declaration runs become `CSSNestedDeclarations`. |
| Mixin serialization algorithm omits a space before the opening brace and emits spaces in empty blocks | Observable `cssText` follows the literal algorithm, including `@mixin --x{  }`. |
| A single empty free-form argument serializes ambiguously as `()` | Observable `cssText` follows the draft; SheetOM's safe/strict serializer uses `({ })` to preserve argument cardinality on reparse. |
| `@private` has syntax but no CSSOM interface in this revision | Retain its authored body as an opaque rule in valid nested contexts. It can be replaced through grouping operations; no descriptor API, filtering/evaluation behavior, or invented `CSSPrivateRule` is claimed. |

The supporting [CSS Values free-form argument grammar](https://drafts.csswg.org/css-values-5/#component-function-commas)
requires braces to enclose an entire argument and does not permit top-level
semicolons or `!` inside a brace wrapper. Nested functions retain their own
punctuation. The parser validates nested error tokens and preserves custom
property blocks without splitting declaration runs at function-internal braces.

## CSS utilities and resource behavior

`CSS.escape()` implements identifier serialization on DOMStrings, including
lone UTF-16 surrogates. `CSS.supports()` checks declaration or conditional syntax
against the authoring engine without a DOM. Its property acceptance uses the
same declaration authority as mutation; it does not evaluate a media condition
or a mixin. Selector and font capability results follow the pinned Chromium
policy. In particular, multi-language `:lang()` queries and unsupported font
technologies return false. Existing future selector retention is unaffected.

The current [CSS Easing draft](https://drafts.csswg.org/css-easing-2/#linear-easing-function-serializing)
and Chromium disagree on whether omitted `linear()` input positions are exposed
in serialization. Stable observable behavior follows the measured Chromium
normalization, as does the existing property contract. The pinned browser
probes cover omitted positions, double positions, clamping, and calculated
percentages; no animation execution is performed.

Boolean supports evaluation and mixin rule assembly/serialization are iterative.
Deep selector capability checks validate nested pseudo-class arguments from the
inside out, preserving nested `:has()` restrictions, before checking the outer
selector. This avoids both the WASM linear-memory stack and the host VM stack.
Resource-limit rejection leaves the instance usable. The existing WASM binary
size, memory, and browser performance limits remain unchanged.

Completion claims are bounded to this dated authoring target and these tested
contracts. This is not an all-WPT result, a release/deployment record, or a proof
for every possible CSS string. Exact execution outcomes belong to the generated
reports and the current change's validation record, not the immutable 0.1.1
release baseline.

## Initial PR validation record — 2026-09-08

Validated locally on Linux x64 with Node 26.8.1 and engine ABI 5. The initial PR's
native/WASM syntax-engine hash was
`7d2977632974a2f86edbafb58fbd71174a3edd76283fdd234e6e835a9fb756ce`.

| Gate | Result |
| --- | --- |
| `npm run check` | Repository/ABI checks, TypeScript, 285 unit tests, conformance manifests, generated catalogs, TypeDoc, build, runtime size, and packed install passed. |
| `npm run native:core-check` | Formatting, Clippy, and 237 core tests passed. |
| `npm run native:vendor-check` | cssparser, cssparser-color, and 182 Lightning CSS tests passed. |
| `npm run css:target:check` | 246 probes, 11,869 supports checks and 264 escape checks per backend: zero mismatches. Both backends passed 14 shared contracts and depth 256/1024/4000 checks. |
| `node scripts/check-webref-property-branches.ts` and `--wasm` | Each backend passed 11,590 checks across 666 properties, 371 profiles, and 8,369 grammar branches with zero mismatches. |
| Native browser differentials | Declaration, value, numeric, relative-color, longhand, geometric, font-face, and rule suites passed against Chromium 151.0.7922.34. Rule coverage includes 27 trees, 114 custom-function preludes, and 2,520 combinatorial preludes. |
| Native crash-safety suite | 149 subprocess cases passed. |
| WASM build/check/backend | Package checks and ArrayBuffer/Response/Module initialization and isolation passed; binary is 4,483,341 bytes raw and 1,316,286 bytes gzip, within existing limits. |
| WASM browser/bundler checks | Main-thread and worker checks passed in Chromium, Firefox, and WebKit, including Vite, esbuild, Rollup, and Webpack. |
| WASM memory/performance checks | 36-cycle memory soak and existing browser performance limits passed. |

The final namespace correction was followed by the repository check, both
backend target suites, rule browser differential, and three-engine WASM browser
smoke checks. Rust/vendor, property corpus, bundler, memory, and performance
gates passed earlier on the same syntax-engine hash. WebKit used an existing
local compatibility-library bundle through a temporary launch override; no
repository or global browser workaround was added. The minor Changeset parses
successfully; this record does not assert a commit, release, or deployment.

## Namespace and round-trip follow-up — 2026-09-08

Revision `.79` retains ABI 5 and has syntax-engine hash
`0cdb7affa3110506b1e8c759d5fb9cef1e46f0baaab6e9767768d83d456cdd5c`.
The repository check passes with 288 unit tests. Formatting/Clippy, 237 core
tests, and the normal vendor gate pass, including 185 Lightning CSS tests.
Both backends pass 259 browser probes, 11,881 supports checks, 264 escape checks,
17 shared contracts, and the depth 256/1024/4000 cases. The Webref corpus passes
11,590 checks per backend with zero mismatches. Native browser differentials
and WASM main-thread/worker checks in Chromium, Firefox, and WebKit also pass.
The WASM binary remains within budget at 4,486,081 bytes raw / 1,320,359 gzip.

An additional standalone `parcel_selectors` test run reports 6 passing tests and
one failure in the dummy parser's `foo::details-content` fixture. The identical
failure was reproduced using the untouched selector sources from parent commit
`e1e9726`; it is outside the normal vendor gate and is not introduced by this
follow-up. The actual Lightning CSS parser's namespace tests pass. This extra
suite limitation remains explicit rather than being counted as a green result.
