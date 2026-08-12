import { readFile, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const repositoryRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const check = process.argv.includes("--check");
if (!check && !process.argv.includes("--record")) {
  throw new Error("Usage: sync-wasm-package.mjs --check|--record");
}

const rootManifest = JSON.parse(await readFile(path.join(repositoryRoot, "package.json"), "utf8"));
const packagePath = path.join(repositoryRoot, "packages/wasm/package.json");
const manifest = {
  name: "@sheetom/wasm",
  version: rootManifest.version,
  description: "First-class WebAssembly backend for SheetOM",
  license: rootManifest.license,
  repository: { ...rootManifest.repository, directory: "packages/wasm" },
  type: "module",
  exports: {
    ".": {
      types: "./dist/index.d.ts",
      import: "./dist/index.js",
    },
  },
  types: "./dist/index.d.ts",
  module: "./dist/index.js",
  files: ["dist", "LICENSE", "README.md"],
  sideEffects: false,
  publishConfig: { access: "public" },
};
const expected = `${JSON.stringify(manifest, null, 2)}\n`;

if (check) {
  const actual = await readFile(packagePath, "utf8").catch(() => "");
  if (actual !== expected) throw new Error("packages/wasm/package.json is stale");
} else {
  await writeFile(packagePath, expected);
}

console.log(`Verified @sheetom/wasm@${rootManifest.version}.`);
