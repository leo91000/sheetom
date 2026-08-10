# SheetOM

SheetOM is a mutable, browser-shaped CSS authoring object model for Node.js,
Bun, and Deno. Its repository-owned Rust engine handles parsing,
canonicalization, declaration state, and safe CSS serialization; JavaScript
only owns the public WebIDL-shaped facade and live object identity.

It is intended for server-side stylesheet editing and serialization. It does
not implement the DOM, cascade, selector matching, layout, or computed styles.

## Install

Install the active release:

```sh
npm install sheetom
```

After the first stable release, use `npm install sheetom@next` to opt into an
active prerelease. Until then, the `latest` and `next` dist-tags both identify
the active release candidate.

The package has no JavaScript runtime or peer dependencies. Its native engine
includes a pinned, locally maintained Lightning CSS source snapshot; consumers
do not install a second parser, provide globals, or select a fallback engine.
SheetOM owns observable-value serialization and shorthand expansion rather
than delegating CSSOM state to a browser-DOM emulation package.

Node.js 22 and 24 are tested on Linux x64, Windows x64, and macOS arm64. Bun
1.3.1 and Deno 2.9.5 are tested on Linux x64. Deno must use a local
`node_modules` directory and grant SheetOM's native binding FFI and
system-information permissions:

```sh
deno run --node-modules-dir=manual --allow-ffi --allow-sys app.ts
```

SheetOM exports browser-named classes but never modifies `globalThis`.

## Use

```ts
import { CSSStyleRule, CSSStyleSheet, parseStyleSheet } from "sheetom";

const sheet = new CSSStyleSheet({ diagnostics: true });
sheet.replaceSync(".card { width: 12px; }");

const rule = sheet.cssRules[0];
if (rule instanceof CSSStyleRule) {
  rule.style.setProperty("padding", "8px 16px");
  rule.style.setProperty("background-color", "rebeccapurple");
}

console.log(sheet.serialize());

const existing = parseStyleSheet('@import "theme.css"; .card { color: red; }', {
  href: "https://example.com/assets/app.css",
});
console.log(existing.cssRules.length); // 2
```

`setProperty()` is the standard, fully typed way to set a declaration,
especially when its name is dynamic. Browser-style camelCase assignments such
as `rule.style.backgroundColor = "rebeccapurple"` work at runtime, but the
current TypeScript declarations do not enumerate every named CSS property.

Constructed sheets follow browser replacement behavior and strip `@import`.
`parseStyleSheet` creates a regular authoring sheet and preserves valid imports
without loading them.

## Recovered values and reparsable serialization

SheetOM distinguishes browser-facing CSSOM text from reparsable stylesheet
output. Chromium accepts some values using CSS Syntax end-of-input recovery
while preserving the unclosed input in `getPropertyValue()` and `cssText`:

```ts
const sheet = new CSSStyleSheet();
sheet.insertRule(".card {}");

const rule = sheet.cssRules[0];
if (!(rule instanceof CSSStyleRule)) {
  throw new TypeError("Expected a style rule");
}

rule.style.setProperty("padding", "72px var(--space, var(--space,");

rule.style.getPropertyValue("padding");
// "72px var(--space, var(--space,"

rule.style.cssText;
// "padding: 72px var(--space, var(--space,;"

sheet.serialize();
// reparsable CSS with the missing syntax repaired
```

The three text surfaces have distinct contracts:

| Surface | Contract |
| --- | --- |
| `rule.style.cssText` | Browser-compatible observable declaration text, including accepted recovery quirks. |
| `rule.cssText` | Browser-compatible observable text for one rule and its current descendants. |
| `sheet.serialize()` | Deterministic, reparsable stylesheet output with recoverable syntax confined to its declaration or rule. |

Reparsable serialization means that the output can be parsed again without one
declaration leaking into the next declaration or rule, while preserving the
valid semantic state owned by SheetOM. It does not sanitize URLs, enforce CSP,
block remote resources, or make valid hostile CSS safe to render.

Invalid `setProperty` values and priorities are atomic no-ops, matching browser
behavior. Opt into mutation diagnostics with `{ diagnostics: true }` and drain
them using `takeDiagnostics()`. `SheetOMDiagnostic.code` is a stable string
union. Diagnostics retain the complete rejected input until drained; when
processing untrusted or very large input, callers should leave diagnostics off
or drain them promptly to bound memory use.

