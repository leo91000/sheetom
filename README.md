# SheetOM

SheetOM is a mutable, browser-shaped CSS authoring object model for Node.js,
Bun, and Deno. It uses Lightning CSS for property parsing and canonicalization
while keeping declaration state and CSSOM serialization under its own control.

It is intended for server-side stylesheet editing and serialization. It does
not implement the DOM, cascade, selector matching, layout, or computed styles.

## Install

```sh
npm install sheetom
```

`lightningcss` and `css-tree` are regular dependencies; consumers do not need
to install peer dependencies or globals.

## Use

```ts
import { CSSStyleRule, CSSStyleSheet, parseStyleSheet } from "sheetom";

const sheet = new CSSStyleSheet({ diagnostics: true });
sheet.replaceSync(".card { width: 12px; }");

const rule = sheet.cssRules[0];
if (rule instanceof CSSStyleRule) {
  rule.style.setProperty("padding", "8px 16px");
  Reflect.set(rule.style, "backgroundColor", "rebeccapurple");
}

console.log(sheet.serialize());

const existing = parseStyleSheet('@import "theme.css"; .card { color: red; }', {
  href: "https://example.com/assets/app.css",
});
console.log(existing.cssRules.length); // 2
```

Constructed sheets follow browser replacement behavior and strip `@import`.
`parseStyleSheet` creates a regular authoring sheet and preserves valid imports
without loading them.

## Recovered values and safe serialization

SheetOM distinguishes the browser-facing CSSOM text from safe output. Chromium
accepts some values using CSS Syntax end-of-input recovery while preserving the
unclosed input in `getPropertyValue()` and `cssText`:

```ts
const sheet = new CSSStyleSheet();
sheet.insertRule(".card {}");

const rule = sheet.cssRules[0] as CSSStyleRule;
rule.style.setProperty("padding", "72px var(--space, var(--space,");

rule.style.getPropertyValue("padding");
// "72px var(--space, var(--space,"

rule.style.cssText;
// "padding: 72px var(--space, var(--space,;"

sheet.serialize();
// reparsable CSS with the missing syntax repaired
```

Invalid `setProperty` values and priorities are atomic no-ops, matching browser
behavior. Opt into mutation diagnostics with `{ diagnostics: true }` and drain
them using `takeDiagnostics()`.

## API scope

- `CSSStyleSheet`, live `CSSRuleList`, `CSSRule`, and `CSSStyleRule`
- live indexed and named `CSSStyleDeclaration`
- `insertRule`, `deleteRule`, `replace`, and `replaceSync`
- `parseStyleSheet` for forgiving regular-sheet parsing
- browser-facing `cssText` and reparsable `serialize()` output
- generic retention and serialization of valid at-rules that do not yet have a
  specialized interface

SheetOM targets standards and shared browser behavior first, with Chromium
behavior used for measured engine divergences. Compatibility is covered by both
Node-based Vitest tests and native Chromium tests through Vitest Browser Mode.
The checked-in ordinary-property manifest was generated against Chromium
151.0.7922.34. Run `npm run generate:properties` when intentionally advancing
that release baseline.

## Development

```sh
npm test
npm run typecheck
npm run build
npm pack --dry-run
```

`npm test` runs both the unit suite and the real-browser compatibility suite.
Use `npm run test:unit` or `npm run test:browser` to run either project alone.

## License

MIT
