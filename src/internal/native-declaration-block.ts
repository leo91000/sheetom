import type {
  SheetOMDiagnostic,
  SheetOMMutationDiagnosticCode,
} from "../diagnostics.js";
import {
  type EngineDeclarationMutation,
  type EngineDeclarationStateHandle,
  type EngineMutationOutcome,
} from "./engine-binding.js";
import { engineBinding } from "./default-engine-binding.js";
import {
  defaultResourceBudget,
  nativeBudgetArguments,
  rethrowResourceBudgetError,
  type NativeResourceBudget,
} from "./resource-budget.js";
import { rethrowSerializationError } from "../serialization-error.js";

type ReportDeclarationDiagnostic = (
  code: SheetOMMutationDiagnosticCode,
  property: string,
  input: string,
) => SheetOMDiagnostic;

type ReportSerializationDiagnostic = (
  shorthand: string,
  conflictingLonghands: readonly string[],
) => void;

interface RecoveredSerialization {
  serialized: string;
  issues: readonly {
    shorthand: string;
    conflictingLonghands: readonly string[];
  }[];
}

type CachedSerialization = string | RecoveredSerialization;

export type NativeDeclarationMutationResult =
  | {
      kind: "set";
      accepted: boolean;
      diagnostic: SheetOMDiagnostic | null;
    }
  | { kind: "remove"; value: string };

/** Thin JS ownership boundary around the Rust declaration state machine. */
export class NativeDeclarationBlock {
  readonly #state: EngineDeclarationStateHandle;
  readonly #reportDiagnostic: ReportDeclarationDiagnostic;
  readonly #reportSerializationDiagnostic: ReportSerializationDiagnostic;
  readonly #reservedNestingDepth: () => number;
  #observableCache: string | undefined;
  readonly #serializationCache = new Map<string, CachedSerialization>();

