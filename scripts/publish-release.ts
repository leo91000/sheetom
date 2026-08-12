import {
  execFileSync,
  spawnSync,
  type ExecFileSyncOptions,
  type ExecFileSyncOptionsWithStringEncoding,
} from "node:child_process";
import { createHash } from "node:crypto";
import { appendFile, mkdtemp, mkdir, readFile, readdir, rm, stat } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

import { assertRootTarballHasNoNativeAddon } from "./native-artifact-contract.ts";
import {
  RELEASE_MANIFEST_FILENAME,
  verifyReleaseArtifactSet,
  type ReleaseArtifact,
} from "./release-artifact-set.ts";

const registryOrigin = "https://registry.npmjs.org";

interface PackageManifest {
  name: string;
  version: string;
  dependencies?: Record<string, unknown>;
}

interface RegistryVersion {
  name?: string;
  deprecated?: string;
  dist?: { integrity?: string };
}

interface PackageMetadata {
  "dist-tags"?: Record<string, string>;
  versions?: Record<string, RegistryVersion>;
}

interface PackResult {
  filename: string;
  integrity?: string;
  size?: number;
  [key: string]: unknown;
}

interface PackedArtifact {
  name: string;
  version: string;
  role: "native" | "wasm" | "root";
  filename: string;
  integrity: string;
  size: number;
  tarball: string;
}

interface ReleaseArtifactInput {
  legacy: boolean;
  packages: PackedArtifact[];
  assets: string[];
}

interface GitHubRelease {
  isDraft: boolean;
  isPrerelease: boolean;
  tagName: string;
  targetCommitish: string;
  url: string;
  assets: Array<{ name: string }>;
}

interface WaitForDistTagOptions {
  attempts?: number;
  intervalMs?: number;
  readTags?: (name: string) => Promise<Record<string, string>>;
  wait?: (milliseconds: number) => Promise<void>;
}

interface WaitForPublishedVersionOptions {
  attempts?: number;
  intervalMs?: number;
  readVersion?: (
    name: string,
    version: string,
  ) => Promise<RegistryVersion | null>;
  wait?: (milliseconds: number) => Promise<void>;
}

export function npmTagForVersion(version: string): "next" | "latest" {
  return version.includes("-") ? "next" : "latest";
}

export function extractReleaseNotes(changelog: string, version: string): string {
  const heading = `## ${version}`;
  const start = changelog.indexOf(heading);
  if (start === -1) return `Release ${version}.`;

  const bodyStart = start + heading.length;
  const nextHeading = changelog.indexOf("\n## ", bodyStart);
  const end = nextHeading === -1 ? changelog.length : nextHeading;
  const body = changelog.slice(bodyStart, end).trim();
  return body === "" ? `Release ${version}.` : body;
}

export function parsePackResult(output: string): PackResult {
  const result = JSON.parse(output);
  if (!Array.isArray(result) || result.length !== 1 || !result[0]?.filename) {
    throw new Error("npm pack did not produce exactly one package artifact");
  }
  return result[0];
}

export async function packMetadataForTarball(tarball: string): Promise<{
  filename: string;
  integrity: string;
  size: number;
}> {
  const bytes = await readFile(tarball);
  return {
    filename: path.basename(tarball),
    integrity: `sha512-${createHash("sha512").update(bytes).digest("base64")}`,
    size: bytes.length,
  };
}

export async function resolveSingleTarball(input: string): Promise<string> {
  const resolved = path.resolve(input);
  if ((await stat(resolved)).isFile()) return resolved;
  const entries = (await readdir(resolved)).filter(entry => entry.endsWith(".tgz"));
  if (entries.length !== 1) {
    throw new Error(`Expected one release tarball in ${resolved}, found ${entries.length}`);
  }
  const [entry] = entries;
  if (!entry) throw new Error(`No release tarball exists in ${resolved}`);
  return path.join(resolved, entry);
}

