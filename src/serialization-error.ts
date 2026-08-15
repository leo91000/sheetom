export type SheetOMSerializationErrorCode =
  | "UNREPRESENTABLE_PENDING_SHORTHAND";

/** A reparsable stylesheet cannot faithfully represent the current CSSOM state. */
export class SheetOMSerializationError extends Error {
  readonly code: SheetOMSerializationErrorCode;
  readonly shorthand: string;
  readonly conflictingLonghands: readonly string[];

  constructor(
    shorthand: string,
    conflictingLonghands: readonly string[],
    options?: ErrorOptions,
  ) {
    super(
      `SheetOM cannot serialize the pending ${shorthand} shorthand without changing its semantics.`,
      options,
    );
    this.name = "SheetOMSerializationError";
    this.code = "UNREPRESENTABLE_PENDING_SHORTHAND";
    this.shorthand = shorthand;
    this.conflictingLonghands = Object.freeze([...conflictingLonghands]);
  }
}

const unrepresentablePendingShorthandPattern =
  /^SHEETOM_UNREPRESENTABLE_PENDING_SHORTHAND: shorthand=([^;]+); conflicting=([a-z0-9,-]+)$/u;

export function serializationErrorFrom(error: unknown): SheetOMSerializationError | null {
  if (!(error instanceof Error)) return null;
  const match = unrepresentablePendingShorthandPattern.exec(error.message);
  if (!match) return null;
  const shorthand = match[1];
  const conflicts = match[2];
  if (!shorthand || !conflicts) return null;
  return new SheetOMSerializationError(shorthand, conflicts.split(","), { cause: error });
}

export function rethrowSerializationError(error: unknown): void {
  const serializationError = serializationErrorFrom(error);
  if (serializationError) throw serializationError;
}