  constructor(
    reportDiagnostic: ReportDeclarationDiagnostic,
    reportSerializationDiagnostic: ReportSerializationDiagnostic,
    context: "style" | "font-face" | "function" = "style",
    resourceBudget: NativeResourceBudget = defaultResourceBudget,
    reservedNestingDepth: () => number = () => 0,
    initialCssText?: string,
  ) {
    this.#reportDiagnostic = reportDiagnostic;
    this.#reportSerializationDiagnostic = reportSerializationDiagnostic;
    this.#reservedNestingDepth = reservedNestingDepth;
    try {
      this.#state = engineBinding.createDeclarationState(
        context,
        ...nativeBudgetArguments(resourceBudget),
        initialCssText,
      );
    } catch (error) {
      rethrowResourceBudgetError(error);
      throw error;
    }
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
    let outcome: EngineMutationOutcome;
    try {
      outcome = this.#state.setProperty(name, value, priority, this.#reservedNestingDepth());
    } catch (error) {
      rethrowResourceBudgetError(error);
      throw error;
    }
    if (outcome === "applied") {
      this.#invalidateSerialization();
      return;
    }
    this.#diagnosticForOutcome(outcome, name, value, priority);
  }

  applyMutations(
    mutations: readonly EngineDeclarationMutation[],
  ): NativeDeclarationMutationResult[] {
    const kinds: number[] = [];
    const properties: string[] = [];
    const values: string[] = [];
    const priorities: string[] = [];
    for (const mutation of mutations) {
      kinds.push(mutation.kind === "set" ? 0 : 1);
      properties.push(mutation.property);
      values.push(mutation.kind === "set" ? mutation.value : "");
      priorities.push(mutation.kind === "set" ? mutation.priority : "");
    }
    let outcomes;
    try {
      outcomes = this.#state.applyMutations(
        kinds,
        properties,
        values,
        priorities,
        this.#reservedNestingDepth(),
      );
    } catch (error) {
      rethrowResourceBudgetError(error);
      throw error;
    }
    if (outcomes.length !== mutations.length) {
      throw new Error("SheetOM engine returned an incomplete declaration mutation result.");
    }
    const results: NativeDeclarationMutationResult[] = [];
    let mutated = false;
    for (let index = 0; index < outcomes.length; index += 1) {
      const mutation = mutations[index];
      const outcome = outcomes[index];
      if (!mutation || outcome === undefined) {
        throw new Error("SheetOM engine returned a mismatched declaration mutation result.");
      }
      if (mutation.kind === "remove") {
        mutated = true;
        results.push({ kind: "remove", value: outcome });
        continue;
      }
      if (outcome === "applied") {
        mutated = true;
        results.push({ kind: "set", accepted: true, diagnostic: null });
        continue;
      }
      if (!isEngineMutationOutcome(outcome)) {
        throw new Error("SheetOM engine returned an invalid declaration mutation outcome.");
      }
      results.push({
        kind: "set",
        accepted: false,
        diagnostic: this.#diagnosticForOutcome(
          outcome,
          mutation.property,
          mutation.value,
          mutation.priority,
        ),
      });
    }
    if (mutated) this.#invalidateSerialization();
    return results;
  }

  removeProperty(name: string): string {
    const removed = this.#state.removeProperty(name);
    this.#invalidateSerialization();
    return removed;
  }

  replaceCssText(source: string): void {
    try {
      this.#state.replaceCssText(source, this.#reservedNestingDepth());
    } catch (error) {
      rethrowResourceBudgetError(error);
      throw error;
    }
    this.#invalidateSerialization();
  }

  serialize(safe: boolean, indent: string, separator: string, strict = false): string {
    const cacheKey = `${safe ? "1" : "0"}${strict ? "1" : "0"}\0${indent}\0${separator}`;
    const cached = this.#serializationCache.get(cacheKey);
    if (cached !== undefined) {
      if (typeof cached === "string") return cached;
      this.#reportSerializationIssues(cached.issues);
      return cached.serialized;
    }
    let result: CachedSerialization;
    try {
      if (strict) {
        result = this.#state.serializeFormatted(safe, indent, separator);
      } else {
        result = parseResilientSerializationResult(
          this.#state.serializeFormattedResilient(safe, indent, separator),
        );
      }
    } catch (error) {
      rethrowResourceBudgetError(error);
      rethrowSerializationError(error);
      throw error;
    }
    this.#serializationCache.set(cacheKey, result);
    if (typeof result === "string") return result;
    this.#reportSerializationIssues(result.issues);
    return result.serialized;
  }

  #invalidateSerialization(): void {
    this.#observableCache = undefined;
    this.#serializationCache.clear();
  }

  #diagnosticForOutcome(
    outcome: Exclude<EngineMutationOutcome, "applied">,
    name: string,
    value: string,
    priority: string,
  ): SheetOMDiagnostic {
    const normalizedName = name.startsWith("--") ? name : name.toLowerCase();
    const [code, input]: [SheetOMMutationDiagnosticCode, string] = outcome === "invalid-priority"
      ? ["INVALID_PRIORITY", priority]
      : outcome === "unsupported-shorthand"
        ? ["UNSUPPORTED_SHORTHAND_VALUE", value]
        : ["INVALID_PROPERTY_VALUE", value];
    return this.#reportDiagnostic(code, normalizedName, input);
  }

  #reportSerializationIssues(issues: RecoveredSerialization["issues"]): void {
    for (const issue of issues) {
      this.#reportSerializationDiagnostic(issue.shorthand, issue.conflictingLonghands);
    }
  }
}

function parseResilientSerializationResult(output: readonly string[]): CachedSerialization {
  const serialized = output[0];
  if (serialized === undefined || output.length % 2 === 0) {
    throw new Error("SheetOM engine returned an invalid resilient serialization result.");
  }
  if (output.length === 1) return serialized;
  const issues: RecoveredSerialization["issues"][number][] = [];
  for (let index = 1; index < output.length; index += 2) {
    const shorthand = output[index];
    const conflicts = output[index + 1];
    if (!shorthand || !conflicts) {
      throw new Error("SheetOM engine returned an incomplete resilient serialization issue.");
    }
    issues.push({
      shorthand,
      conflictingLonghands: Object.freeze(conflicts.split(",")),
    });
  }
  return { serialized, issues };
}

function isEngineMutationOutcome(
  value: string,
): value is Exclude<EngineMutationOutcome, "applied"> {
  return value === "invalid-name"
    || value === "invalid-priority"
    || value === "invalid-value"
    || value === "unsupported-shorthand";
}

function toUnsignedLong(value: unknown): number {
  const numeric = Number(value);
  if (!Number.isFinite(numeric) || numeric === 0) return 0;
  const integer = Math.trunc(numeric);
  return ((integer % (2 ** 32)) + (2 ** 32)) % (2 ** 32);
}
