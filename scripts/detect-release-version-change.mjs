import { execFileSync } from "node:child_process";
import { appendFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";

function manifestAtRevision(revision) {
  const source = execFileSync(
    "git",
    ["show", `${revision}:package.json`],
    { encoding: "utf8" },
  );
  return JSON.parse(source);
}

function packageVersion(manifest, label) {
  const version = manifest?.version;
  if (typeof version !== "string" || version === "") {
    throw new Error(`${label} package.json does not contain a version`);
  }
  return version;
}

/**
 * @param {{ version?: unknown }} currentManifest
 * @param {{ version?: unknown } | null} previousManifest
 */
export function hasReleaseVersionChange(currentManifest, previousManifest) {
  if (previousManifest === null) return true;
  return packageVersion(currentManifest, "Current") !==
    packageVersion(previousManifest, "Previous");
}

export function latestVersionTag(revision) {
  try {
    return execFileSync(
      "git",
      ["describe", "--tags", "--abbrev=0", "--match", "v[0-9]*", revision],
      { encoding: "utf8", stdio: ["ignore", "pipe", "ignore"] },
    ).trim() || null;
  } catch {
    return null;
  }
}

async function main() {
  const currentRevision = process.env.SHEETOM_RELEASE_CURRENT_REF ?? "HEAD";
  const previousRevision = process.env.SHEETOM_RELEASE_PREVIOUS_REF ??
    latestVersionTag(currentRevision);
  const currentManifest = manifestAtRevision(currentRevision);
  const previousManifest = previousRevision === null
    ? null
    : manifestAtRevision(previousRevision);
  const changed = hasReleaseVersionChange(currentManifest, previousManifest);
  const currentVersion = packageVersion(currentManifest, "Current");
  const previousVersion = previousManifest === null
    ? null
    : packageVersion(previousManifest, "Previous");

  console.log(
    changed
      ? previousVersion === null
        ? `No prior version tag exists; ${currentVersion} is eligible for its first publication.`
        : `Release version changed from tagged ${previousVersion} to ${currentVersion}.`
      : `Package version remains ${currentVersion}; publication is not eligible.`,
  );
  if (process.env.GITHUB_OUTPUT) {
    await appendFile(process.env.GITHUB_OUTPUT, `version_changed=${changed}\n`);
  }
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  await main();
}
