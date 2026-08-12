import { createHash } from "node:crypto";
import { execFileSync } from "node:child_process";
import { cp, mkdir, mkdtemp, readFile, rm, stat, writeFile } from "node:fs/promises";
import { createRequire } from "node:module";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

import { nativePackageManifest } from "./native-package-contract.mjs";

const repositoryRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const require = createRequire(import.meta.url);
const { NATIVE_TARGETS, TARGET_BY_NAME } = require("../native/resolve-target.cjs");
const outputDirectory = path.resolve(process.argv[2] ?? "package-artifact");
const targetArgument = process.argv.find(argument => argument.startsWith("--target="));
const selectedTarget = targetArgument ? TARGET_BY_NAME.get(targetArgument.slice(9)) : null;
if (targetArgument && !selectedTarget) throw new Error(`Unknown native package target: ${targetArgument}`);
const targets = selectedTarget ? [selectedTarget] : NATIVE_TARGETS;
const rootManifest = JSON.parse(await readFile(path.join(repositoryRoot, "package.json"), "utf8"));
const stagingRoot = await mkdtemp(path.join(os.tmpdir(), "sheetom-native-pack-"));
await mkdir(outputDirectory, { recursive: true });

function npmPack(directory) {
  const result = JSON.parse(execFileSync(
    "npm",
    ["pack", "--json", "--pack-destination", outputDirectory],
    { cwd: directory, encoding: "utf8" },
  ));
  const packed = result[0];
  if (!packed?.filename || !packed.integrity) throw new Error(`npm pack failed for ${directory}`);
  return packed;
}

async function digest(filename) {
  const bytes = await readFile(filename);
  return createHash("sha256").update(bytes).digest("hex");
}

try {
  const packages = [];
  for (const target of targets) {
    const sourceDirectory = path.join(repositoryRoot, "packages", `native-${target.target}`);
    const artifact = path.join(sourceDirectory, target.artifact);
    if ((await stat(artifact)).size < 1_000_000) {
      throw new Error(`Native artifact is unexpectedly small: ${artifact}`);
    }
    const stagingDirectory = path.join(stagingRoot, `native-${target.target}`);
    await mkdir(stagingDirectory);
    for (const filename of ["index.cjs", "LICENSE", "README.md", target.artifact]) {
      await cp(path.join(sourceDirectory, filename), path.join(stagingDirectory, filename));
    }
    await writeFile(
      path.join(stagingDirectory, "package.json"),
      `${JSON.stringify(nativePackageManifest(rootManifest, target, { publishable: true }), null, 2)}\n`,
    );
    const packed = npmPack(stagingDirectory);
    const packedAddons = packed.files
      .map(file => file.path)
      .filter(filename => filename.endsWith(".node"));
    if (
      packedAddons.length !== 1 ||
      packedAddons[0] !== target.artifact
    ) {
      throw new Error(
        `${target.packageName} must pack only ${target.artifact}; received ${packedAddons.join(", ")}`,
      );
    }
    const tarball = path.join(outputDirectory, packed.filename);
    packages.push({
      name: target.packageName,
      version: rootManifest.version,
      target: target.target,
      artifact: target.artifact,
      filename: packed.filename,
      integrity: packed.integrity,
      sha256: await digest(tarball),
      size: (await stat(tarball)).size,
      unpackedSize: packed.unpackedSize,
    });
  }
  console.log(JSON.stringify(packages, null, 2));
} finally {
  await rm(stagingRoot, { recursive: true, force: true });
}
