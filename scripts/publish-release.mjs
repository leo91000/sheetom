import { execFileSync, spawnSync } from "node:child_process";
import { mkdtemp, mkdir, readFile, rm } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

const registryOrigin = "https://registry.npmjs.org";

export function npmTagForVersion(version) {
  return version.includes("-") ? "next" : "latest";
}

export function extractReleaseNotes(changelog, version) {
  const heading = `## ${version}`;
  const start = changelog.indexOf(heading);
  if (start === -1) return `Release ${version}.`;

  const bodyStart = start + heading.length;
  const nextHeading = changelog.indexOf("\n## ", bodyStart);
  const end = nextHeading === -1 ? changelog.length : nextHeading;
  const body = changelog.slice(bodyStart, end).trim();
  return body === "" ? `Release ${version}.` : body;
}

export function parsePackResult(output) {
  const result = JSON.parse(output);
  if (!Array.isArray(result) || result.length !== 1 || !result[0]?.filename) {
    throw new Error("npm pack did not produce exactly one package artifact");
  }
  return result[0];
}

function run(command, arguments_, options = {}) {
  return execFileSync(command, arguments_, {
    encoding: "utf8",
    ...options,
  });
}

function runInherited(command, arguments_, options = {}) {
  execFileSync(command, arguments_, {
    stdio: "inherit",
    ...options,
  });
}

function readRelease(tag) {
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
  if (result.status === 0) return JSON.parse(result.stdout);
  if (result.stderr.includes("release not found")) return null;
  throw new Error(result.stderr.trim() || `Unable to inspect GitHub Release ${tag}`);
}

async function readPublishedVersion(name, version) {
  const encodedName = encodeURIComponent(name);
  const response = await fetch(`${registryOrigin}/${encodedName}/${version}`);
  if (response.status === 404) return null;
  if (!response.ok) {
    throw new Error(`npm registry returned ${response.status} for ${name}@${version}`);
  }
  return response.json();
}

async function readDistTags(name) {
  const encodedName = encodeURIComponent(name);
  const response = await fetch(`${registryOrigin}/${encodedName}`);
  if (!response.ok) {
    throw new Error(`npm registry returned ${response.status} while reading dist-tags`);
  }
  const document = await response.json();
  return document["dist-tags"] ?? {};
}

export async function waitForDistTag(
  name,
  tag,
  version,
  {
    attempts = 10,
    intervalMs = 2_000,
    readTags = readDistTags,
    wait = milliseconds => new Promise(resolve => setTimeout(resolve, milliseconds)),
  } = {},
) {
  let observedVersion;
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

async function waitForPublishedVersion(name, version, integrity) {
  for (let attempt = 0; attempt < 10; attempt += 1) {
    const published = await readPublishedVersion(name, version);
    if (published?.dist?.integrity === integrity) return published;
    await new Promise(resolve => setTimeout(resolve, 2_000));
  }
  throw new Error(`npm did not serve the expected integrity for ${name}@${version}`);
}

async function assertSameFile(expected, actual, label) {
  const [expectedBytes, actualBytes] = await Promise.all([
    readFile(expected),
    readFile(actual),
  ]);
  if (expectedBytes.equals(actualBytes)) return;
  throw new Error(`${label} downloaded from GitHub does not match the local release input`);
}

async function verifyReleaseAssets(tag, tarball, report, temporaryRoot) {
  const downloadDirectory = path.join(temporaryRoot, "github-release");
  await rm(downloadDirectory, { recursive: true, force: true });
  await mkdir(downloadDirectory);
  runInherited("gh", ["release", "download", tag, "--dir", downloadDirectory]);
  await assertSameFile(
    tarball,
    path.join(downloadDirectory, path.basename(tarball)),
    "Package tarball",
  );
  await assertSameFile(
    report,
    path.join(downloadDirectory, path.basename(report)),
    "Compatibility Report",
  );
}

function createDraftRelease({ tag, version, sha, notes, tarball, report, prerelease }) {
  const arguments_ = [
    "release",
    "create",
    tag,
    `${tarball}#npm package tarball`,
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

async function main() {
  const dryRun = process.argv.includes("--dry-run");
  const manifest = JSON.parse(await readFile("package.json", "utf8"));
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
    const packOutput = run("npm", [
      "pack",
      "--json",
      "--pack-destination",
      temporaryRoot,
    ]);
    const pack = parsePackResult(packOutput);
    const tarball = path.join(temporaryRoot, pack.filename);
    const runtimes = (process.env.SHEETOM_RELEASE_RUNTIMES ?? "node")
      .split(",")
      .filter(Boolean);
    for (const runtime of runtimes) {
      runInherited(process.execPath, ["scripts/test-tarball.mjs", tarball, runtime]);
    }

    let published = await readPublishedVersion(manifest.name, version);
    if (published && published.dist?.integrity !== pack.integrity) {
      throw new Error(
        `npm already serves ${manifest.name}@${version} with a different integrity`,
      );
    }

    let release = readRelease(tag);
    let createdRelease = false;
    if (!release && !dryRun) {
      createDraftRelease({
        tag,
        version,
        sha,
        notes,
        tarball,
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
      await verifyReleaseAssets(tag, tarball, report, temporaryRoot);
    }

    if (!published && dryRun) {
      console.log(`Dry run verified ${manifest.name}@${version}; publishing was skipped.`);
      return;
    }
    if (!published) {
      if (!release?.isDraft) {
        throw new Error("A verified draft GitHub Release is required before npm publication");
      }
      runInherited("npm", [
        "publish",
        tarball,
        "--tag",
        npmTag,
        "--access",
        "public",
      ]);
      published = await waitForPublishedVersion(manifest.name, version, pack.integrity);
    }

    if (published.dist?.integrity !== pack.integrity) {
      throw new Error(`Published integrity mismatch for ${manifest.name}@${version}`);
    }
    await waitForDistTag(manifest.name, npmTag, version);
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
    await verifyReleaseAssets(tag, tarball, report, temporaryRoot);
    console.log(
      `Release automation verified ${manifest.name}@${version} (${npmTag}) and ${tag}.`,
    );
  } finally {
    await rm(temporaryRoot, { recursive: true, force: true });
  }
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  await main();
}
