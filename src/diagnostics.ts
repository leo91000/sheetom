/** Stable machine-readable codes emitted by opt-in mutation diagnostics. */
export type SheetOMMutationDiagnosticCode =
  | "INVALID_PRIORITY"
  | "INVALID_PROPERTY_VALUE"
  | "UNSUPPORTED_SHORTHAND_VALUE";

export type SheetOMSerializationDiagnosticCode =
  | "UNREPRESENTABLE_PENDING_SHORTHAND";

export type SheetOMDiagnosticCode =
  | SheetOMMutationDiagnosticCode
  | SheetOMSerializationDiagnosticCode;

/** A structured explanation for an ignored mutation. */
export interface SheetOMMutationDiagnostic {
  code: SheetOMMutationDiagnosticCode;
  severity: "warning";
  operation: "setProperty";
  message: string;
  property: string;
  input: string;
  location: null;
}

/** A structured explanation for a best-effort serialization recovery. */
export interface SheetOMSerializationDiagnostic {
  code: SheetOMSerializationDiagnosticCode;
  severity: "warning";
  operation: "serialize";
  message: string;
  property: string;
  input: "";
  location: null;
  conflictingLonghands: readonly string[];
}

export type SheetOMDiagnostic =
  | SheetOMMutationDiagnostic
  | SheetOMSerializationDiagnostic;
