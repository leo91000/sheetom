# @sheetom/wasm

Explicit, ESM-only WebAssembly backend for [SheetOM](https://www.npmjs.com/package/sheetom).

```js
import { createSheetOM } from "@sheetom/wasm";

const { CSSStyleSheet } = await createSheetOM();
const sheet = new CSSStyleSheet();
sheet.insertRule(".card { color: red; }");
```

The default factory resolves `sheetom_wasm_bg.wasm` relative to the package
module, initializes it once per JavaScript realm, and shares concurrent calls:

```js
const [first, second] = await Promise.all([createSheetOM(), createSheetOM()]);
console.log(first === second); // true
```

Serve the `.wasm` asset with `Content-Type: application/wasm` to use streaming
instantiation. The loader falls back to buffered instantiation when the server
uses another MIME type. It requires neither WASI nor cross-origin isolation and
works in a module worker through the same async factory.

Pass a `URL`, `Response`, `ArrayBuffer`, or precompiled `WebAssembly.Module` to
create an independent backend:

```js
const response = await fetch(new URL("./engine.wasm", import.meta.url));
const isolated = await createSheetOM(response);
```

Each independent facade has distinct class identity, so use constructors from
the same returned object for `instanceof`. Unexpected WebAssembly traps poison
only that backend and become `SheetOMWasmBindingError` errors; ordinary CSS
validation and Resource Budget errors keep the same transactional behavior as
the native package.

The package never acts as an automatic fallback for `sheetom`, never installs
native binaries, and does not modify `globalThis`. Bundlers must copy the
external `.wasm` asset instead of embedding it into JavaScript. See the root
project documentation for the full API, resource-budget, and compatibility
contracts.
