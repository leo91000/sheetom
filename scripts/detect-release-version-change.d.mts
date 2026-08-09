interface PackageManifestLike {
  version?: unknown;
  [key: string]: unknown;
}

export function hasReleaseVersionChange(
  currentManifest: PackageManifestLike,
  previousManifest: PackageManifestLike,
): boolean;
