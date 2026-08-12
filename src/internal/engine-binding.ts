import generatedEngineAbiIdentity from "../../engine-abi.json" with { type: "json" };

type EngineBudgetArguments = [number, number, number, number, number];

export interface EngineAbiIdentity {
  abiVersion: number;
  sheetomVersion: string;
  syntaxEngineSetSha256: string;
}

export const expectedEngineAbiIdentity: Readonly<EngineAbiIdentity> = Object.freeze({
  abiVersion: generatedEngineAbiIdentity.abiVersion,
  sheetomVersion: generatedEngineAbiIdentity.sheetomVersion,
  syntaxEngineSetSha256: generatedEngineAbiIdentity.syntaxEngineSetSha256,
});

export type EngineMutationOutcome =
  | "applied"
  | "invalid-name"
  | "invalid-priority"
  | "invalid-value"
  | "unsupported-shorthand";

export interface EngineDeclarationStateHandle {
  readonly length: number;
  readonly cssText: string;
  item(index: number): string;
  getPropertyValue(name: string): string;
  getPropertyPriority(name: string): string;
  setProperty(
    name: string,
    value: string,
    priority: string,
    reservedNestingDepth?: number,
  ): EngineMutationOutcome;
  removeProperty(name: string): string;
  replaceCssText(source: string, reservedNestingDepth?: number): void;
  serializeFormatted(safe: boolean, indent: string, separator: string): string;
}

export interface EngineBindingIdentityProvider {
  engineAbiIdentity(): string;
}

/** Transport-neutral private contract implemented by native and WebAssembly engines. */
export interface EngineBinding extends EngineBindingIdentityProvider {
  normalizeMedia(source: string, ...budget: EngineBudgetArguments): string;
  normalizeSelector(source: string, ...budget: EngineBudgetArguments): string;
  normalizeSupports(source: string, ...budget: EngineBudgetArguments): string;
  parseContainerPreludeJson(source: string, ...budget: EngineBudgetArguments): string;
  parseCounterStyleDescriptorValue(
    name: string,
    value: string,
    ...budget: EngineBudgetArguments
  ): string | null;
  parseCounterStyleDescriptorsJson(source: string, ...budget: EngineBudgetArguments): string;
  parseCounterStyleNameJson(source: string, ...budget: EngineBudgetArguments): string | null;
  serializeIdentifierValue(value: string): string;
  serializeFontFamilyValue(value: string): string;
  createDeclarationState(
    context?: "style" | "font-face" | "function",
    ...budget: EngineBudgetArguments
  ): EngineDeclarationStateHandle;
  parseRecoveredRuleTreeJson(source: string, ...budget: EngineBudgetArguments): string;
  parseRecoveredSingleRuleTreeJson(source: string, ...budget: EngineBudgetArguments): string;
  parseStylesheetTreeJson(
    source: string,
    errorRecovery: boolean,
    ...budget: EngineBudgetArguments
  ): string;
  parseScopePreludeJson(source: string, ...budget: EngineBudgetArguments): string;
  scanTopLevelRulesJson(source: string, ...budget: EngineBudgetArguments): string;
}

export class SheetOMEngineBindingError extends Error {
  readonly code: "SHEETOM_ENGINE_ABI_INVALID" | "SHEETOM_ENGINE_ABI_MISMATCH";

  constructor(
    code: "SHEETOM_ENGINE_ABI_INVALID" | "SHEETOM_ENGINE_ABI_MISMATCH",
    message: string,
    options?: ErrorOptions,
  ) {
    super(message, options);
    this.name = "SheetOMEngineBindingError";
    this.code = code;
  }
}

export function validateEngineBindingIdentity(provider: EngineBindingIdentityProvider): void {
  let parsed: unknown;
  try {
    parsed = JSON.parse(provider.engineAbiIdentity());
  } catch (cause) {
    throw new SheetOMEngineBindingError(
      "SHEETOM_ENGINE_ABI_INVALID",
      "SheetOM could not decode the engine ABI identity.",
      { cause },
    );
  }

  if (!isEngineAbiIdentity(parsed)) {
    throw new SheetOMEngineBindingError(
      "SHEETOM_ENGINE_ABI_INVALID",
      "SheetOM received an invalid engine ABI identity.",
    );
  }
  if (
    parsed.abiVersion === expectedEngineAbiIdentity.abiVersion
    && parsed.sheetomVersion === expectedEngineAbiIdentity.sheetomVersion
    && parsed.syntaxEngineSetSha256 === expectedEngineAbiIdentity.syntaxEngineSetSha256
  ) return;

  throw new SheetOMEngineBindingError(
    "SHEETOM_ENGINE_ABI_MISMATCH",
    "SheetOM refused an incompatible engine binding.",
  );
}

function isEngineAbiIdentity(value: unknown): value is EngineAbiIdentity {
  if (typeof value !== "object" || value === null) return false;
  const identity = value as Partial<EngineAbiIdentity>;
  return Number.isSafeInteger(identity.abiVersion)
    && typeof identity.sheetomVersion === "string"
    && /^[0-9a-f]{64}$/u.test(identity.syntaxEngineSetSha256 ?? "");
}
