import { nativeBinding } from "./native-binding.js";

/** Consumes top-level CSS Syntax rules and retains their exact UTF-8 spans. */
export function scanTopLevelRules(css: string): string[] {
  const parsed: unknown = JSON.parse(nativeBinding.scanTopLevelRulesJson(css));
  if (!Array.isArray(parsed) || parsed.some(rule => typeof rule !== "string")) {
    throw new TypeError("SHEETOM_NATIVE_PROTOCOL: invalid top-level rule scan result");
  }
  return parsed;
}
