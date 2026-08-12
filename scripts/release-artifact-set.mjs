import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import { readFile, readdir, stat, writeFile } from "node:fs/promises";
import { createRequire } from "node:module";
import path from "node:path";

import {
  assertPlatformTarballEntries,
  assertRootTarballHasNoNativeAddon,
} from "./native-artifact-contract.mjs";

const require = createRequire(import.meta.url);
const { NATIVE_TARGETS } = require("../native/resolve-target.cjs");

export const RELEASE_MANIFEST_FILENAME = "sheetom-release-manifest.json";

export function expectedReleasePackages(rootManifest) {
  const version = rootManifest.version;
  const nativePackages = NATIVE_TARGETS.map(target => ({
    name: target.packageName,
    version,
    role: "native",
    target: target.target,
    nativeArtifact: target.artifact,
  }));

  return [
    ...nativePackages,
    { name: "@sheetom/wasm", version, role: "wasm" },
    { name: rootManifest.name, version, role: "root" },
  ];
}

async function discoverTarballs(directory) {
  const tarballs = [];
  for (const entry of await readdir(directory, { withFileTypes: true })) {
    const entryPath = path.join(directory, entry.name);
    if (entry.isDirectory()) {
      tarballs.push(...await discoverTarballs(entryPath));
      continue;
    }
    if (entry.isFile() && entry.name.endsWith(".tgz")) tarballs.push(entryPath);
  }
  return tarballs.sort();
}

function tarballEntries(tarball) {
  return execFileSync("tar", ["-tzf", tarball], { encoding: "utf8" })
    .split("\n")
    .filter(Boolean);
}

function tarballPackageManifest(tarball) {
  const output = execFileSync(
    "tar",
    ["-xOzf", tarball, "package/package.json"],
    { encoding: "utf8" },
  );
  return JSON.parse(output);
}

async function tarballDigests(tarball) {
  const bytes = await readFile(tarball);
  return {
    sha256: createHash("sha256").update(bytes).digest("hex"),
    integrity: `sha512-${createHash("sha512").update(bytes).digest("base64")}`,
    size: bytes.length,
  };
}

function assertTarballShape(expected, entries) {
  if (expected.role === "native") {
    assertPlatformTarballEntries(entries, expected.nativeArtifact);
    return;
  }

  assertRootTarballHasNoNativeAddon(entries);
  if (expected.role === "wasm") {
    const wasmEntries = entries.filter(entry => entry.endsWith(".wasm"));
    const expectedWasm = "package/dist/sheetom_wasm_bg.wasm";
    if (wasmEntries.length !== 1 || wasmEntries[0] !== expectedWasm) {
      throw new Error(
        `@sheetom/wasm must contain only ${expectedWasm}; received ${wasmEntries.join(", ")}`,
      );
    }
  }
}

