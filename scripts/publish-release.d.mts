export function npmTagForVersion(version: string): "next" | "latest";

export function extractReleaseNotes(changelog: string, version: string): string;

export function parsePackResult(output: string): {
  filename: string;
  integrity?: string;
  [key: string]: unknown;
};

export function assessReleaseChannels(
  packageMetadata: {
    "dist-tags"?: Record<string, string>;
    versions?: Record<string, { deprecated?: string }>;
  },
  version: string,
): { ready: boolean; reasons: string[] };

export function waitForDistTag(
  name: string,
  tag: string,
  version: string,
  options?: {
    attempts?: number;
    intervalMs?: number;
    readTags?: (name: string) => Promise<Record<string, string>>;
    wait?: (milliseconds: number) => Promise<void>;
  },
): Promise<Record<string, string>>;
