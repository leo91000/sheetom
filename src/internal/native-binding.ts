import { createRequire } from "node:module";
import path from "node:path";
import { fileURLToPath } from "node:url";

type NativeBudgetArguments = [number, number, number, number, number];

export type NativeMutationOutcome =
  | "applied"
  | "invalid-name"
  | "invalid-priority"
  | "invalid-value"
  | "unsupported-shorthand";

export interface NativeDeclarationStateHandle {
  readonly length: number;
  readonly cssText: string;
  item(index: number): string;
  getPropertyValue(name: string): string;
  getPropertyPriority(name: string): string;
  setProperty(name: string, value: string, priority: string): NativeMutationOutcome;
  removeProperty(name: string): string;
  replaceCssText(source: string): void;
  serializeFormatted(safe: boolean, indent: string, separator: string): string;
}

interface NativeBinding {
  normalizeMedia(source: string, ...budget: NativeBudgetArguments): string;
  normalizeSelector(source: string, ...budget: NativeBudgetArguments): string;
  normalizeSupports(source: string, ...budget: NativeBudgetArguments): string;
  parseContainerPreludeJson(source: string, ...budget: NativeBudgetArguments): string;
  parseCounterStyleDescriptorValue(
    name: string,
    value: string,
    ...budget: NativeBudgetArguments
  ): string | null;
  parseCounterStyleDescriptorsJson(source: string, ...budget: NativeBudgetArguments): string;
  parseCounterStyleNameJson(source: string, ...budget: NativeBudgetArguments): string | null;
  serializeIdentifierValue(value: string): string;
  serializeFontFamilyValue(value: string): string;
  NativeDeclarationState: new (
    context?: "style" | "font-face" | "function",
    ...budget: NativeBudgetArguments
  ) => NativeDeclarationStateHandle;
  parseRecoveredRuleTreeJson(source: string, ...budget: NativeBudgetArguments): string;
  parseRecoveredSingleRuleTreeJson(source: string, ...budget: NativeBudgetArguments): string;
  parseStylesheetTreeJson(
    source: string,
    errorRecovery: boolean,
    ...budget: NativeBudgetArguments
  ): string;
  parseScopePreludeJson(source: string, ...budget: NativeBudgetArguments): string;
  scanTopLevelRulesJson(source: string, ...budget: NativeBudgetArguments): string;
}

function loadBinding(): NativeBinding {
  const moduleDirectory = path.dirname(fileURLToPath(import.meta.url));
  const packageRoot = path.basename(moduleDirectory) === "dist"
    ? path.dirname(moduleDirectory)
    : path.resolve(moduleDirectory, "../..");
  const require = createRequire(import.meta.url);
  return require(path.join(packageRoot, "native", "index.cjs")) as NativeBinding;
}

export const nativeBinding = loadBinding();
