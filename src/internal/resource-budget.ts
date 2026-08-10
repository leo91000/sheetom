export interface SheetOMResourceBudget {
  maxStylesheetBytes?: number;
  maxDeclarationValueBytes?: number;
  maxSyntaxDepth?: number;
  maxRuleCount?: number;
  maxDeclarationsPerBlock?: number;
}

export interface NativeResourceBudget {
  readonly maxStylesheetBytes: number;
  readonly maxDeclarationValueBytes: number;
  readonly maxSyntaxDepth: number;
  readonly maxRuleCount: number;
  readonly maxDeclarationsPerBlock: number;
}

export const defaultResourceBudget: NativeResourceBudget = Object.freeze({
  maxStylesheetBytes: 64 * 1024 * 1024,
  maxDeclarationValueBytes: 1024 * 1024,
  maxSyntaxDepth: 4096,
  maxRuleCount: 1_000_000,
  maxDeclarationsPerBlock: 100_000,
});

const maximumUnsignedLong = (2 ** 32) - 1;
const maximumConfigurableSyntaxDepth = 16_384;

export function normalizeResourceBudget(
  value: SheetOMResourceBudget | null | undefined,
): NativeResourceBudget {
  if (
    value !== undefined
    && value !== null
    && (typeof value !== "object" || Array.isArray(value))
  ) {
    throw new TypeError("resourceBudget must be an object");
  }
  const input = value ?? {};
  return Object.freeze({
    maxStylesheetBytes: limitValue(
      input.maxStylesheetBytes,
      "maxStylesheetBytes",
      defaultResourceBudget.maxStylesheetBytes,
    ),
    maxDeclarationValueBytes: limitValue(
      input.maxDeclarationValueBytes,
      "maxDeclarationValueBytes",
      defaultResourceBudget.maxDeclarationValueBytes,
    ),
    maxSyntaxDepth: limitValue(
      input.maxSyntaxDepth,
      "maxSyntaxDepth",
      defaultResourceBudget.maxSyntaxDepth,
      maximumConfigurableSyntaxDepth,
    ),
    maxRuleCount: limitValue(
      input.maxRuleCount,
      "maxRuleCount",
      defaultResourceBudget.maxRuleCount,
    ),
    maxDeclarationsPerBlock: limitValue(
      input.maxDeclarationsPerBlock,
      "maxDeclarationsPerBlock",
      defaultResourceBudget.maxDeclarationsPerBlock,
    ),
  });
}

export function nativeBudgetArguments(
  budget: NativeResourceBudget,
): [number, number, number, number, number] {
  return [
    budget.maxStylesheetBytes,
    budget.maxDeclarationValueBytes,
    budget.maxSyntaxDepth,
    budget.maxRuleCount,
    budget.maxDeclarationsPerBlock,
  ];
}

export function rethrowResourceBudgetError(error: unknown): void {
  if (!(error instanceof Error)) return;
  if (!/^SHEETOM_(?:INPUT|DECLARATION|RULE|NESTING)_LIMIT:/u.test(error.message)) return;
  throw new RangeError(error.message);
}

function limitValue(
  value: unknown,
  name: string,
  fallback: number,
  maximum = maximumUnsignedLong,
): number {
  if (value === undefined) return fallback;
  if (
    typeof value === "number"
    && Number.isSafeInteger(value)
    && value >= 0
    && value <= maximum
  ) return value;
  throw new RangeError(`${name} must be an integer between 0 and ${maximum}`);
}
