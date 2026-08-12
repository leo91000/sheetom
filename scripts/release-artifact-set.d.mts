export type ReleasePackageRole = "native" | "wasm" | "root";

export type ExpectedReleasePackage = {
  name: string;
  version: string;
  role: ReleasePackageRole;
  target?: string;
  nativeArtifact?: string;
};

export type ReleaseArtifact = {
  name: string;
  version: string;
  role: ReleasePackageRole;
  target?: string;
  filename: string;
  integrity: string;
  sha256: string;
  size: number;
};

export type ReleaseArtifactManifest = {
  schemaVersion: 1;
  version: string;
  packages: ReleaseArtifact[];
  totalSize: number;
};

export const RELEASE_MANIFEST_FILENAME: "sheetom-release-manifest.json";

export function expectedReleasePackages(rootManifest: {
  name: string;
  version: string;
}): ExpectedReleasePackage[];

export function assertReleaseArtifactManifest(
  manifest: ReleaseArtifactManifest,
  rootManifest: { name: string; version: string },
): void;

export function createReleaseArtifactManifest(
  directory: string,
  rootManifest: { name: string; version: string },
): Promise<ReleaseArtifactManifest>;

export function writeReleaseArtifactManifest(
  directory: string,
  rootManifest: { name: string; version: string },
): Promise<ReleaseArtifactManifest>;

export function verifyReleaseArtifactSet(
  directory: string,
  rootManifest: { name: string; version: string },
): Promise<ReleaseArtifactManifest>;
