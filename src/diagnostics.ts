/** Stable machine-readable codes emitted by opt-in mutation diagnostics. */
export type SheetOMDiagnosticCode =
  | "INVALID_PRIORITY"
  | "INVALID_PROPERTY_VALUE"
  | "UNSUPPORTED_SHORTHAND_VALUE";

/** A structured explanation for an ignored or recovered mutation. */
export interface SheetOMDiagnostic {
  code: SheetOMDiagnosticCode;
  severity: "warning";
  operation: "setProperty";
  message: string;
  property: string;
  input: string;
  location: null;
}
