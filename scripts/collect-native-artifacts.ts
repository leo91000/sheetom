import { cp, mkdir, readdir, stat } from "node:fs/promises";
import { createRequire } from "node:module";
import path from "node:path";
import { fileURLToPath } from "node:url";

import {
  assertCompleteNativeArtifactNames,
  expectedNativeArtifacts,
} from "./native-artifact-contract.ts";

const repositoryRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const require = createRequire(import.meta.url);
const { TARGET_BY_NAME } = require("../native/resolve-target.cjs");
const sourceDirectory = path.resolve(process.argv[2] ?? "native-artifacts");
const expected = new Set(expectedNativeArtifacts);

async function discover(directory) {
  const artifacts = [];
  for (const entry of await readdir(directory, { withFileTypes: true })) {
    const entryPath = path.join(directory, entry.name);
    if (entry.isDirectory()) {
      artifacts.push(...await discover(entryPath));
    } else if (entry.name.endsWith(".node")) {
      artifacts.push(entryPath);
    }
  }
  return artifacts;
}

const artifacts = await discover(sourceDirectory);
const byName = new Map();
for (const artifact of artifacts) {
  const filename = path.basename(artifact);
  if (!expected.has(filename)) throw new Error(`Unexpected native artifact: ${artifact}`);
  if (byName.has(filename)) throw new Error(`Duplicate native artifact: ${filename}`);
  if ((await stat(artifact)).size < 1_000_000) {
    throw new Error(`Native artifact is unexpectedly small: ${artifact}`);
  }
  byName.set(filename, artifact);
}

assertCompleteNativeArtifactNames(byName.keys());

for (const [filename, artifact] of byName) {
  const targetName = filename.slice("sheetom-native.".length, -".node".length);
  const target = TARGET_BY_NAME.get(targetName);
  if (!target) throw new Error(`Native artifact has no package target: ${filename}`);
  const packageDirectory = path.join(repositoryRoot, "packages", `native-${target.target}`);
  await mkdir(packageDirectory, { recursive: true });
  await cp(artifact, path.join(packageDirectory, filename));
}

console.log(`Collected ${byName.size} complete native artifacts.`);
