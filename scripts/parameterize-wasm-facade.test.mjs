import assert from "node:assert/strict";
import test from "node:test";

import { parameterizeWasmFacade } from "./parameterize-wasm-facade.mjs";

const representativeFacade = `
const engineBinding = globalThis.__SHEETOM_WASM_ENGINE_BINDING__;
class CSSStyleSheet { constructor() { this.binding = engineBinding; } }
function parseStyleSheet() { return new CSSStyleSheet(); }
export { CSSStyleSheet, parseStyleSheet };
//# sourceMappingURL=facade.js.map
`;

test("the shared facade becomes an Engine Binding factory", () => {
  const transformed = parameterizeWasmFacade(representativeFacade);
  assert.match(transformed, /export function createSheetOMFacade\(engineBinding\)/u);
  assert.match(transformed, /Object\.freeze\(\{ CSSStyleSheet, parseStyleSheet \}\)/u);
  assert.doesNotMatch(transformed, /__SHEETOM_WASM_ENGINE_BINDING__/u);
  assert.doesNotMatch(transformed, /sourceMappingURL/u);
});

test("the facade transform fails closed on imports and surface drift", () => {
  assert.throws(
    () => parameterizeWasmFacade(`import "x";\n${representativeFacade}`),
    /unexpected runtime import/u,
  );
  assert.throws(
    () => parameterizeWasmFacade(representativeFacade.replaceAll("CSSStyleSheet", "StyleSheet")),
    /required SheetOM exports/u,
  );
});
