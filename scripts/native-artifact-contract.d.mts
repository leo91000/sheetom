export const expectedNativeArtifacts: readonly string[];
export const expectedNativePackages: readonly string[];

export function assertCompleteNativeArtifactNames(names: Iterable<string>): void;

export function assertRootTarballHasNoNativeAddon(entries: string[]): void;

export function assertPlatformTarballEntries(entries: string[], artifact: string): void;
