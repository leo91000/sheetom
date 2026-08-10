import { cp, mkdir, readdir, rm, stat } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

import {
  assertCompleteNativeArtifactNames,
  expectedNativeArtifacts,
} from "./native-artifact-contract.mjs";

const repositoryRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const sourceDirectory = path.resolve(process.argv[2] ?? "native-artifacts");
const nativeDirectory = path.join(repositoryRoot, "native");
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

await mkdir(nativeDirectory, { recursive: true });
for (const entry of await readdir(nativeDirectory)) {
  if (entry.endsWith(".node")) await rm(path.join(nativeDirectory, entry));
}
for (const [filename, artifact] of byName) {
  await cp(artifact, path.join(nativeDirectory, filename));
}

console.log(`Collected ${byName.size} complete native artifacts.`);