export function assessReleaseChannels(
  packageMetadata: PackageMetadata,
  version: string,
): { ready: boolean; reasons: string[] } {
  const distTags = packageMetadata["dist-tags"] ?? {};
  const versions = packageMetadata.versions ?? {};
  const prerelease = version.includes("-");
  const stableVersions = Object.keys(versions).filter(candidate => !candidate.includes("-"));
  const reasons: string[] = [];

  if (prerelease) {
    if (distTags.next !== version) {
      reasons.push(`next must point to active prerelease ${version}`);
    }
    if (stableVersions.length === 0) {
      if (distTags.latest !== version) {
        reasons.push(`latest must point to ${version} before the first stable release`);
      }
    } else {
      const latest = distTags.latest;
      if (
        typeof latest !== "string" ||
        latest.includes("-") ||
        !Object.hasOwn(versions, latest) ||
        versions[latest]?.deprecated
      ) {
        reasons.push("latest must point to a non-deprecated stable release");
      }
    }
  } else {
    if (distTags.latest !== version) {
      reasons.push(`latest must point to active stable release ${version}`);
    }
    if (distTags.next !== undefined) {
      reasons.push("next must be removed when publishing a stable release");
    }
  }

  for (const [candidate, metadata] of Object.entries(versions)) {
    if (!candidate.includes("-") || candidate === version) continue;
    if (!metadata?.deprecated) {
      reasons.push(`superseded prerelease ${candidate} must be deprecated`);
    }
  }

  return { ready: reasons.length === 0, reasons };
}

export function assessImplementationPackageChannels(
  packageMetadata: PackageMetadata,
  version: string,
): { ready: boolean; reasons: string[] } {
  const distTags = packageMetadata["dist-tags"] ?? {};
  const versions = packageMetadata.versions ?? {};
  const prerelease = version.includes("-");
  const reasons: string[] = [];

  if (prerelease) {
    if (distTags.next !== version) reasons.push(`next must point to active prerelease ${version}`);
  } else {
    if (distTags.latest !== version) reasons.push(`latest must point to active stable release ${version}`);
    if (distTags.next !== undefined) reasons.push("next must be removed when publishing a stable release");
  }

  for (const [candidate, metadata] of Object.entries(versions)) {
    if (!candidate.includes("-") || candidate === version) continue;
    if (!metadata?.deprecated) {
      reasons.push(`superseded prerelease ${candidate} must be deprecated`);
    }
  }

  return { ready: reasons.length === 0, reasons };
}

export function assessNpmPublication(
  name: string,
  published: RegistryVersion | null,
  distTags: Record<string, string>,
  tag: string,
  version: string,
  integrity: string,
): "published" | "scanning" | "unpublished" {
  if (published?.dist?.integrity === integrity) return "published";
  if (published) {
    throw new Error(`npm serves unexpected integrity for ${name}@${version}`);
  }
  return distTags[tag] === version ? "scanning" : "unpublished";
}

function run(
  command: string,
  arguments_: readonly string[],
  options: Omit<ExecFileSyncOptionsWithStringEncoding, "encoding"> = {},
): string {
  return execFileSync(command, arguments_, {
    encoding: "utf8",
    ...options,
  });
}

function runInherited(
  command: string,
  arguments_: readonly string[],
  options: ExecFileSyncOptions = {},
): void {
  execFileSync(command, arguments_, {
    stdio: "inherit",
    ...options,
  });
}

function verifyReleaseTarball(tarball: string, manifest: PackageManifest): void {
  const entries = run("tar", ["-tzf", tarball]).trim().split("\n").filter(Boolean);
  assertRootTarballHasNoNativeAddon(entries);
  const packedManifest = JSON.parse(
    run("tar", ["-xOzf", tarball, "package/package.json"]),
  ) as PackageManifest;
  if (packedManifest.name !== manifest.name || packedManifest.version !== manifest.version) {
    throw new Error(
      `Release tarball contains ${packedManifest.name}@${packedManifest.version}, expected ` +
        `${manifest.name}@${manifest.version}`,
    );
  }
  if (Object.keys(packedManifest.dependencies ?? {}).length > 0) {
    throw new Error("Release tarball must not contain JavaScript runtime dependencies");
  }
}

