import type {
  EngineBinding,
  EngineDeclarationStateHandle,
  EngineMutationOutcome,
} from "../../../src/internal/engine-binding.js";
import { validateEngineBindingIdentity } from "../../../src/internal/engine-binding.js";

type BudgetArguments = [number, number, number, number, number];

interface GeneratedDeclarationState {
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
  ): string;
  removeProperty(name: string): string;
  replaceCssText(source: string, reservedNestingDepth?: number): void;
  serializeFormatted(safe: boolean, indent: string, separator: string): string;
  free(): void;
}

interface WasmGlueModule {
  default(input: { module_or_path: SheetOMWasmSource }): Promise<unknown>;
  engineAbiIdentity(): string;
  normalizeMedia(source: string, ...budget: BudgetArguments): string;
  normalizeSelector(source: string, ...budget: BudgetArguments): string;
  normalizeSupports(source: string, ...budget: BudgetArguments): string;
  parseContainerPreludeJson(source: string, ...budget: BudgetArguments): string;
  parseCounterStyleDescriptorValue(
    name: string,
    value: string,
    ...budget: BudgetArguments
  ): string | undefined;
  parseCounterStyleDescriptorsJson(source: string, ...budget: BudgetArguments): string;
  parseCounterStyleNameJson(source: string, ...budget: BudgetArguments): string | undefined;
  parseRecoveredRuleTreeJson(source: string, ...budget: BudgetArguments): string;
  parseRecoveredSingleRuleTreeJson(source: string, ...budget: BudgetArguments): string;
  parseStylesheetTreeJson(
    source: string,
    errorRecovery: boolean,
    ...budget: BudgetArguments
  ): string;
  parseScopePreludeJson(source: string, ...budget: BudgetArguments): string;
  scanTopLevelRulesJson(source: string, ...budget: BudgetArguments): string;
  serializeFontFamilyValue(value: string): string;
  serializeIdentifierValue(value: string): string;
  WasmDeclarationState: new (
    context: "style" | "font-face" | "function",
    ...budget: BudgetArguments
  ) => GeneratedDeclarationState;
}

export type SheetOMWasmSource = URL | Response | ArrayBuffer | WebAssembly.Module;

export class SheetOMWasmBindingError extends Error {
  readonly code:
    | "SHEETOM_WASM_INITIALIZATION_FAILED"
    | "SHEETOM_WASM_SOURCE_INVALID"
    | "SHEETOM_WASM_TRAP";

  constructor(
    code: SheetOMWasmBindingError["code"],
    message: string,
    options?: ErrorOptions,
  ) {
    super(message, options);
    this.name = "SheetOMWasmBindingError";
    this.code = code;
  }
}

