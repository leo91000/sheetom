import { engineBinding } from "./default-engine-binding.js";
import {
  defaultResourceBudget,
  nativeBudgetArguments,
  rethrowResourceBudgetError,
  type NativeResourceBudget,
} from "./resource-budget.js";

export interface NativeContainerPrelude {
  conditionText: string;
  name: string;
  query: string;
}

export interface NativeScopePrelude {
  start: string | null;
  end: string | null;
}

export function normalizeNativeSelector(
  source: string,
  resourceBudget: NativeResourceBudget = defaultResourceBudget,
): string | null {
  return callNative(() => engineBinding.normalizeSelector(
    source,
    ...nativeBudgetArguments(resourceBudget),
  ));
}

export function normalizeNativeMedia(
  source: string,
  resourceBudget: NativeResourceBudget = defaultResourceBudget,
): string | null {
  return callNative(() => engineBinding.normalizeMedia(
    source,
    ...nativeBudgetArguments(resourceBudget),
  ));
}

export function normalizeNativeSupports(
  source: string,
  resourceBudget: NativeResourceBudget = defaultResourceBudget,
): string | null {
  return callNative(() => engineBinding.normalizeSupports(
    source,
    ...nativeBudgetArguments(resourceBudget),
  ));
}

export function parseNativeContainerPrelude(
  source: string,
  resourceBudget: NativeResourceBudget = defaultResourceBudget,
): NativeContainerPrelude | null {
  const parsed = callNative(() => engineBinding.parseContainerPreludeJson(
    source,
    ...nativeBudgetArguments(resourceBudget),
  ));
  if (parsed === null) return null;
  try {
    const value: unknown = JSON.parse(parsed);
    if (typeof value !== "object" || value === null) return null;
    const candidate = value as Partial<NativeContainerPrelude>;
    if (
      typeof candidate.conditionText !== "string"
      || typeof candidate.name !== "string"
      || typeof candidate.query !== "string"
    ) return null;
    return candidate as NativeContainerPrelude;
  } catch {
    return null;
  }
}

export function parseNativeScopePrelude(
  source: string,
  resourceBudget: NativeResourceBudget = defaultResourceBudget,
): NativeScopePrelude | null {
  const parsed = callNative(() => engineBinding.parseScopePreludeJson(
    source,
    ...nativeBudgetArguments(resourceBudget),
  ));
  if (parsed === null) return null;
  try {
    const value: unknown = JSON.parse(parsed);
    if (typeof value !== "object" || value === null) return null;
    const candidate = value as Partial<NativeScopePrelude>;
    if (
      candidate.start !== null && typeof candidate.start !== "string"
      || candidate.end !== null && typeof candidate.end !== "string"
    ) return null;
    return candidate as NativeScopePrelude;
  } catch {
    return null;
  }
}

function callNative(operation: () => string): string | null {
  try {
    return operation();
  } catch (error) {
    rethrowResourceBudgetError(error);
    return null;
  }
}
