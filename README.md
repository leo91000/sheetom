# SheetOM

**A browser-compatible, mutable CSSOM for JavaScript outside the browser.**

SheetOM lets build tools, publishers, and server applications parse, inspect,
mutate, and serialize stylesheets with familiar browser APIs. It preserves
browser recovery behavior for malformed-but-accepted CSS while still producing
deterministic, reparsable stylesheet output.

- Browser-shaped `CSSStyleSheet`, rules, and declarations
- Transactional `setProperty()` behavior and correct shorthand/longhand state
- Forgiving whole-sheet parsing without a DOM or global installation
- One repository-owned Rust engine for native and WebAssembly backends
- ESM, CommonJS, TypeScript, Node.js, Bun, Deno, browsers, and workers

SheetOM focuses on the **Authoring CSSOM**. It does not implement the cascade,
selector matching, layout, computed styles, fetching, or DOM attachment.

## Install

For Node.js, Bun, or Deno:

```sh
npm install sheetom
```

For browsers and compatible JavaScript isolates:

```sh
npm install @sheetom/wasm
```

## Quick start

```ts
import { CSSStyleRule, CSSStyleSheet, parseStyleSheet } from "sheetom";

const sheet = new CSSStyleSheet({ diagnostics: true });
sheet.replaceSync(".card { width: 12px; }");

const rule = sheet.cssRules[0];
if (rule instanceof CSSStyleRule) {
  rule.style.padding = "8px 16px";
  rule.style.backgroundColor = "rebeccapurple";
  rule.style.setProperty("--card-gap", "12px");
}

console.log(sheet.serialize());

const existing = parseStyleSheet(
  '@import "theme.css"; .card { color: red; }',
  { href: "https://example.com/assets/app.css" },
);

console.log(existing.cssRules.length); // 2
```

Named CSS properties are typed and assignable directly. Use `setProperty()`
when a property name is dynamic, custom, or hyphenated. `CSSStyleSheet` is the
only public CSSOM constructor; rules and declarations come from a sheet, just
as they do in browsers.

Constructed sheets strip `@import` during replacement. `parseStyleSheet()` is a
SheetOM extension for existing regular stylesheets and preserves valid imports
without fetching them.

## Browser recovery, safe output

Browsers accept some CSS using end-of-input recovery and expose the incomplete
spelling through CSSOM. SheetOM keeps that observable behavior separate from
whole-sheet output:

```ts
const sheet = new CSSStyleSheet();
sheet.insertRule(".card {}");

const rule = sheet.cssRules[0];
if (!(rule instanceof CSSStyleRule)) throw new TypeError("Expected a style rule");

rule.style.setProperty("padding", "72px var(--space, var(--space,");

rule.style.getPropertyValue("padding");
// "72px var(--space, var(--space,"

rule.style.cssText;
// "padding: 72px var(--space, var(--space,;"

sheet.serialize();
// reparsable CSS with the missing syntax safely confined and repaired
```

| Surface | Purpose |
| --- | --- |
| `style.cssText` | Browser-compatible declaration text, including measured recovery quirks. |
| `rule.cssText` | Browser-compatible text for the current rule. |
| `sheet.serialize()` | Deterministic, reparsable CSS for a complete stylesheet. |

Invalid property values and priorities are atomic no-ops: the previous state
survives unchanged. Static shorthands are stored as canonical longhands, so
editing and then removing one longhand cannot accidentally reactivate an old
shorthand value.

Reparsable output prevents malformed input from leaking into a following
declaration or rule. It is not a CSS sanitizer, URL policy, CSP, or rendering
sandbox.

## Native and WebAssembly runtimes

The `sheetom` package contains the JavaScript facade and selects one exact
`@sheetom/native-*` optional package for the current platform. It installs no
JavaScript parser dependency, downloads no binary at install time, and never
falls back to a different engine. Omitting optional dependencies is unsupported
and fails explicitly.

| Runtime | Entry point | Notes |
| --- | --- | --- |
| Node.js 22+ | `sheetom` | ESM and CommonJS; prebuilt native packages for macOS, Windows, GNU Linux, and musl Linux targets. |
| Bun | `sheetom` | Tested through the npm package on Linux x64. |
| Deno | `sheetom` | Uses npm resolution and requires native FFI/system permissions. |
| Browsers and workers | `@sheetom/wasm` | ESM-only asynchronous factory; no WASI or global installation. |

Deno example:

```sh
deno run --node-modules-dir=manual --allow-ffi --allow-sys app.ts
```

WebAssembly example:

```ts
import { createSheetOM } from "@sheetom/wasm";

const { CSSStyleSheet } = await createSheetOM();
const sheet = new CSSStyleSheet();
sheet.replaceSync(".card { color: red; }");
```

The native and WebAssembly packages execute the same Rust engine and verify an
exact engine ABI identity before exposing CSSOM objects. `createSheetOM()`
shares default initialization within one JavaScript realm; passing an explicit
`URL`, `Response`, `ArrayBuffer`, or `WebAssembly.Module` creates an isolated
facade with its own class identity.

## Compatibility contract

SheetOM tests complete observable state—not only whether serialized CSS looks
similar. Its versioned evidence covers getters, `cssText`, indexed property
order, priorities, invalid-mutation atomicity, live identity, shorthand
sequences, recovery, native-browser reparsing, subprocess crash safety,
grammar-oriented fuzzing, and Publisher-shaped performance workloads.

Standards and shared browser behavior take precedence. When browser engines
genuinely differ, SheetOM records the result and follows the pinned Chromium
baseline as its final fallback. Browser observations and WPT mappings remain
test evidence; they are never imported as runtime authority.

See [Compatibility](./docs/compatibility.md) for the exact promise, evidence,
and exclusions.

## Resource limits and diagnostics

Each sheet has configurable limits for stylesheet bytes, declaration-value
bytes, syntax depth, rule count, and declarations per block. A limit violation
throws `RangeError` before mutation. These budgets protect the host process;
they are not content policy.

Mutation diagnostics are opt-in:

```ts
const sheet = new CSSStyleSheet({ diagnostics: true });
// ...mutate the sheet...
const diagnostics = sheet.takeDiagnostics();
```

Diagnostics retain rejected inputs until drained. Leave them disabled or drain
them promptly when handling untrusted or unusually large source.

## Documentation

- [API behavior](./docs/api.md)
- [Architecture](https://github.com/leo91000/sheetom/blob/main/docs/architecture.md)
- [Compatibility](https://github.com/leo91000/sheetom/blob/main/docs/compatibility.md)
- [Contributing](https://github.com/leo91000/sheetom/blob/main/CONTRIBUTING.md)
- [Release process](https://github.com/leo91000/sheetom/blob/main/docs/releasing.md)
- [Architecture decisions](https://github.com/leo91000/sheetom/blob/main/docs/adr/README.md)
- [Security policy](./SECURITY.md)

## License

MIT. Vendored parser sources retain their upstream licenses and provenance.