export function assertReleaseArtifactManifest(manifest, rootManifest) {
  if (manifest.schemaVersion !== 1) {
    throw new Error("Release artifact manifest must use schema version 1");
  }
  if (manifest.version !== rootManifest.version) {
    throw new Error(
      `Release artifact manifest describes ${manifest.version}, expected ${rootManifest.version}`,
    );
  }

  const expectedPackages = expectedReleasePackages(rootManifest);
  const packages = manifest.packages ?? [];
  if (packages.length !== expectedPackages.length) {
    throw new Error(
      `Release artifact set is incomplete: expected ${expectedPackages.length} packages, ` +
        `received ${packages.length}`,
    );
  }

  const seenNames = new Set();
  const seenFilenames = new Set();
  for (let index = 0; index < expectedPackages.length; index += 1) {
    const expected = expectedPackages[index];
    const artifact = packages[index];
    if (!artifact || artifact.name !== expected.name || artifact.role !== expected.role) {
      throw new Error(
        `Release package ${index + 1} must be ${expected.name} (${expected.role})`,
      );
    }
    if (artifact.version !== expected.version) {
      throw new Error(`${artifact.name} must use lockstep version ${expected.version}`);
    }
    if (seenNames.has(artifact.name)) throw new Error(`Duplicate release package ${artifact.name}`);
    if (seenFilenames.has(artifact.filename)) {
      throw new Error(`Duplicate release tarball ${artifact.filename}`);
    }
    seenNames.add(artifact.name);
    seenFilenames.add(artifact.filename);

    if (!artifact.filename?.endsWith(".tgz")) {
      throw new Error(`${artifact.name} has an invalid tarball filename`);
    }
    if (!Number.isSafeInteger(artifact.size) || artifact.size <= 0) {
      throw new Error(`${artifact.name} has an invalid tarball size`);
    }
    if (!/^[0-9a-f]{64}$/u.test(artifact.sha256 ?? "")) {
      throw new Error(`${artifact.name} has an invalid SHA-256 digest`);
    }
    if (!/^sha512-[A-Za-z0-9+/]+={0,2}$/u.test(artifact.integrity ?? "")) {
      throw new Error(`${artifact.name} has an invalid npm integrity`);
    }
    if (expected.target !== undefined && artifact.target !== expected.target) {
      throw new Error(`${artifact.name} has an invalid native target`);
    }
  }

  const totalSize = packages.reduce((sum, artifact) => sum + artifact.size, 0);
  if (manifest.totalSize !== totalSize) {
    throw new Error(`Release artifact total size must be ${totalSize}`);
  }
}

export async function createReleaseArtifactManifest(directory, rootManifest) {
  const expectedPackages = expectedReleasePackages(rootManifest);
  const expectedByName = new Map(expectedPackages.map(entry => [entry.name, entry]));
  const artifactsByName = new Map();

  for (const tarball of await discoverTarballs(directory)) {
    const packageManifest = tarballPackageManifest(tarball);
    const expected = expectedByName.get(packageManifest.name);
    if (!expected) throw new Error(`Unexpected release package ${packageManifest.name}`);
    if (artifactsByName.has(packageManifest.name)) {
      throw new Error(`Duplicate release package ${packageManifest.name}`);
    }
    if (packageManifest.version !== expected.version) {
      throw new Error(
        `${packageManifest.name} contains ${packageManifest.version}, expected ${expected.version}`,
      );
    }

    const entries = tarballEntries(tarball);
    assertTarballShape(expected, entries);
    const digests = await tarballDigests(tarball);
    artifactsByName.set(packageManifest.name, {
      name: packageManifest.name,
      version: packageManifest.version,
      role: expected.role,
      ...(expected.target === undefined ? {} : { target: expected.target }),
      filename: path.basename(tarball),
      ...digests,
    });
  }

  const packages = expectedPackages.map(expected => artifactsByName.get(expected.name));
  const missing = expectedPackages
    .filter(expected => !artifactsByName.has(expected.name))
    .map(expected => expected.name);
  if (missing.length > 0) {
    throw new Error(`Release artifact set is incomplete: missing ${missing.join(", ")}`);
  }

  const manifest = {
    schemaVersion: 1,
    version: rootManifest.version,
    packages,
    totalSize: packages.reduce((sum, artifact) => sum + artifact.size, 0),
  };
  assertReleaseArtifactManifest(manifest, rootManifest);
  return manifest;
}

export async function writeReleaseArtifactManifest(directory, rootManifest) {
  const manifest = await createReleaseArtifactManifest(directory, rootManifest);
  await writeFile(
    path.join(directory, RELEASE_MANIFEST_FILENAME),
    `${JSON.stringify(manifest, null, 2)}\n`,
  );
  return manifest;
}

export async function verifyReleaseArtifactSet(directory, rootManifest) {
  const recorded = JSON.parse(
    await readFile(path.join(directory, RELEASE_MANIFEST_FILENAME), "utf8"),
  );
  assertReleaseArtifactManifest(recorded, rootManifest);
  const actual = await createReleaseArtifactManifest(directory, rootManifest);
  if (JSON.stringify(recorded) !== JSON.stringify(actual)) {
    throw new Error("Release artifact bytes do not match the recorded manifest");
  }

  for (const artifact of recorded.packages) {
    await stat(path.join(directory, artifact.filename));
  }
  return recorded;
}
