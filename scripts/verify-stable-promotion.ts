import { execFileSync } from "node:child_process";
import { readFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { isDeepStrictEqual } from "node:util";

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

const referenceVersion = "0.1.0-rc.11";
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

function packageCoordinate({ name, role, target }: ReleaseArtifact): string {
  return `${role}:${name}:${target ?? ""}`;
}

function collectPackageCoordinateMismatches(
  reference: ReleaseManifest,
  candidate: ReleaseManifest,
  mismatches: string[],
): void {
  if (reference.schemaVersion !== candidate.schemaVersion) {
    mismatches.push("release manifest schema changed");
  }
  if (reference.version !== referenceVersion) {
    mismatches.push(`unexpected reference release ${reference.version}`);
  }
  if (candidate.version !== stableVersion) {
    mismatches.push(`candidate ${candidate.version} is not the first stable release`);
  }
  const referenceCoordinates = reference.packages.map(packageCoordinate);
  const candidateCoordinates = candidate.packages.map(packageCoordinate);
  if (!isDeepStrictEqual(candidateCoordinates, referenceCoordinates)) {
    mismatches.push(
      `the stable package cohort differs from RC11: expected ${referenceCoordinates.join(", ")}; ` +
        `received ${candidateCoordinates.join(", ")}`,
    );
  }
}

function collectTarballPromotionMismatches(
  referenceDirectory: string,
  candidateDirectory: string,
  referenceArtifact: ReleaseArtifact,
  candidateArtifact: ReleaseArtifact,
  engineDigests: readonly string[],
  mismatches: string[],
): void {
  const referenceTarball = path.join(referenceDirectory, referenceArtifact.filename);
  const candidateTarball = path.join(candidateDirectory, candidateArtifact.filename);
  let referenceEntries: string[];
  let candidateEntries: string[];
  try {
    referenceEntries = tarEntries(referenceTarball);
  } catch (error) {
    mismatches.push(`${referenceArtifact.name} reference tarball is unreadable: ${errorMessage(error)}`);
    return;
  }
  try {
    candidateEntries = tarEntries(candidateTarball);
  } catch (error) {
    mismatches.push(`${candidateArtifact.name} candidate tarball is unreadable: ${errorMessage(error)}`);
    return;
  }
  if (!isDeepStrictEqual(candidateEntries, referenceEntries)) {
    mismatches.push(`${candidateArtifact.name} changed package topology`);
  }
  const candidateEntrySet = new Set(candidateEntries);

  for (const entry of referenceEntries) {
    if (entry.endsWith("/") || !candidateEntrySet.has(entry)) continue;
    let referenceBytes: Buffer;
    let candidateBytes: Buffer;
    try {
      referenceBytes = tarEntry(referenceTarball, entry);
      candidateBytes = tarEntry(candidateTarball, entry);
    } catch (error) {
      mismatches.push(`${candidateArtifact.name} could not compare ${entry}: ${errorMessage(error)}`);
      continue;
    }
    const extension = path.extname(entry);

    if (binaryExtensions.has(extension)) {
      if (referenceBytes.length === 0) {
        mismatches.push(`${referenceArtifact.name} contains an empty ${entry}`);
      }
      if (candidateBytes.length === 0) {
        mismatches.push(`${candidateArtifact.name} contains an empty ${entry}`);
      }
      continue;
    }

    if (entry === "package/package.json") {
      try {
        const candidateManifest = normalizeVersions(JSON.parse(candidateBytes.toString("utf8")));
        const referenceManifest = normalizeVersions(JSON.parse(referenceBytes.toString("utf8")));
        if (!isDeepStrictEqual(candidateManifest, referenceManifest)) {
          mismatches.push(`${candidateArtifact.name} changed its normalized package manifest`);
        }
      } catch (error) {
        mismatches.push(
          `${candidateArtifact.name} contains an unreadable package manifest: ${errorMessage(error)}`,
        );
      }
      continue;
    }

    try {
      if (
        normalizedText(entry, candidateBytes, engineDigests) !==
        normalizedText(entry, referenceBytes, engineDigests)
      ) {
        mismatches.push(
          `${candidateArtifact.name} changed ${entry} beyond stable-version metadata`,
        );
      }
    } catch (error) {
      mismatches.push(`${candidateArtifact.name} could not normalize ${entry}: ${errorMessage(error)}`);
    }
  }
}

function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : `${error}`;
}

