export function npmTagForVersion(version: string): "next" | "latest";

export function extractReleaseNotes(changelog: string, version: string): string;

export function parsePackResult(output: string): {
  filename: string;
  integrity?: string;
  [key: string]: unknown;
};

export function packMetadataForTarball(tarball: string): Promise<{
  filename: string;
  integrity: string;
  size: number;
}>;

export function resolveSingleTarball(input: string): Promise<string>;

export function assessReleaseChannels(
  packageMetadata: {
    "dist-tags"?: Record<string, string>;
    versions?: Record<string, { deprecated?: string }>;
  },
  version: string,
): { ready: boolean; reasons: string[] };

export function assessImplementationPackageChannels(
  packageMetadata: {
    "dist-tags"?: Record<string, string>;
    versions?: Record<string, { deprecated?: string }>;
  },
  version: string,
): { ready: boolean; reasons: string[] };

export function assessNpmPublication(
  name: string,
  published: { name?: string; dist?: { integrity?: string } } | null,
  distTags: Record<string, string>,
  tag: string,
  version: string,
  integrity: string,
): "published" | "scanning" | "unpublished";

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

export function waitForPublishedVersion(
  name: string,
  version: string,
  integrity: string,
  options?: {
    attempts?: number;
    intervalMs?: number;
    readVersion?: (
      name: string,
      version: string,
    ) => Promise<{ dist?: { integrity?: string } } | null>;
    wait?: (milliseconds: number) => Promise<void>;
  },
): Promise<{ dist?: { integrity?: string } }>;
