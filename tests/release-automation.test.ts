import { describe, expect, it } from "vitest";
import {
  assessImplementationPackageChannels,
  assessNpmPublication,
  assessReleaseChannels,
  extractReleaseNotes,
  npmTagForVersion,
  packMetadataForTarball,
  parsePackResult,
  resolveSingleTarball,
  waitForDistTag,
  waitForPublishedVersion,
} from "../scripts/publish-release.ts";
import { hasReleaseVersionChange } from "../scripts/detect-release-version-change.ts";
import {
  assertCompleteNativeArtifactNames,
  expectedNativeArtifacts,
  assertPlatformTarballEntries,
  assertRootTarballHasNoNativeAddon,
  assertRuntimeTarballHasNoCompatibilityEvidence,
} from "../scripts/native-artifact-contract.ts";
import {
  assertReleaseArtifactManifest,
  expectedReleasePackages,
} from "../scripts/release-artifact-set.ts";
import { mkdtemp, mkdir, rm, writeFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";

describe("release automation", () => {
  it("publishes only when the package version changed", () => {
    expect(hasReleaseVersionChange(
      { version: "0.1.0-rc.6" },
      { version: "0.1.0-rc.5" },
    )).toBe(true);
    expect(hasReleaseVersionChange(
      { version: "0.1.0-rc.5", description: "Updated README" },
      { version: "0.1.0-rc.5" },
    )).toBe(false);
    expect(hasReleaseVersionChange(
      { version: "0.1.0-rc.0" },
      null,
    )).toBe(true);
  });

  it("requires latest and next to identify the active prerelease before stable", () => {
    expect(assessReleaseChannels({
      "dist-tags": {
        latest: "0.1.0-rc.0",
        next: "0.1.0-rc.1",
      },
      versions: {
        "0.1.0-rc.0": {},
        "0.1.0-rc.1": {},
      },
    }, "0.1.0-rc.1")).toEqual({
      ready: false,
      reasons: [
        "latest must point to 0.1.0-rc.1 before the first stable release",
        "superseded prerelease 0.1.0-rc.0 must be deprecated",
      ],
    });
  });

  it("keeps latest stable while next identifies an active prerelease", () => {
    expect(assessReleaseChannels({
      "dist-tags": {
        latest: "0.1.0",
        next: "0.2.0-rc.1",
      },
      versions: {
        "0.1.0-rc.1": { deprecated: "Upgrade." },
        "0.1.0": {},
        "0.2.0-rc.1": {},
      },
    }, "0.2.0-rc.1")).toEqual({ ready: true, reasons: [] });
  });

  it("requires stable releases to own latest and clear next", () => {
    expect(assessReleaseChannels({
      "dist-tags": {
        latest: "0.1.0",
        next: "0.1.0-rc.1",
      },
      versions: {
        "0.1.0-rc.1": { deprecated: "Stable is available." },
        "0.1.0": {},
      },
    }, "0.1.0")).toEqual({
      ready: false,
      reasons: ["next must be removed when publishing a stable release"],
    });
  });

  it("keeps native prereleases on next without requiring a latest tag", () => {
    expect(assessImplementationPackageChannels({
      "dist-tags": { next: "0.1.0-rc.7" },
      versions: { "0.1.0-rc.7": {} },
    }, "0.1.0-rc.7")).toEqual({ ready: true, reasons: [] });

    expect(assessImplementationPackageChannels({
      "dist-tags": { next: "0.1.0-rc.6" },
      versions: {
        "0.1.0-rc.6": {},
        "0.1.0-rc.7": {},
      },
    }, "0.1.0-rc.7")).toEqual({
      ready: false,
      reasons: [
        "next must point to active prerelease 0.1.0-rc.7",
        "superseded prerelease 0.1.0-rc.6 must be deprecated",
      ],
    });
  });

  it("routes prereleases to next and stable releases to latest", () => {
    expect(npmTagForVersion("0.1.0-rc.1")).toBe("next");
    expect(npmTagForVersion("0.1.0")).toBe("latest");
  });

  it("does not republish a version whose dist-tag is visible during registry scanning", () => {
    expect(assessNpmPublication(
      "sheetom",
      null,
      { next: "0.1.0-rc.7" },
      "next",
      "0.1.0-rc.7",
      "sha512-expected",
    )).toBe("scanning");
    expect(assessNpmPublication(
      "sheetom",
      null,
      { next: "0.1.0-rc.6" },
      "next",
      "0.1.0-rc.7",
      "sha512-expected",
    )).toBe("unpublished");
    expect(assessNpmPublication(
      "sheetom",
      { name: "sheetom", dist: { integrity: "sha512-expected" } },
      { next: "0.1.0-rc.7" },
      "next",
      "0.1.0-rc.7",
      "sha512-expected",
    )).toBe("published");
    expect(() => assessNpmPublication(
      "sheetom",
      { name: "sheetom", dist: { integrity: "sha512-other" } },
      { next: "0.1.0-rc.7" },
      "next",
      "0.1.0-rc.7",
      "sha512-expected",
    )).toThrow("npm serves unexpected integrity for sheetom@0.1.0-rc.7");
  });

  it("extracts only the requested changelog section", () => {
    const changelog = [
      "# sheetom",
      "",
      "## 0.2.0",
      "",
      "New release.",
      "",
      "## 0.1.0",
      "",
      "Old release.",
    ].join("\n");
    expect(extractReleaseNotes(changelog, "0.2.0")).toBe("New release.");
    expect(extractReleaseNotes(changelog, "9.0.0")).toBe("Release 9.0.0.");
  });

  it("requires npm pack to return exactly one artifact", () => {
    expect(parsePackResult('[{"filename":"sheetom-0.1.0.tgz"}]')).toEqual({
      filename: "sheetom-0.1.0.tgz",
    });
    expect(() => parsePackResult("[]")).toThrow(/exactly one/);
  });

  it("requires every supported native binary before packaging", () => {
    expect(() => assertCompleteNativeArtifactNames(expectedNativeArtifacts)).not.toThrow();
    expect(() => assertCompleteNativeArtifactNames(expectedNativeArtifacts.slice(1)))
      .toThrow(/incomplete/);
  });

  it("keeps native addons out of the root package and exactly one per platform package", () => {
    expect(() => assertRootTarballHasNoNativeAddon([
      "package/dist/index.js",
      "package/native/index.cjs",
    ])).not.toThrow();
    expect(() => assertRootTarballHasNoNativeAddon([
      "package/native/sheetom-native.linux-x64-gnu.node",
    ])).toThrow(/contains native addons/);
    expect(() => assertPlatformTarballEntries([
      "package/index.cjs",
      "package/sheetom-native.linux-x64-gnu.node",
    ], "sheetom-native.linux-x64-gnu.node")).not.toThrow();
  });

  it("keeps immutable compatibility evidence out of runtime tarballs", () => {
    expect(() => assertRuntimeTarballHasNoCompatibilityEvidence([
      "package/dist/index.js",
      "package/docs/api.md",
    ])).not.toThrow();
    expect(() => assertRuntimeTarballHasNoCompatibilityEvidence([
      "package/compatibility/baselines/0.1.0-rc.8.json",
    ])).toThrow(/contains release compatibility evidence/);
  });

  it("requires a lockstep native, WebAssembly, and root release artifact set", () => {
    const rootManifest = { name: "sheetom", version: "0.1.0-rc.7" };
    const expected = expectedReleasePackages(rootManifest);
    expect(expected).toHaveLength(15);
    expect(expected.at(-2)).toMatchObject({ name: "@sheetom/wasm", role: "wasm" });
    expect(expected.at(-1)).toMatchObject({ name: "sheetom", role: "root" });

    const packages = expected.map((entry, index) => ({
      name: entry.name,
      version: entry.version,
      role: entry.role,
      ...(entry.target === undefined ? {} : { target: entry.target }),
      filename: `artifact-${index}.tgz`,
      integrity: "sha512-YQ==",
      sha256: "a".repeat(64),
      size: index + 1,
    }));
    const manifest = {
      schemaVersion: 1 as const,
      version: rootManifest.version,
      packages,
      totalSize: packages.reduce((sum, artifact) => sum + artifact.size, 0),
    };

    expect(() => assertReleaseArtifactManifest(manifest, rootManifest)).not.toThrow();
    expect(() => assertReleaseArtifactManifest({
      ...manifest,
      packages: packages.slice(1),
    }, rootManifest)).toThrow(/incomplete/u);
    expect(() => assertReleaseArtifactManifest({
      ...manifest,
      packages: [packages[1]!, packages[0]!, ...packages.slice(2)],
    }, rootManifest)).toThrow(/must be/u);
  });

  it("uses the exact prebuilt release tarball and computes npm integrity", async () => {
    const directory = await mkdtemp(path.join(os.tmpdir(), "sheetom-release-test-"));
    try {
      const artifactDirectory = path.join(directory, "artifact");
      await mkdir(artifactDirectory);
      const tarball = path.join(artifactDirectory, "sheetom-0.1.0-rc.6.tgz");
      await writeFile(tarball, "verified artifact");
      await expect(resolveSingleTarball(artifactDirectory)).resolves.toBe(tarball);
      await expect(packMetadataForTarball(tarball)).resolves.toMatchObject({
        filename: "sheetom-0.1.0-rc.6.tgz",
        integrity: expect.stringMatching(/^sha512-/u),
        size: 17,
      });
    } finally {
      await rm(directory, { recursive: true, force: true });
    }
  });

  it("waits for an npm dist-tag to propagate", async () => {
    let reads = 0;
    const distTags = await waitForDistTag("sheetom", "next", "0.1.0-rc.1", {
      attempts: 3,
      intervalMs: 0,
      readTags: async () => {
        reads += 1;
        return { next: reads === 1 ? "0.1.0-rc.0" : "0.1.0-rc.1" };
      },
      wait: async () => {},
    });

    expect(reads).toBe(2);
    expect(distTags.next).toBe("0.1.0-rc.1");
  });

  it("bounds npm dist-tag propagation retries", async () => {
    let waits = 0;
    const result = waitForDistTag("sheetom", "next", "0.1.0-rc.1", {
      attempts: 3,
      intervalMs: 0,
      readTags: async () => ({ next: "0.1.0-rc.0" }),
      wait: async () => {
        waits += 1;
      },
    });

    await expect(result).rejects.toThrow(
      "npm dist-tag next did not point to 0.1.0-rc.1 after 3 attempts; " +
        "last observed 0.1.0-rc.0",
    );
    expect(waits).toBe(2);
  });

  it("waits through npm publish-time scanning until the exact artifact is public", async () => {
    let reads = 0;
    let waits = 0;
    const published = await waitForPublishedVersion(
      "@sheetom/wasm",
      "0.1.0-rc.7",
      "sha512-expected",
      {
        attempts: 4,
        intervalMs: 0,
        readVersion: async () => {
          reads += 1;
          return reads < 3 ? null : { dist: { integrity: "sha512-expected" } };
        },
        wait: async () => {
          waits += 1;
        },
      },
    );

    expect(published.dist?.integrity).toBe("sha512-expected");
    expect(reads).toBe(3);
    expect(waits).toBe(2);
  });

  it("fails closed when npm exposes a different artifact integrity", async () => {
    const result = waitForPublishedVersion(
      "sheetom",
      "0.1.0-rc.7",
      "sha512-expected",
      {
        attempts: 3,
        intervalMs: 0,
        readVersion: async () => ({ dist: { integrity: "sha512-other" } }),
        wait: async () => {},
      },
    );

    await expect(result).rejects.toThrow(
      "npm serves unexpected integrity for sheetom@0.1.0-rc.7",
    );
  });
});
