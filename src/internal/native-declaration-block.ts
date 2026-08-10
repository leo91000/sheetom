import { createRequire } from "node:module";
import path from "node:path";
import { fileURLToPath } from "node:url";

import type { SheetOMDiagnosticCode } from "../diagnostics.js";
import type { ReportDeclarationDiagnostic } from "./declaration-block.js";

type MutationOutcome =
  | "applied"
  | "invalid-name"
  | "invalid-priority"
  | "invalid-value"
  | "unsupported-shorthand";

interface NativeDeclarationState {
  readonly length: number;
  readonly cssText: string;
  item(index: number): string;
  getPropertyValue(name: string): string;
  getPropertyPriority(name: string): string;
  setProperty(name: string, value: string, priority: string): MutationOutcome;
  removeProperty(name: string): string;
  replaceCssText(source: string): void;
  serializeFormatted(safe: boolean, indent: string, separator: string): string;
}

interface NativeBinding {
  NativeDeclarationState: new () => NativeDeclarationState;
}

function loadBinding(): NativeBinding {
  const moduleDirectory = path.dirname(fileURLToPath(import.meta.url));
  const packageRoot = path.basename(moduleDirectory) === "dist"
    ? path.dirname(moduleDirectory)
    : path.resolve(moduleDirectory, "../..");
  const require = createRequire(import.meta.url);
  return require(path.join(packageRoot, "native", "index.cjs")) as NativeBinding;
}

const binding = loadBinding();

/** Thin JS ownership boundary around the Rust declaration state machine. */
export class NativeDeclarationBlock {
  readonly #state = new binding.NativeDeclarationState();
  readonly #reportDiagnostic: ReportDeclarationDiagnostic;
  #observableCache: string | undefined;
  readonly #serializationCache = new Map<string, string>();

  constructor(reportDiagnostic: ReportDeclarationDiagnostic) {
    this.#reportDiagnostic = reportDiagnostic;
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
    const outcome = this.#state.setProperty(name, value, priority);
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
    this.#state.replaceCssText(source);
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
