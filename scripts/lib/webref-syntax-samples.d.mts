export interface WebrefSyntaxSample {
  value: string;
  branch: string;
}

export interface WebrefSyntaxIssue {
  kind: string;
  path: string;
  [detail: string]: unknown;
}

export interface WebrefDefinitions {
  properties: Record<string, { syntax: string; [field: string]: unknown }>;
  types: Record<string, { syntax: string; [field: string]: unknown }>;
  functions: Record<string, { syntax: string; [field: string]: unknown }>;
}

export interface WebrefSyntaxSamplerOptions {
  definitions: WebrefDefinitions;
  property: string;
  syntax: string;
  fallbackValue?: (property: string) => string | null | undefined;
  terminalValues?: Record<string, string[]>;
  maximumDepth?: number;
  maximumSamplesPerNode?: number;
}

export function generateWebrefSyntaxSamples(
  options: WebrefSyntaxSamplerOptions,
): { samples: WebrefSyntaxSample[]; issues: WebrefSyntaxIssue[] };
