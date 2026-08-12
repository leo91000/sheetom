import { execFileSync } from "node:child_process";
import { copyFile, mkdir, stat } from "node:fs/promises";
import { createRequire } from "node:module";
import path from "node:path";
import { fileURLToPath } from "node:url";

const repositoryRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const require = createRequire(import.meta.url);
const { TARGET_BY_TRIPLE } = require("../native/resolve-target.cjs");
const target = process.argv[2];
const supportedCrossTargets = new Set([
  "armv7-unknown-linux-gnueabihf",
  "armv7-unknown-linux-musleabihf",
  "powerpc64le-unknown-linux-gnu",
  "s390x-unknown-linux-gnu",
]);

if (!supportedCrossTargets.has(target)) {
  throw new Error(`Unsupported cross-built native target: ${target ?? "<missing>"}`);
}
const metadata = TARGET_BY_TRIPLE.get(target);
if (!metadata) throw new Error(`Native package registry is missing ${target}`);

execFileSync(
  "cross",
  [
    "build",
    "--release",
    "--package",
    "sheetom-native",
    "--target",
    target,
    "--manifest-path",
    "crates/sheetom-native/Cargo.toml",
  ],
  { cwd: repositoryRoot, stdio: "inherit" },
);

const compiled = path.join(
  repositoryRoot,
  "target",
  target,
  "release",
  "libsheetom_native.so",
);
const artifactSize = (await stat(compiled)).size;
if (artifactSize < 1_000_000) {
  throw new Error(`Native artifact is unexpectedly small: ${compiled} (${artifactSize} bytes)`);
}

const nativeArtifact = path.join(repositoryRoot, "native", metadata.artifact);
const packageDirectory = path.join(repositoryRoot, "packages", `native-${metadata.target}`);
await mkdir(path.dirname(nativeArtifact), { recursive: true });
await mkdir(packageDirectory, { recursive: true });
await copyFile(compiled, nativeArtifact);
await copyFile(compiled, path.join(packageDirectory, metadata.artifact));

console.log(`Built ${metadata.artifact} (${artifactSize} bytes) with cross.`);
