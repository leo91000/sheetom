import {
  registerEngineBinding,
  revokeEngineBinding,
} from "./binding-registry.js";
import {
  initializeWasmEngineBinding,
  SheetOMWasmBindingError,
  type SheetOMWasmSource,
} from "./wasm-adapter.js";

export { SheetOMWasmBindingError } from "./wasm-adapter.js";
export type { SheetOMWasmSource } from "./wasm-adapter.js";

export type SheetOMFacade = Readonly<typeof import("./facade.js")>;

let nextInstanceId = 0;
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
  nextInstanceId += 1;
  const instanceToken = nextInstanceId.toString(36);
  const binding = await initializeWasmEngineBinding(source, instanceToken, onTrap);
  const bindingToken = registerEngineBinding(binding);
  const facadeUrl = new URL("./facade.js", import.meta.url);
  facadeUrl.searchParams.set("sheetom-binding", bindingToken);
  try {
    const facade = await import(
      /* @vite-ignore */
      /* webpackIgnore: true */
      facadeUrl.href
    );
    return Object.freeze({ ...facade }) as SheetOMFacade;
  } finally {
    revokeEngineBinding(bindingToken);
  }
}
