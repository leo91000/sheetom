import { mkdir, readFile, writeFile } from "node:fs/promises";
import { createRequire } from "node:module";
import path from "node:path";
import { fileURLToPath } from "node:url";

import { nativePackageManifest } from "./native-package-contract.ts";

const repositoryRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const require = createRequire(import.meta.url);
const { NATIVE_TARGETS } = require("../native/resolve-target.cjs");
const check = process.argv.includes("--check");
if (!check && !process.argv.includes("--record")) {
  throw new Error("Usage: sync-native-packages.ts --check|--record");
}

const rootManifest = JSON.parse(await readFile(path.join(repositoryRoot, "package.json"), "utf8"));
const license = await readFile(path.join(repositoryRoot, "LICENSE"), "utf8");
const loader = await readFile(path.join(repositoryRoot, "native/platform-package.cjs"), "utf8");

async function synchronize(filename, expected) {
  if (!check) {
    await writeFile(filename, expected);
    return;
  }
  const actual = await readFile(filename, "utf8").catch(() => "");
  if (actual !== expected) throw new Error(`${path.relative(repositoryRoot, filename)} is stale`);
}

for (const target of NATIVE_TARGETS) {
  const directory = path.join(repositoryRoot, "packages", `native-${target.target}`);
  if (!check) await mkdir(directory, { recursive: true });
  await synchronize(
    path.join(directory, "package.json"),
    `${JSON.stringify(nativePackageManifest(rootManifest, target), null, 2)}\n`,
  );
  await synchronize(path.join(directory, "index.cjs"), loader);
  await synchronize(path.join(directory, "LICENSE"), license);
  await synchronize(
    path.join(directory, "README.md"),
    `# ${target.packageName}\n\nPlatform-specific native engine for ` +
      `[SheetOM](https://www.npmjs.com/package/sheetom) on \`${target.target}\`. ` +
      "Install `sheetom`; package managers select this implementation artifact automatically.\n",
  );
}

if (!check) {
  rootManifest.workspaces = [".", "packages/native-*", "packages/wasm"];
  rootManifest.optionalDependencies = Object.fromEntries(
    NATIVE_TARGETS
      .map(target => [target.packageName, rootManifest.version])
      .sort(([left], [right]) => left.localeCompare(right)),
  );
  await writeFile(
    path.join(repositoryRoot, "package.json"),
    `${JSON.stringify(rootManifest, null, 2)}\n`,
  );
}

console.log(`Verified ${NATIVE_TARGETS.length} native platform package manifests.`);
