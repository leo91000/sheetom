import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { readFile } from "node:fs/promises";

const manifest = JSON.parse(await readFile("package.json", "utf8")) as {
  dependencies?: Record<string, string>;
  devDependencies?: Record<string, string>;
  files?: string[];
};

assert.deepEqual(
  Object.keys(manifest.dependencies ?? {}),
  [],
  "the root package must not install JavaScript runtime dependencies",
);

const obsoleteDevelopmentDependencies = [
  "@csstools/css-tokenizer",
  "lightningcss",
];
for (const dependency of obsoleteDevelopmentDependencies) {
  assert.equal(
    Object.hasOwn(manifest.devDependencies ?? {}, dependency),
    false,
    `${dependency} is obsolete because SheetOM owns the vendored Rust parser stack`,
  );
}

assert.deepEqual(manifest.files, [
  "dist",
  "engine-abi.json",
  "native/index.cjs",
  "native/resolve-target.cjs",
  "docs/api.md",
  "LICENSE",
  "README.md",
  "SECURITY.md",
]);

const trackedScripts = execFileSync("git", ["ls-files", "scripts"], {
  encoding: "utf8",
})
  .split("\n")
  .filter(Boolean);
const legacyScripts = trackedScripts.filter(file =>
  file.endsWith(".mjs") || file.endsWith(".mts") || file.endsWith(".d.mts")
);
assert.deepEqual(
  legacyScripts,
  [],
  `repository scripts must use native TypeScript: ${legacyScripts.join(", ")}`,
);

console.log(
  `Verified a dependency-free runtime and ${trackedScripts.length} TypeScript repository scripts.`,
);
