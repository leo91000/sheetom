export function npmTagForVersion(version: string): "next" | "latest";

export function extractReleaseNotes(changelog: string, version: string): string;

export function parsePackResult(output: string): {
  filename: string;
  integrity?: string;
  [key: string]: unknown;
};
