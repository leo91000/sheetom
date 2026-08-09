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

export function hasReleaseVersionChange(currentManifest, previousManifest) {
  return packageVersion(currentManifest, "Current") !==
    packageVersion(previousManifest, "Previous");
}

async function main() {
  const currentRevision = process.env.SHEETOM_RELEASE_CURRENT_REF ?? "HEAD";
  const previousRevision = process.env.SHEETOM_RELEASE_PREVIOUS_REF ?? `${currentRevision}^`;
  const currentManifest = manifestAtRevision(currentRevision);
  const previousManifest = manifestAtRevision(previousRevision);
  const changed = hasReleaseVersionChange(currentManifest, previousManifest);
  const currentVersion = packageVersion(currentManifest, "Current");
  const previousVersion = packageVersion(previousManifest, "Previous");

  console.log(
    changed
      ? `Release version changed from ${previousVersion} to ${currentVersion}.`
      : `Package version remains ${currentVersion}; publication is not eligible.`,
  );
  if (process.env.GITHUB_OUTPUT) {
    await appendFile(process.env.GITHUB_OUTPUT, `version_changed=${changed}\n`);
  }
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  await main();
}
