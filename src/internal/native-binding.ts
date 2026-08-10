import { createRequire } from "node:module";
import path from "node:path";
import { fileURLToPath } from "node:url";

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
  normalizeMedia(source: string): string;
  normalizeSelector(source: string): string;
  normalizeSupports(source: string): string;
  parseContainerPreludeJson(source: string): string;
  parseCounterStyleDescriptorValue(name: string, value: string): string | null;
  parseCounterStyleDescriptorsJson(source: string): string;
  parseCounterStyleNameJson(source: string): string | null;
  NativeDeclarationState: new (
    context?: "style" | "font-face",
  ) => NativeDeclarationStateHandle;
  parseRecoveredRuleTreeJson(source: string): string;
  parseScopePreludeJson(source: string): string;
  scanTopLevelRulesJson(source: string): string;
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
