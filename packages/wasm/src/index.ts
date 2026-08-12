import { createSheetOMFacade } from "./facade_factory.js";
import {
  initializeWasmEngineBinding,
  SheetOMWasmBindingError,
  type SheetOMWasmSource,
} from "./wasm-adapter.js";

export { SheetOMWasmBindingError } from "./wasm-adapter.js";
export type { SheetOMWasmSource } from "./wasm-adapter.js";

export type SheetOMFacade = Readonly<typeof import("./facade.js")>;

let defaultFacade: Promise<SheetOMFacade> | undefined;
let defaultPoison: SheetOMWasmBindingError | undefined;

export function createSheetOM(source?: SheetOMWasmSource): Promise<SheetOMFacade> {
  if (source !== undefined) return createIndependentFacade(source, () => {});
  if (defaultPoison) return Promise.reject(defaultPoison);
  if (defaultFacade) return defaultFacade;

  const defaultSource = new URL("./sheetom_wasm_bg.wasm", import.meta.url);
  defaultFacade = createIndependentFacade(defaultSource, error => {
    defaultPoison = error;
  }).catch(error => {
    if (!defaultPoison) defaultFacade = undefined;
    throw error;
  });
  return defaultFacade;
}

async function createIndependentFacade(
  source: SheetOMWasmSource,
  onTrap: (error: SheetOMWasmBindingError) => void,
): Promise<SheetOMFacade> {
  const binding = await initializeWasmEngineBinding(source, onTrap);
  return createSheetOMFacade(binding) as SheetOMFacade;
}