function readRelease(tag: string): GitHubRelease | null {
  const result = spawnSync(
    "gh",
    [
      "release",
      "view",
      tag,
      "--json",
      "isDraft,isPrerelease,tagName,targetCommitish,url,assets",
    ],
    { encoding: "utf8" },
  );
  if (result.status === 0) return JSON.parse(result.stdout) as GitHubRelease;
  if (result.stderr.includes("release not found")) return null;
  throw new Error(result.stderr.trim() || `Unable to inspect GitHub Release ${tag}`);
}

async function readPublishedVersion(
  name: string,
  version: string,
): Promise<RegistryVersion | null> {
  const encodedName = encodeURIComponent(name);
  const response = await fetch(`${registryOrigin}/${encodedName}/${version}`);
  if (response.status === 404) return null;
  if (!response.ok) {
    throw new Error(`npm registry returned ${response.status} for ${name}@${version}`);
  }
  return await response.json() as RegistryVersion;
}

async function readPackageMetadata(name: string): Promise<PackageMetadata> {
  const encodedName = encodeURIComponent(name);
  const response = await fetch(`${registryOrigin}/${encodedName}`);
  if (!response.ok) {
    throw new Error(`npm registry returned ${response.status} while reading package metadata`);
  }
  return await response.json() as PackageMetadata;
}

async function readDistTags(name: string): Promise<Record<string, string>> {
  const encodedName = encodeURIComponent(name);
  const response = await fetch(`${registryOrigin}/-/package/${encodedName}/dist-tags`);
  if (response.status === 404) return {};
  if (!response.ok) {
    throw new Error(`npm registry returned ${response.status} while reading dist-tags`);
  }
  return await response.json() as Record<string, string>;
}

async function reportPendingReleaseAction(
  heading: string,
  message: string,
): Promise<void> {
  console.log(message);
  if (process.env.GITHUB_STEP_SUMMARY) {
    await appendFile(process.env.GITHUB_STEP_SUMMARY, `## ${heading}\n\n${message}\n`);
  }
}

async function reportPendingChannelReconciliation(
  name: string,
  version: string,
  reasons: string[],
): Promise<void> {
  const lines = [
    `Release ${name}@${version} is published, but npm channel reconciliation is pending:`,
    ...reasons.map(reason => `- ${reason}`),
    "Authenticate with npm on the web, reconcile the channels and deprecations, then rerun the Release workflow.",
  ];
  const message = lines.join("\n");
  await reportPendingReleaseAction("npm channel reconciliation required", message);
}

export async function waitForDistTag(
  name: string,
  tag: string,
  version: string,
  {
    attempts = 10,
    intervalMs = 2_000,
    readTags = readDistTags,
    wait = milliseconds => new Promise(resolve => setTimeout(resolve, milliseconds)),
  }: WaitForDistTagOptions = {},
): Promise<Record<string, string>> {
  let observedVersion: string | undefined;
  for (let attempt = 0; attempt < attempts; attempt += 1) {
    const distTags = await readTags(name);
    observedVersion = distTags[tag];
    if (observedVersion === version) return distTags;
    if (attempt < attempts - 1) await wait(intervalMs);
  }
  throw new Error(
    `npm dist-tag ${tag} did not point to ${version} after ${attempts} attempts; ` +
      `last observed ${observedVersion ?? "missing"}`,
  );
}

