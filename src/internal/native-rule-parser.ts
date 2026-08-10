import { nativeBinding } from "./native-binding.js";

export interface NativeRuleDescription {
  kind: string;
  prelude: string;
  declarations: string;
  children: NativeRuleDescription[];
  cssText: string;
}

export function parseNativeRule(source: string): NativeRuleDescription | null {
  try {
    const parsed: unknown = JSON.parse(nativeBinding.parseRecoveredRuleTreeJson(source));
    return isNativeRuleDescription(parsed) ? parsed : null;
  } catch {
    return null;
  }
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