Static shorthands are owned as canonical longhand records. Updating or removing
one longhand can therefore never reactivate an older shorthand value. Shorthand
getters and `cssText` are synthesized only while the complete current longhand
set has compatible values and priorities; genuinely deferred substitutions
retain separate provenance until a longhand mutation breaks their group.

The Chromium 151 compatibility baseline covers all 129 manifested
multi-longhand shorthands. Grammar depth is a finite reviewed contract rather
than a claim to implement every future CSS production: 24 codec profiles
currently carry 96 positive and neighboring-negative branch cases, including
cardinality, slash and comma lists, compound optional keywords, mutation
atomicity, and reparsable round-trips. CI reprobes those cases against the
pinned Chromium engine, and a release is blocked unless all 129 breadth seeds
and all 96 reviewed branches pass exactly. The contracts and browser
observations ship as audit evidence but are never imported as runtime value
authority.

## API scope

- `CSSStyleSheet`, live `CSSRuleList`, `CSSRule`, and `MediaList`
- nested `CSSStyleRule`, `CSSGroupingRule`, and `CSSConditionRule`
- media, supports, container, layer, scope, and starting-style rules
- import, font-face, page/margin, nested-declaration, and position-try rules
- keyframes and mutable keyframe rules
- counter-style descriptors and font-feature-value maps
- custom `@function` parameters, return types, conditional groups, and live
  function descriptors
- live indexed and named `CSSStyleDeclaration`
- `insertRule`, `deleteRule`, `replace`, and `replaceSync`
- `parseStyleSheet` for forgiving regular-sheet parsing
- browser-facing `cssText` and reparsable `serialize()` output
- generic retention of read-only metadata rules and experimental/future rules

SheetOM targets standards and shared browser behavior first, with Chromium
behavior used only as the final measured divergence fallback. Versioned JSON
Operation Fixtures execute through SheetOM and native browser adapters;
applicable WPT subtests retain their pinned source path, title, and blob SHA.
Every release ships its machine-readable Compatibility Report.

Modern or implementation-dependent value grammar is bounded by the checked-in
[`Value Capability Corpus`](./compatibility/value-capabilities.json). Each
measured family includes accepted and rejected neighboring cases; browser
probing occurs only during conformance work and never at runtime.

The corresponding shorthand depth evidence is split between the reviewed
[`Grammar Branch Contracts`](./compatibility/shorthand-grammar-contracts.json)
and the generated
[`Chromium observations`](./compatibility/shorthand-grammar-observations.json).

See [the behavioral API reference](./docs/api.md) for return, exception,
identity, and detachment contracts. Maintainers use the separately reviewed
[release procedure](./docs/releasing.md); Changesets never publishes by itself.

The checked-in ordinary-property manifest was generated against Chromium
151.0.7922.34. Run `npm run generate:properties` only when intentionally
advancing the Compatibility Baseline.

## Security and resource boundaries

SheetOM does not fetch `@import` targets or URLs and does not execute CSS. It is
also not a sanitizer: valid authored URLs, imports, and browser features remain
in serialized output. Callers must apply their own content and resource policy
before attaching untrusted output to a document or another rendering
environment.

The browser-shaped interface has no implicit input-size, nesting-depth, or
mutation-count limits. Isolate or bound untrusted workloads according to your
application's resource policy. See [SECURITY.md](./SECURITY.md) for private
vulnerability reporting and supported-release policy.

## Development

```sh
npm run check
npm run test:browser:matrix
npm run test:differential
npm run conformance:drift
npm run fuzz
npm run benchmark
```

`npm run check` covers types, public unit behavior, conformance documents, API
documentation, build output, runtime artifacts, and the packed-package smoke
test. Use `npm run test:browser:matrix` on a machine with all Playwright
dependencies to run Chromium, Firefox, and WebKit. CI separately runs
`npm run test:differential`, a deterministic, grammar-oriented declaration
differential with native-browser reparsing witnesses. Any minimized mismatch
must become an immutable Operation Fixture with an explicit Compatibility
Resolution.

`npm run docs:build` generates the TypeDoc reference under `site/api`. The
benchmark gates both a 10,000-rule stress case and a publisher-shaped reference
workload with one shared stylesheet, 20 page stylesheets, nested layers/media,
distributed mutations, and two idempotent serialization passes.

Only the latest published `0.x` minor and its active prereleases receive fixes
before 1.0. During `0.x`, observable compatibility corrections and interface
breaks require a new minor version and migration note; patches remain backward
compatible.

## License

MIT
