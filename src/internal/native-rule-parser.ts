import { nativeBinding } from "./native-binding.js";
import {
  defaultResourceBudget,
  nativeBudgetArguments,
  rethrowResourceBudgetError,
  type NativeResourceBudget,
} from "./resource-budget.js";

export interface NativeRuleDescription {
  kind: string;
  prelude: string;
  declarations: string;
  children: NativeRuleDescription[];
  cssText: string;
}

export function parseNativeRule(
  source: string,
  resourceBudget: NativeResourceBudget = defaultResourceBudget,
): NativeRuleDescription | null {
  return parseNativeRulePayload(() => nativeBinding.parseRecoveredRuleTreeJson(
    source,
    ...nativeBudgetArguments(resourceBudget),
  ));
}

export function parseNativeRuleWithErrorRecovery(
  source: string,
  resourceBudget: NativeResourceBudget = defaultResourceBudget,
): NativeRuleDescription | null {
  return parseNativeRulePayload(() => nativeBinding.parseRecoveredSingleRuleTreeJson(
    source,
    ...nativeBudgetArguments(resourceBudget),
  ));
}

/*
 * Keep native parse failures as CSS parse failures, but never reinterpret a
 * transport/validation failure as an empty or invalid rule. Doing so would
 * silently drop valid deeply nested CSS during replaceSync().
 */
function parseNativeRulePayload(readPayload: () => string): NativeRuleDescription | null {
  let payload: string;
  try {
    payload = readPayload();
  } catch (error) {
    rethrowResourceBudgetError(error);
    return null;
  }

  let parsed: unknown;
  try {
    parsed = JSON.parse(payload);
  } catch (error) {
    throw nativeProtocolError("could not decode the owned rule tree", error);
  }

  try {
    return isNativeRuleDescription(parsed) ? parsed : null;
  } catch (error) {
    throw nativeProtocolError("could not validate the owned rule tree", error);
  }
}

function nativeProtocolError(message: string, cause: unknown): Error {
  return new Error(`SHEETOM_NATIVE_PROTOCOL_ERROR: ${message}`, { cause });
}

function isNativeRuleDescription(value: unknown): value is NativeRuleDescription {
  if (typeof value !== "object" || value === null) return false;
  const candidate = value as Partial<NativeRuleDescription>;
  if (
    typeof candidate.kind !== "string"
    || typeof candidate.prelude !== "string"
    || typeof candidate.declarations !== "string"
    || typeof candidate.cssText !== "string"
    || !Array.isArray(candidate.children)
  ) return false;
  return candidate.children.every(isNativeRuleDescription);
}