export class StablePromotionError extends AggregateError {
  readonly mismatches: readonly string[];

  constructor(mismatches: readonly string[]) {
    const summary = mismatches.map(mismatch => `- ${mismatch}`).join("\n");
    super(
      mismatches.map(mismatch => new Error(mismatch)),
      `Stable promotion verification found ${mismatches.length} mismatch(es):\n${summary}`,
    );
    this.name = "StablePromotionError";
    this.mismatches = mismatches;
  }
}

function throwMismatches(mismatches: readonly string[]): void {
  if (mismatches.length > 0) throw new StablePromotionError(mismatches);
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

function collectStableSourcePromotionMismatches(
  baseSha: string,
  repositoryRoot: string,
  mismatches: string[],
): void {
  try {
    const baseManifest = JSON.parse(
      run("git", ["show", `${baseSha}:package.json`], repositoryRoot).toString("utf8"),
    ) as { version?: string };
    if (baseManifest.version !== referenceVersion) {
      mismatches.push(
        `stable promotion must be based on RC11, received ${baseManifest.version ?? "no version"}`,
      );
    }
  } catch (error) {
    mismatches.push(`could not inspect stable promotion base ${baseSha}: ${errorMessage(error)}`);
  }

  try {
    const changedFiles = run(
      "git",
      ["diff", "--name-only", `${baseSha}...HEAD`],
      repositoryRoot,
    ).toString("utf8").split("\n").filter(Boolean);
    const unexpected = changedFiles.filter(
      filename => !allowedStablePaths.some(pattern => pattern.test(filename)),
    );
    if (unexpected.length > 0) {
      mismatches.push(`stable promotion contains non-release source changes: ${unexpected.join(", ")}`);
    }
  } catch (error) {
    mismatches.push(`could not inspect stable promotion source changes: ${errorMessage(error)}`);
  }
}

export function assertStableSourcePromotion(baseSha: string, repositoryRoot = process.cwd()): void {
  const mismatches: string[] = [];
  collectStableSourcePromotionMismatches(baseSha, repositoryRoot, mismatches);
  throwMismatches(mismatches);
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
  const mismatches: string[] = [];
  collectPackageCoordinateMismatches(reference, candidate, mismatches);
  if (baseSha) collectStableSourcePromotionMismatches(baseSha, process.cwd(), mismatches);
  const engineDigests = [
    engineDigest(referenceDirectory, reference),
    engineDigest(candidateDirectory, candidate),
  ].filter((digest): digest is string => typeof digest === "string");

  const candidates = new Map(
    candidate.packages.map(artifact => [packageCoordinate(artifact), artifact]),
  );
  for (const referenceArtifact of reference.packages) {
    const candidateArtifact = candidates.get(packageCoordinate(referenceArtifact));
    if (!candidateArtifact) continue;
    collectTarballPromotionMismatches(
      referenceDirectory,
      candidateDirectory,
      referenceArtifact,
      candidateArtifact,
      engineDigests,
      mismatches,
    );
  }
  throwMismatches(mismatches);
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
  try {
    await verifyStablePromotion({
      referenceDirectory: path.resolve(referenceDirectory),
      candidateDirectory: path.resolve(candidateDirectory),
      baseSha: argument("base-sha"),
    });
    console.log("Verified normalized RC11-to-0.1.0 artifact promotion.");
  } catch (error) {
    console.error(errorMessage(error));
    process.exitCode = 1;
  }
}
