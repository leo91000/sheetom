import type { EngineBinding } from "../../../src/internal/engine-binding.js";

// The build-only global is replaced by the generated facade factory parameter.
declare global {
  var __SHEETOM_WASM_ENGINE_BINDING__: EngineBinding;
}

export const engineBinding = globalThis.__SHEETOM_WASM_ENGINE_BINDING__;
