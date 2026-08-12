import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { readFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

interface ReleaseArtifact {
  name: string;
  version: string;
  role: "native" | "wasm" | "root";
  target?: string;
  filename: string;
}

interface ReleaseManifest {
  schemaVersion: number;
  version: string;
  packages: ReleaseArtifact[];
}

interface PromotionOptions {
  referenceDirectory: string;
  candidateDirectory: string;
  baseSha?: string;
}

const referenceVersion = "0.1.0-rc.8";
const stableVersion = "0.1.0";
const binaryExtensions = new Set([".node", ".wasm"]);

const allowedStablePaths = [
  /^\.changeset\//u,
  /^CHANGELOG\.md$/u,
  /^Cargo\.lock$/u,
  /^Cargo\.toml$/u,
  /^compatibility\/baselines\/0\.1\.0\.json$/u,
  /^crates\/[^/]+\/Cargo\.toml$/u,
  /^engine-abi\.json$/u,
  /^package-lock\.json$/u,
  /^package\.json$/u,
  /^packages\/[^/]+\/CHANGELOG\.md$/u,
  /^packages\/[^/]+\/package\.json$/u,
];

function run(command: string, arguments_: readonly string[], cwd?: string): Buffer {
  return execFileSync(command, arguments_, {
    cwd,
    encoding: "buffer",
    maxBuffer: 64 * 1024 * 1024,
  });
}

async function readManifest(directory: string): Promise<ReleaseManifest> {
  return JSON.parse(
    await readFile(path.join(directory, "sheetom-release-manifest.json"), "utf8"),
  ) as ReleaseManifest;
}

function tarEntries(tarball: string): string[] {
  return run("tar", ["-tzf", tarball])
    .toString("utf8")
    .split("\n")
    .filter(Boolean)
    .sort();
}

function tarEntry(tarball: string, entry: string): Buffer {
  return run("tar", ["-xOzf", tarball, entry]);
}

function normalizeVersions(value: unknown): unknown {
  if (typeof value === "string") {
    return value
      .replaceAll(referenceVersion, "<SHEETOM_VERSION>")
      .replaceAll(stableVersion, "<SHEETOM_VERSION>");
  }
  if (Array.isArray(value)) return value.map(normalizeVersions);
  if (!value || typeof value !== "object") return value;
  return Object.fromEntries(
    Object.entries(value).map(([key, entry]) => [key, normalizeVersions(entry)]),
  );
}

function normalizedText(entry: string, bytes: Buffer, engineDigests: readonly string[]): string {
  if (entry === "package/engine-abi.json") {
    const identity = normalizeVersions(JSON.parse(bytes.toString("utf8"))) as Record<string, unknown>;
    identity.syntaxEngineSetSha256 = "<VERSIONED_ENGINE_SOURCE_DIGEST>";
    return `${JSON.stringify(identity, null, 2)}\n`;
  }
  let normalized = bytes
    .toString("utf8")
    .replaceAll(referenceVersion, "<SHEETOM_VERSION>")
    .replaceAll(stableVersion, "<SHEETOM_VERSION>");
  for (const digest of engineDigests) {
    normalized = normalized.replaceAll(digest, "<VERSIONED_ENGINE_SOURCE_DIGEST>");
  }
  return normalized;
}

function assertPackageCoordinates(
  reference: ReleaseManifest,
  candidate: ReleaseManifest,
): void {
  assert.equal(reference.schemaVersion, candidate.schemaVersion, "release manifest schema changed");
  assert.equal(reference.version, referenceVersion, "unexpected reference release");
  assert.equal(candidate.version, stableVersion, "candidate is not the first stable release");
  assert.deepEqual(
    candidate.packages.map(({ name, role, target }) => ({ name, role, target })),
    reference.packages.map(({ name, role, target }) => ({ name, role, target })),
    "the stable package cohort differs from RC8",
  );
}

function assertTarballPromotion(
  referenceDirectory: string,
  candidateDirectory: string,
  referenceArtifact: ReleaseArtifact,
  candidateArtifact: ReleaseArtifact,
  engineDigests: readonly string[],
): void {
  const referenceTarball = path.join(referenceDirectory, referenceArtifact.filename);
  const candidateTarball = path.join(candidateDirectory, candidateArtifact.filename);
  const referenceEntries = tarEntries(referenceTarball);
  const candidateEntries = tarEntries(candidateTarball);
  assert.deepEqual(candidateEntries, referenceEntries, `${candidateArtifact.name} changed package topology`);

  for (const entry of referenceEntries) {
    if (entry.endsWith("/")) continue;
    const referenceBytes = tarEntry(referenceTarball, entry);
    const candidateBytes = tarEntry(candidateTarball, entry);
    const extension = path.extname(entry);

    if (binaryExtensions.has(extension)) {
      assert.ok(referenceBytes.length > 0, `${referenceArtifact.name} contains an empty ${entry}`);
      assert.ok(candidateBytes.length > 0, `${candidateArtifact.name} contains an empty ${entry}`);
      continue;
    }

    if (entry === "package/package.json") {
      assert.deepEqual(
        normalizeVersions(JSON.parse(candidateBytes.toString("utf8"))),
        normalizeVersions(JSON.parse(referenceBytes.toString("utf8"))),
        `${candidateArtifact.name} changed its normalized package manifest`,
      );
      continue;
    }

    assert.equal(
      normalizedText(entry, candidateBytes, engineDigests),
      normalizedText(entry, referenceBytes, engineDigests),
      `${candidateArtifact.name} changed ${entry} beyond stable-version metadata`,
    );
  }
}

function engineDigest(directory: string, manifest: ReleaseManifest): string | undefined {
  const root = manifest.packages.find(artifact => artifact.role === "root");
  if (!root) return undefined;
  const tarball = path.join(directory, root.filename);
  const entries = tarEntries(tarball);
  if (!entries.includes("package/engine-abi.json")) return undefined;
  const identity = JSON.parse(
    tarEntry(tarball, "package/engine-abi.json").toString("utf8"),
  ) as { syntaxEngineSetSha256?: string };
  return identity.syntaxEngineSetSha256;
}

export function assertStableSourcePromotion(baseSha: string, repositoryRoot = process.cwd()): void {
  const baseManifest = JSON.parse(
    run("git", ["show", `${baseSha}:package.json`], repositoryRoot).toString("utf8"),
  ) as { version?: string };
  assert.equal(baseManifest.version, referenceVersion, "stable promotion must be based on RC8");

  const changedFiles = run(
    "git",
    ["diff", "--name-only", `${baseSha}...HEAD`],
    repositoryRoot,
  ).toString("utf8").split("\n").filter(Boolean);
  const unexpected = changedFiles.filter(
    filename => !allowedStablePaths.some(pattern => pattern.test(filename)),
  );
  assert.deepEqual(
    unexpected,
    [],
    `stable promotion contains non-release source changes: ${unexpected.join(", ")}`,
  );
}

export async function verifyStablePromotion({
  referenceDirectory,
  candidateDirectory,
  baseSha,
}: PromotionOptions): Promise<void> {
  const [reference, candidate] = await Promise.all([
    readManifest(referenceDirectory),
    readManifest(candidateDirectory),
  ]);
  assertPackageCoordinates(reference, candidate);
  if (baseSha) assertStableSourcePromotion(baseSha);
  const engineDigests = [
    engineDigest(referenceDirectory, reference),
    engineDigest(candidateDirectory, candidate),
  ].filter((digest): digest is string => typeof digest === "string");

  for (let index = 0; index < reference.packages.length; index += 1) {
    const referenceArtifact = reference.packages[index];
    const candidateArtifact = candidate.packages[index];
    assert.ok(referenceArtifact && candidateArtifact, "release package cohort is incomplete");
    assertTarballPromotion(
      referenceDirectory,
      candidateDirectory,
      referenceArtifact,
      candidateArtifact,
      engineDigests,
    );
  }
}

function argument(name: string): string | undefined {
  const prefix = `--${name}=`;
  return process.argv.find(entry => entry.startsWith(prefix))?.slice(prefix.length);
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  const referenceDirectory = argument("reference");
  const candidateDirectory = argument("candidate");
  if (!referenceDirectory || !candidateDirectory) {
    throw new Error(
      "Usage: verify-stable-promotion.ts --reference=<directory> --candidate=<directory> " +
        "[--base-sha=<sha>]",
    );
  }
  await verifyStablePromotion({
    referenceDirectory: path.resolve(referenceDirectory),
    candidateDirectory: path.resolve(candidateDirectory),
    baseSha: argument("base-sha"),
  });
  console.log("Verified normalized RC8-to-0.1.0 artifact promotion.");
}