export async function initializeWasmEngineBinding(
  source: SheetOMWasmSource,
  instanceToken: string,
  onTrap: (error: SheetOMWasmBindingError) => void,
): Promise<EngineBinding> {
  assertWasmSource(source);
  const glueUrl = new URL("./sheetom_wasm.js", import.meta.url);
  glueUrl.searchParams.set("sheetom-instance", instanceToken);

  let glue: WasmGlueModule;
  try {
    glue = await import(
      /* @vite-ignore */
      /* webpackIgnore: true */
      glueUrl.href
    ) as WasmGlueModule;
    await glue.default({ module_or_path: source });
  } catch (cause) {
    throw new SheetOMWasmBindingError(
      "SHEETOM_WASM_INITIALIZATION_FAILED",
      "SheetOM could not initialize the WebAssembly engine.",
      { cause },
    );
  }

  let poisoned: SheetOMWasmBindingError | undefined;
  const guard = <Result>(operation: () => Result): Result => {
    if (poisoned) throw poisoned;
    try {
      return operation();
    } catch (cause) {
      if (!(cause instanceof WebAssembly.RuntimeError)) throw cause;
      poisoned = new SheetOMWasmBindingError(
        "SHEETOM_WASM_TRAP",
        "SheetOM stopped a trapped WebAssembly engine instance.",
        { cause },
      );
      onTrap(poisoned);
      throw poisoned;
    }
  };

  const finalizer = new FinalizationRegistry<GeneratedDeclarationState>(state => {
    try {
      state.free();
    } catch {
      // Finalizers are best-effort cleanup and never alter observable state.
    }
  });

  const binding: EngineBinding = {
    engineAbiIdentity: () => guard(() => glue.engineAbiIdentity()),
    normalizeMedia: (sourceValue, ...budget) => guard(
      () => glue.normalizeMedia(sourceValue, ...budget),
    ),
    normalizeSelector: (sourceValue, ...budget) => guard(
      () => glue.normalizeSelector(sourceValue, ...budget),
    ),
    normalizeSupports: (sourceValue, ...budget) => guard(
      () => glue.normalizeSupports(sourceValue, ...budget),
    ),
    parseContainerPreludeJson: (sourceValue, ...budget) => guard(
      () => glue.parseContainerPreludeJson(sourceValue, ...budget),
    ),
    parseCounterStyleDescriptorValue: (name, value, ...budget) => guard(
      () => glue.parseCounterStyleDescriptorValue(name, value, ...budget) ?? null,
    ),
    parseCounterStyleDescriptorsJson: (sourceValue, ...budget) => guard(
      () => glue.parseCounterStyleDescriptorsJson(sourceValue, ...budget),
    ),
    parseCounterStyleNameJson: (sourceValue, ...budget) => guard(
      () => glue.parseCounterStyleNameJson(sourceValue, ...budget) ?? null,
    ),
    serializeIdentifierValue: value => guard(() => glue.serializeIdentifierValue(value)),
    serializeFontFamilyValue: value => guard(() => glue.serializeFontFamilyValue(value)),
    createDeclarationState: (context = "style", ...budget) => {
      const state = guard(() => new glue.WasmDeclarationState(context, ...budget));
      const handle = declarationStateHandle(state, guard);
      finalizer.register(handle, state);
      return handle;
    },
    parseRecoveredRuleTreeJson: (sourceValue, ...budget) => guard(
      () => glue.parseRecoveredRuleTreeJson(sourceValue, ...budget),
    ),
    parseRecoveredSingleRuleTreeJson: (sourceValue, ...budget) => guard(
      () => glue.parseRecoveredSingleRuleTreeJson(sourceValue, ...budget),
    ),
    parseStylesheetTreeJson: (sourceValue, errorRecovery, ...budget) => guard(
      () => glue.parseStylesheetTreeJson(sourceValue, errorRecovery, ...budget),
    ),
    parseScopePreludeJson: (sourceValue, ...budget) => guard(
      () => glue.parseScopePreludeJson(sourceValue, ...budget),
    ),
    scanTopLevelRulesJson: (sourceValue, ...budget) => guard(
      () => glue.scanTopLevelRulesJson(sourceValue, ...budget),
    ),
  };
  const identity = binding.engineAbiIdentity();
  validateEngineBindingIdentity({ engineAbiIdentity: () => identity });
  return binding;
}

function declarationStateHandle(
  state: GeneratedDeclarationState,
  guard: <Result>(operation: () => Result) => Result,
): EngineDeclarationStateHandle {
  return {
    get length() {
      return guard(() => state.length);
    },
    get cssText() {
      return guard(() => state.cssText);
    },
    item: index => guard(() => state.item(index)),
    getPropertyValue: name => guard(() => state.getPropertyValue(name)),
    getPropertyPriority: name => guard(() => state.getPropertyPriority(name)),
    setProperty: (name, value, priority, reservedNestingDepth) => guard(
      () => state.setProperty(
        name,
        value,
        priority,
        reservedNestingDepth,
      ) as EngineMutationOutcome,
    ),
    removeProperty: name => guard(() => state.removeProperty(name)),
    replaceCssText: (source, reservedNestingDepth) => guard(
      () => state.replaceCssText(source, reservedNestingDepth),
    ),
    serializeFormatted: (safe, indent, separator) => guard(
      () => state.serializeFormatted(safe, indent, separator),
    ),
  };
}

function assertWasmSource(source: unknown): asserts source is SheetOMWasmSource {
  if (
    (typeof URL === "function" && source instanceof URL)
    || (typeof Response === "function" && source instanceof Response)
    || source instanceof ArrayBuffer
    || source instanceof WebAssembly.Module
  ) return;
  throw new SheetOMWasmBindingError(
    "SHEETOM_WASM_SOURCE_INVALID",
    "WebAssembly source must be a URL, Response, ArrayBuffer, or WebAssembly.Module.",
  );
}
