import assert from "node:assert/strict";
import { createRequire } from "node:module";
import { readdir } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const repositoryRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const nativeDirectory = path.join(repositoryRoot, "native");
const nativeArtifacts = (await readdir(nativeDirectory)).filter(
    entry => entry.startsWith("sheetom-native.") && entry.endsWith(".node"),
);

assert.equal(nativeArtifacts.length, 1, "expected exactly one local native artifact");

const require = createRequire(import.meta.url);
const binding = require(path.join(nativeDirectory, nativeArtifacts[0]));

assert.equal(binding.nativeEngineRevision(), "lightningcss-1.33.0-c6a0c3ce-sheetom.2");

const result = binding.canonicalizeDeclarationBlock(
    "background: image-set(url(a.png) 1x, url(b.png) 2x) center/cover no-repeat red",
);

assert.match(result, /background:/u);
assert.match(result, /image-set\(/u);

console.log(`Native foundation loaded ${nativeArtifacts[0]} successfully.`);
