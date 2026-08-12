import type { EngineBinding } from "../../../src/internal/engine-binding.js";

const registeredBindings = new Map<string, EngineBinding>();
let nextBindingId = 0;

export function registerEngineBinding(binding: EngineBinding): string {
  nextBindingId += 1;
  const token = nextBindingId.toString(36);
  registeredBindings.set(token, binding);
  return token;
}

export function revokeEngineBinding(token: string): void {
  registeredBindings.delete(token);
}

export function takeEngineBinding(moduleUrl: string): EngineBinding {
  const token = new URL(moduleUrl).searchParams.get("sheetom-binding");
  const binding = token === null ? undefined : registeredBindings.get(token);
  if (token !== null) registeredBindings.delete(token);
  if (binding) return binding;
  throw new Error("SHEETOM_WASM_BINDING_MISSING: facade engine binding was not registered");
}
