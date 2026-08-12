import { readFile, readdir, stat } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

import { expectedNativePackages } from "./native-artifact-contract.ts";

const repositoryRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const sourceRoot = path.join(repositoryRoot, "src");
const packageManifest = JSON.parse(await readFile(
  path.join(repositoryRoot, "package.json"),
  "utf8",
));
const maximumBundleBytes = 225_000;
const bundlePaths = [
  path.join(repositoryRoot, "dist/index.js"),
  path.join(repositoryRoot, "dist/index.cjs"),
];

const runtimeDependencies = Object.keys(packageManifest.dependencies ?? {});
if (runtimeDependencies.length > 0) {
  throw new Error(
    `SheetOM must not have JavaScript runtime dependencies: ${runtimeDependencies.join(", ")}`,
  );
}

const optionalDependencies = packageManifest.optionalDependencies ?? {};
const optionalPackageNames = Object.keys(optionalDependencies).sort();
const expectedOptionalPackageNames = [...expectedNativePackages].sort();
if (JSON.stringify(optionalPackageNames) !== JSON.stringify(expectedOptionalPackageNames)) {
  throw new Error("SheetOM optional dependencies do not match the native target registry");
}
for (const packageName of expectedNativePackages) {
  if (optionalDependencies[packageName] === packageManifest.version) continue;
  throw new Error(`${packageName} must be pinned exactly to ${packageManifest.version}`);
}

const packagedFiles = packageManifest.files ?? [];
if (
  packagedFiles.includes("native") ||
  !packagedFiles.includes("native/index.cjs") ||
  !packagedFiles.includes("native/resolve-target.cjs")
) {
  throw new Error("The root package must include only the native loader, never native addons");
}

async function sourceFiles(directory) {
  const files = [];
  for (const entry of await readdir(directory, { withFileTypes: true })) {
    const entryPath = path.join(directory, entry.name);
    if (entry.isDirectory()) {
      files.push(...await sourceFiles(entryPath));
      continue;
    }
    if (entry.isFile() && /\.[cm]?[jt]sx?$/.test(entry.name)) files.push(entryPath);
  }
  return files;
}

for (const sourceFile of await sourceFiles(sourceRoot)) {
  const source = await readFile(sourceFile, "utf8");
  if (!/from\s+["'][^"']*compatibility\//.test(source)) continue;
  throw new Error(`Runtime source imports compatibility evidence: ${path.relative(repositoryRoot, sourceFile)}`);
}

for (const bundlePath of bundlePaths) {
  const bundle = await readFile(bundlePath, "utf8");
  const bundleSize = (await stat(bundlePath)).size;
  if (bundleSize > maximumBundleBytes) {
    throw new Error(
      `${path.relative(repositoryRoot, bundlePath)} is ${bundleSize} bytes; ` +
      `the runtime budget is ${maximumBundleBytes} bytes`,
    );
  }
  if (!bundle.includes("computed-initial-longhands@1")) continue;
  throw new Error(`${path.relative(repositoryRoot, bundlePath)} embeds shorthand browser evidence`);
}

console.log(JSON.stringify({
  maximumBundleBytes,
  bundles: Object.fromEntries(await Promise.all(bundlePaths.map(async bundlePath => [
    path.relative(repositoryRoot, bundlePath),
    (await stat(bundlePath)).size,
  ]))),
}, null, 2));
