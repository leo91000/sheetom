import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";

import {
  createSheetOM,
  SheetOMWasmBindingError,
} from "../packages/wasm/dist/index.js";

const wasmPath = new URL(
  "../packages/wasm/dist/sheetom_wasm_bg.wasm",
  import.meta.url,
);
const bytes = await readFile(wasmPath);
const arrayBuffer = bytes.buffer.slice(
  bytes.byteOffset,
  bytes.byteOffset + bytes.byteLength,
);

await assert.rejects(
  createSheetOM(new Uint8Array(arrayBuffer)),
  error => error instanceof SheetOMWasmBindingError
    && error.code === "SHEETOM_WASM_SOURCE_INVALID",
);

const trappingModule = new WebAssembly.Module(Buffer.from(
  "AGFzbQEAAAABEANgAX8Bf2ADf39/AGABfwADBAMAAQIFAwEAAQYIAX8BQYCABAsHVQQGbWVtb3J5AgAfX193YmluZGdlbl9hZGRfdG9fc3RhY2tfcG9pbnRlcgAAEl9fd2JpbmRnZW5fZXhwb3J0MwABEWVuZ2luZUFiaUlkZW50aXR5AAIKFAMLACMAIABqJAAjAAsCAAsDAAAL",
  "base64",
));
await assert.rejects(
  createSheetOM(trappingModule),
  error => error instanceof SheetOMWasmBindingError
    && error.code === "SHEETOM_WASM_TRAP",
);

const response = new Response(arrayBuffer, {
  headers: { "content-type": "application/wasm" },
});
const module = await WebAssembly.compile(arrayBuffer);
const [fromBytes, fromResponse, fromModule] = await Promise.all([
  createSheetOM(arrayBuffer),
  createSheetOM(response),
  createSheetOM(module),
]);

assert.ok(Object.isFrozen(fromBytes));
assert.notEqual(fromBytes.CSSStyleSheet, fromResponse.CSSStyleSheet);
assert.notEqual(fromResponse.CSSStyleSheet, fromModule.CSSStyleSheet);

for (const api of [fromBytes, fromResponse, fromModule]) {
  const sheet = new api.CSSStyleSheet({ diagnostics: true });
  sheet.replaceSync(`
    @layer components {
      .card {
        background: image-set(url(a.png) 1x, url(b.png) 2x) center / cover no-repeat red;
      }
    }
  `);
  assert.equal(sheet.cssRules.length, 1);
  const layer = sheet.cssRules[0];
  const rule = layer.cssRules[0];
  rule.style.setProperty("padding", "72px var(--space, var(--space,");
  const mutationResults = rule.style.applyMutations([
    { kind: "set", property: "place-content", value: "center space-between" },
    { kind: "set", property: "align-content", value: "safe center" },
    { kind: "set", property: "width", value: "20px; color: red" },
    { kind: "remove", property: "justify-content" },
  ]);
  assert.deepEqual(
    mutationResults.map(result => result.kind === "set" ? result.accepted : result.value),
    [true, true, false, "space-between"],
  );
  assert.equal(rule.style.getPropertyValue("align-content"), "safe center");
  assert.equal(rule.style.getPropertyValue("justify-content"), "");
  assert.match(sheet.serialize(), /image-set\(/u);
  assert.match(sheet.serialize(), /var\(--space/u);
  assert.equal(sheet.serialize(), sheet.serialize());
}

const limitedSheet = new fromBytes.CSSStyleSheet({
  resourceBudget: { maxStylesheetBytes: 8 },
});
assert.throws(
  () => limitedSheet.replaceSync(".too-large { color: red; }"),
  RangeError,
);

console.log("Verified isolated ArrayBuffer, Response, and Module WASM backends.");
