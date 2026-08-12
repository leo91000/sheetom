interface PackageManifestLike {
  version?: unknown;
  [key: string]: unknown;
}

export function hasReleaseVersionChange(
  currentManifest: PackageManifestLike,
  previousManifest: PackageManifestLike | null,
): boolean;

export function latestVersionTag(revision: string): string | null;
