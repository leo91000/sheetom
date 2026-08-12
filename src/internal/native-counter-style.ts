import { engineBinding } from "./default-engine-binding.js";
import {
  defaultResourceBudget,
  nativeBudgetArguments,
  rethrowResourceBudgetError,
  type NativeResourceBudget,
} from "./resource-budget.js";

export interface NativeCounterStyleDescriptor {
  name: string;
  value: string;
}

export interface NativeCounterStyleName {
  name: string;
  serialized: string;
}

export function parseNativeCounterStyleName(
  source: string,
  resourceBudget: NativeResourceBudget = defaultResourceBudget,
): NativeCounterStyleName | null {
  try {
    const encoded = engineBinding.parseCounterStyleNameJson(
      source,
      ...nativeBudgetArguments(resourceBudget),
    );
    if (encoded === null) return null;
    const parsed: unknown = JSON.parse(encoded);
    if (typeof parsed !== "object" || parsed === null) return null;
    const candidate = parsed as Partial<NativeCounterStyleName>;
    if (typeof candidate.name !== "string" || typeof candidate.serialized !== "string") return null;
    return candidate as NativeCounterStyleName;
  } catch (error) {
    rethrowResourceBudgetError(error);
    return null;
  }
}

export function serializeNativeIdentifier(value: string): string {
  return engineBinding.serializeIdentifierValue(value);
}

export function serializeNativeFontFamily(value: string): string {
  return engineBinding.serializeFontFamilyValue(value);
}

export function parseNativeCounterStyleDescriptor(
  name: string,
  value: string,
  resourceBudget: NativeResourceBudget = defaultResourceBudget,
): string | null {
  try {
    return engineBinding.parseCounterStyleDescriptorValue(
      name,
      value,
      ...nativeBudgetArguments(resourceBudget),
    );
  } catch (error) {
    rethrowResourceBudgetError(error);
    return null;
  }
}

export function parseNativeCounterStyleDescriptors(
  source: string,
  resourceBudget: NativeResourceBudget = defaultResourceBudget,
): NativeCounterStyleDescriptor[] {
  try {
    const parsed: unknown = JSON.parse(engineBinding.parseCounterStyleDescriptorsJson(
      source,
      ...nativeBudgetArguments(resourceBudget),
    ));
    if (!Array.isArray(parsed)) return [];
    const descriptors: NativeCounterStyleDescriptor[] = [];
    for (const value of parsed) {
      if (typeof value !== "object" || value === null) continue;
      const candidate = value as Partial<NativeCounterStyleDescriptor>;
      if (typeof candidate.name !== "string" || typeof candidate.value !== "string") continue;
      descriptors.push(candidate as NativeCounterStyleDescriptor);
    }
    return descriptors;
  } catch (error) {
    rethrowResourceBudgetError(error);
    return [];
  }
}