export async function waitForPublishedVersion(
  name: string,
  version: string,
  integrity: string,
  {
    attempts = 121,
    intervalMs = 10_000,
    readVersion = readPublishedVersion,
    wait = milliseconds => new Promise(resolve => setTimeout(resolve, milliseconds)),
  }: WaitForPublishedVersionOptions = {},
): Promise<RegistryVersion> {
  for (let attempt = 0; attempt < attempts; attempt += 1) {
    const published = await readVersion(name, version);
    if (published?.dist?.integrity === integrity) return published;
    if (published) {
      throw new Error(`npm serves unexpected integrity for ${name}@${version}`);
    }
    if (attempt < attempts - 1) await wait(intervalMs);
  }
  throw new Error(
    `npm did not serve the expected integrity for ${name}@${version} after ${attempts} attempts`,
  );
}

async function assertSameFile(
  expected: string,
  actual: string,
  label: string,
): Promise<void> {
  const [expectedBytes, actualBytes] = await Promise.all([
    readFile(expected),
    readFile(actual),
  ]);
  if (expectedBytes.equals(actualBytes)) return;
  throw new Error(`${label} downloaded from GitHub does not match the local release input`);
}

async function verifyReleaseAssets(
  tag: string,
  artifacts: string[],
  report: string,
  temporaryRoot: string,
): Promise<void> {
  const downloadDirectory = path.join(temporaryRoot, "github-release");
  await rm(downloadDirectory, { recursive: true, force: true });
  await mkdir(downloadDirectory);
  runInherited("gh", ["release", "download", tag, "--dir", downloadDirectory]);
  for (const artifact of artifacts) {
    await assertSameFile(
      artifact,
      path.join(downloadDirectory, path.basename(artifact)),
      `Release asset ${path.basename(artifact)}`,
    );
  }
  await assertSameFile(
    report,
    path.join(downloadDirectory, path.basename(report)),
    "Compatibility Report",
  );
}

function createDraftRelease({
  tag,
  version,
  sha,
  notes,
  artifacts,
  report,
  prerelease,
}: {
  tag: string;
  version: string;
  sha: string;
  notes: string;
  artifacts: string[];
  report: string;
  prerelease: boolean;
}): void {
  const arguments_ = [
    "release",
    "create",
    tag,
    ...artifacts,
    `${report}#Compatibility Report`,
    "--target",
    sha,
    "--title",
    `SheetOM v${version}`,
    "--notes",
    notes,
    "--draft",
    "--latest=false",
  ];
  if (prerelease) arguments_.push("--prerelease");
  runInherited("gh", arguments_);
}

async function releaseArtifactInput(
  input: string | undefined,
  rootManifest: PackageManifest,
  temporaryRoot: string,
): Promise<ReleaseArtifactInput> {
  if (!input) {
    const generatedPack = parsePackResult(run("npm", [
      "pack",
      "--json",
      "--pack-destination",
      temporaryRoot,
    ]));
    if (
      typeof generatedPack.integrity !== "string"
      || typeof generatedPack.size !== "number"
    ) {
      throw new Error("npm pack did not report integrity and size");
    }
    const tarball = path.join(temporaryRoot, generatedPack.filename);
    return {
      legacy: true,
      packages: [{
        name: rootManifest.name,
        version: rootManifest.version,
        role: "root",
        filename: generatedPack.filename,
        integrity: generatedPack.integrity,
        size: generatedPack.size,
        tarball,
      }],
      assets: [tarball],
    };
  }

  const resolved = path.resolve(input);
  if ((await stat(resolved)).isFile()) {
    const pack = await packMetadataForTarball(resolved);
    return {
      legacy: true,
      packages: [{
        name: rootManifest.name,
        version: rootManifest.version,
        role: "root",
        ...pack,
        tarball: resolved,
      }],
      assets: [resolved],
    };
  }

  const manifest = await verifyReleaseArtifactSet(resolved, rootManifest);
  const packages = manifest.packages.map(artifact => ({
    ...artifact,
    tarball: path.join(resolved, artifact.filename),
  }));
  return {
    legacy: false,
    packages,
    assets: [
      ...packages.map(artifact => artifact.tarball),
      path.join(resolved, RELEASE_MANIFEST_FILENAME),
    ],
  };
}

