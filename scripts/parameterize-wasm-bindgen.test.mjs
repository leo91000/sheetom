import assert from "node:assert/strict";
import test from "node:test";

import { parameterizeWasmBindgenGlue } from "./parameterize-wasm-bindgen.mjs";

const representativeGlue = `
export class WasmDeclarationState {}
export function engineAbiIdentity() { return wasm.identity(); }
let wasmModule, wasmInstance, wasm;
async function __wbg_init(module_or_path) {
  wasm = { identity: () => "test" };
  return wasm;
}
function initSync() {}
export { initSync, __wbg_init as default };
`;

test("wasm-bindgen glue becomes an instance-scoped static module", () => {
  const transformed = parameterizeWasmBindgenGlue(representativeGlue);
  assert.match(transformed, /export async function createWasmBindings/u);
  assert.match(transformed, /await __wbg_init\(\{ module_or_path \}\)/u);
  assert.match(transformed, /Object\.freeze\(\{ WasmDeclarationState, engineAbiIdentity \}\)/u);
  assert.doesNotMatch(transformed, /^export class/gmu);
  assert.doesNotMatch(transformed, /__wbg_init as default/u);
});

test("the transform fails closed when pinned wasm-bindgen output drifts", () => {
  assert.throws(
    () => parameterizeWasmBindgenGlue(representativeGlue.replace("let wasmModule", "let module")),
    /expected module state/u,
  );
  assert.throws(
    () => parameterizeWasmBindgenGlue(representativeGlue.replace(" as default", "")),
    /terminal export/u,
  );
});
