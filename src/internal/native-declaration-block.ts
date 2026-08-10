import type { SheetOMDiagnosticCode } from "../diagnostics.js";
import {
  nativeBinding,
  type NativeDeclarationStateHandle,
  type NativeMutationOutcome,
} from "./native-binding.js";
import {
  defaultResourceBudget,
  nativeBudgetArguments,
  rethrowResourceBudgetError,
  type NativeResourceBudget,
} from "./resource-budget.js";

type ReportDeclarationDiagnostic = (
  code: SheetOMDiagnosticCode,
  property: string,
  input: string,
) => void;

/** Thin JS ownership boundary around the Rust declaration state machine. */
export class NativeDeclarationBlock {
  readonly #state: NativeDeclarationStateHandle;
  readonly #reportDiagnostic: ReportDeclarationDiagnostic;
  #observableCache: string | undefined;
  readonly #serializationCache = new Map<string, string>();

  constructor(
    reportDiagnostic: ReportDeclarationDiagnostic,
    context: "style" | "font-face" | "function" = "style",
    resourceBudget: NativeResourceBudget = defaultResourceBudget,
  ) {
    this.#reportDiagnostic = reportDiagnostic;
    this.#state = new nativeBinding.NativeDeclarationState(
      context,
      ...nativeBudgetArguments(resourceBudget),
    );
  }

  get cssText(): string {
    this.#observableCache ??= this.#state.cssText;
    return this.#observableCache;
  }

  get length(): number {
    return this.#state.length;
  }

  item(index: unknown): string {
    return this.#state.item(toUnsignedLong(index));
  }

  getPropertyValue(name: string): string {
    return this.#state.getPropertyValue(name);
  }

  getPropertyPriority(name: string): string {
    return this.#state.getPropertyPriority(name);
  }

  setProperty(name: string, value: string, priority: string): void {
    let outcome: NativeMutationOutcome;
    try {
      outcome = this.#state.setProperty(name, value, priority);
    } catch (error) {
      rethrowResourceBudgetError(error);
      throw error;
    }
    if (outcome === "applied") {
      this.#invalidateSerialization();
      return;
    }

    const normalizedName = name.startsWith("--") ? name : name.toLowerCase();
    const [code, input]: [SheetOMDiagnosticCode, string] = outcome === "invalid-priority"
      ? ["INVALID_PRIORITY", priority]
      : outcome === "unsupported-shorthand"
        ? ["UNSUPPORTED_SHORTHAND_VALUE", value]
        : ["INVALID_PROPERTY_VALUE", value];
    this.#reportDiagnostic(code, normalizedName, input);
  }

  removeProperty(name: string): string {
    const removed = this.#state.removeProperty(name);
    this.#invalidateSerialization();
    return removed;
  }

  replaceCssText(source: string): void {
    try {
      this.#state.replaceCssText(source);
    } catch (error) {
      rethrowResourceBudgetError(error);
      throw error;
    }
    this.#invalidateSerialization();
  }

  serialize(safe: boolean, indent: string, separator: string): string {
    const cacheKey = `${safe ? "1" : "0"}\0${indent}\0${separator}`;
    const cached = this.#serializationCache.get(cacheKey);
    if (cached !== undefined) return cached;
    const serialized = this.#state.serializeFormatted(safe, indent, separator);
    this.#serializationCache.set(cacheKey, serialized);
    return serialized;
  }

  #invalidateSerialization(): void {
    this.#observableCache = undefined;
    this.#serializationCache.clear();
  }
}

function toUnsignedLong(value: unknown): number {
  const numeric = Number(value);
  if (!Number.isFinite(numeric) || numeric === 0) return 0;
  const integer = Math.trunc(numeric);
  return ((integer % (2 ** 32)) + (2 ** 32)) % (2 ** 32);
}