async function publishPackageArtifact(
  artifact: PackedArtifact,
  npmTag: "next" | "latest",
  dryRun: boolean,
): Promise<RegistryVersion | null> {
  let published = await readPublishedVersion(artifact.name, artifact.version);
  const distTags = published ? {} : await readDistTags(artifact.name);
  const publicationState = assessNpmPublication(
    artifact.name,
    published,
    distTags,
    npmTag,
    artifact.version,
    artifact.integrity,
  );
  if (dryRun) return published;
  if (publicationState === "scanning") {
    published = await waitForPublishedVersion(
      artifact.name,
      artifact.version,
      artifact.integrity,
    );
  }
  if (publicationState === "unpublished") {
    runInherited("npm", [
      "publish",
      artifact.tarball,
      "--tag",
      npmTag,
      "--access",
      "public",
    ]);
    published = await waitForPublishedVersion(
      artifact.name,
      artifact.version,
      artifact.integrity,
    );
  }
  if (published?.dist?.integrity !== artifact.integrity) {
    throw new Error(`Published integrity mismatch for ${artifact.name}@${artifact.version}`);
  }
  await waitForDistTag(artifact.name, npmTag, artifact.version);
  return published;
}

async function main(): Promise<void> {
  const dryRun = process.argv.includes("--dry-run");
  const bootstrapImplementations = process.env.SHEETOM_BOOTSTRAP_IMPLEMENTATIONS === "1";
  const manifest = JSON.parse(
    await readFile("package.json", "utf8"),
  ) as PackageManifest;
  const version = manifest.version;
  const tag = `v${version}`;
  const prerelease = version.includes("-");
  const npmTag = npmTagForVersion(version);
  const report = path.resolve(`compatibility/baselines/${version}.json`);
  const changelog = await readFile("CHANGELOG.md", "utf8");
  const notes = extractReleaseNotes(changelog, version);
  const sha = process.env.SHEETOM_RELEASE_SHA ?? run("git", ["rev-parse", "HEAD"]).trim();
  const temporaryRoot = await mkdtemp(path.join(os.tmpdir(), "sheetom-release-"));

  try {
    const suppliedArtifact = process.env.SHEETOM_RELEASE_TARBALL;
    const artifactSet = await releaseArtifactInput(suppliedArtifact, manifest, temporaryRoot);
    const rootArtifact = artifactSet.packages.find(artifact => artifact.role === "root");
    if (!rootArtifact) throw new Error("Release artifact set has no root package");
    verifyReleaseTarball(rootArtifact.tarball, manifest);

    const runtimes = (process.env.SHEETOM_RELEASE_RUNTIMES ?? "node")
      .split(",")
      .filter(Boolean);
    for (const runtime of runtimes) {
      const consumerInput = artifactSet.legacy
        ? rootArtifact.tarball
        : suppliedArtifact
          ? path.resolve(suppliedArtifact)
          : rootArtifact.tarball;
      runInherited(process.execPath, ["scripts/test-tarball.ts", consumerInput, runtime]);
    }

    let release = readRelease(tag);
    let createdRelease = false;
    if (!release && !dryRun) {
      createDraftRelease({
        tag,
        version,
        sha,
        notes,
        artifacts: artifactSet.assets,
        report,
        prerelease,
      });
      release = readRelease(tag);
      createdRelease = true;
    }
    if (createdRelease && release?.targetCommitish !== sha) {
      throw new Error(`GitHub Release ${tag} does not target ${sha}`);
    }
    if (release) {
      await verifyReleaseAssets(tag, artifactSet.assets, report, temporaryRoot);
    }

    if (dryRun) {
      console.log(
        `Dry run verified ${artifactSet.packages.length} package artifacts for ${version}; ` +
          "publishing was skipped.",
      );
      return;
    }

    if (!release?.isDraft) {
      for (const artifact of artifactSet.packages) {
        const published = await readPublishedVersion(artifact.name, artifact.version);
        if (published?.dist?.integrity !== artifact.integrity) {
          throw new Error(
            `Published GitHub Release ${tag} does not match ${artifact.name}@${artifact.version}`,
          );
        }
      }
    }

    const implementationArtifacts = artifactSet.packages.filter(
      artifact => artifact.role !== "root",
    );
    const unpublishedImplementations: PackedArtifact[] = [];
    for (const artifact of implementationArtifacts) {
      const published = await readPublishedVersion(artifact.name, artifact.version);
      if (!published) unpublishedImplementations.push(artifact);
      if (published && published.dist?.integrity !== artifact.integrity) {
        throw new Error(
          `npm already serves ${artifact.name}@${artifact.version} with a different integrity`,
        );
      }
    }

    if (unpublishedImplementations.length > 0 && !bootstrapImplementations) {
      if (!release?.isDraft) {
        throw new Error(
          `Published GitHub Release ${tag} is missing implementation packages`,
        );
      }
      await reportPendingReleaseAction(
        "npm implementation-package bootstrap required",
        `Release ${version} requires the first authenticated publication of:\n` +
          unpublishedImplementations.map(artifact => `- ${artifact.name}`).join("\n") +
          "\nRun the release script with SHEETOM_BOOTSTRAP_IMPLEMENTATIONS=1 against " +
          "this exact artifact set, configure Trusted Publishing, then rerun the workflow.",
      );
      return;
    }

    const artifactsToPublish = bootstrapImplementations
      ? implementationArtifacts
      : artifactSet.packages;
    for (const artifact of artifactsToPublish) {
      await publishPackageArtifact(artifact, npmTag, false);
    }

    if (bootstrapImplementations) {
      await reportPendingReleaseAction(
        "npm implementation-package bootstrap complete",
        `Published ${implementationArtifacts.length} implementation packages for ${version}. ` +
          "Configure their GitHub Actions Trusted Publishers, reconcile npm channels, " +
          "and rerun the Release workflow to publish the root package.",
      );
      return;
    }

    const channelFailures: string[] = [];
    for (const artifact of artifactSet.packages) {
      const packageMetadata = await readPackageMetadata(artifact.name);
      const channelAssessment = artifact.role === "native"
        ? assessImplementationPackageChannels(packageMetadata, version)
        : assessReleaseChannels(packageMetadata, version);
      for (const reason of channelAssessment.reasons) {
        channelFailures.push(`${artifact.name}: ${reason}`);
      }
    }
    if (channelFailures.length > 0) {
      if (!release?.isDraft) {
        throw new Error(
          `Published GitHub Release ${tag} has invalid npm channels: ` +
            channelFailures.join("; "),
        );
      }
      await reportPendingChannelReconciliation("SheetOM artifact set", version, channelFailures);
      return;
    }

    if (release?.isDraft && !dryRun) {
      runInherited("gh", [
        "release",
        "edit",
        tag,
        "--draft=false",
        `--prerelease=${prerelease}`,
        `--latest=${!prerelease}`,
      ]);
      release = readRelease(tag);
    }
    if (!release) {
      throw new Error(`GitHub Release ${tag} is missing`);
    }
    if (release.isDraft || release.isPrerelease !== prerelease) {
      throw new Error(`GitHub Release ${tag} has an unexpected publication state`);
    }
    await verifyReleaseAssets(tag, artifactSet.assets, report, temporaryRoot);
    console.log(
      `Release automation verified ${artifactSet.packages.length} packages ` +
        `for ${version} (${npmTag}) and ${tag}.`,
    );
  } finally {
    await rm(temporaryRoot, { recursive: true, force: true });
  }
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  await main();
}
