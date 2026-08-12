import { createRequire } from "node:module";

const require = createRequire(import.meta.url);
const { NATIVE_TARGETS } = require("../native/resolve-target.cjs") as {
  NATIVE_TARGETS: Array<{ artifact: string; packageName: string }>;
};

export const expectedNativeArtifacts = Object.freeze(
  NATIVE_TARGETS.map(target => target.artifact),
);
export const expectedNativePackages = Object.freeze(
  NATIVE_TARGETS.map(target => target.packageName),
);

export function assertCompleteNativeArtifactNames(names: Iterable<string>): void {
  const actual = [...names].sort();
  const expected = [...expectedNativeArtifacts].sort();
  if (JSON.stringify(actual) === JSON.stringify(expected)) return;
  throw new Error(
    `Native artifact set is incomplete: expected ${expected.join(", ")}; ` +
      `received ${actual.join(", ")}`,
  );
}

export function assertRootTarballHasNoNativeAddon(entries: string[]): void {
  const addons = entries.filter(entry => entry.endsWith(".node"));
  if (addons.length === 0) return;
  throw new Error(`Root SheetOM tarball contains native addons: ${addons.join(", ")}`);
}

export function assertPlatformTarballEntries(
  entries: string[],
  artifact: string,
): void {
  const addons = entries.filter(entry => entry.endsWith(".node"));
  const expected = [`package/${artifact}`];
  if (JSON.stringify(addons) === JSON.stringify(expected)) return;
  throw new Error(
    `Native platform tarball must contain only ${expected[0]}; received ${addons.join(", ")}`,
  );
}
