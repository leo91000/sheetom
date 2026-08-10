import { nativeBinding } from "./native-binding.js";
import {
  defaultResourceBudget,
  nativeBudgetArguments,
  rethrowResourceBudgetError,
  type NativeResourceBudget,
} from "./resource-budget.js";

/** Consumes top-level CSS Syntax rules and retains their exact UTF-8 spans. */
export function scanTopLevelRules(
  css: string,
  resourceBudget: NativeResourceBudget = defaultResourceBudget,
): string[] {
  let encoded: string;
  try {
    encoded = nativeBinding.scanTopLevelRulesJson(
      css,
      ...nativeBudgetArguments(resourceBudget),
    );
  } catch (error) {
    rethrowResourceBudgetError(error);
    throw error;
  }
  const parsed: unknown = JSON.parse(encoded);
  if (!Array.isArray(parsed) || parsed.some(rule => typeof rule !== "string")) {
    throw new TypeError("SHEETOM_NATIVE_PROTOCOL: invalid top-level rule scan result");
  }
  return parsed;
}
