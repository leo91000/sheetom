import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { createRequire } from "node:module";
import test from "node:test";

import {
  nativePackageManifest,
  nativePlatformMetadata,
} from "./native-package-contract.mjs";

const require = createRequire(import.meta.url);
const { NATIVE_TARGETS } = require("../native/resolve-target.cjs");
const changesetConfig = JSON.parse(readFileSync(new URL("../.changeset/config.json", import.meta.url)));
const rootManifest = {
  name: "sheetom",
  version: "1.2.3-test.4",
  license: "MIT",
  repository: { type: "git", url: "https://example.test/sheetom.git" },
  engines: { node: ">=22" },
};

test("Changesets versions the complete artifact set as one fixed cohort", () => {
  const expected = [
    rootManifest.name,
    "@sheetom/wasm",
    ...NATIVE_TARGETS.map(target => target.packageName),
  ];
  assert.equal(changesetConfig.fixed.length, 1);
  assert.deepEqual(new Set(changesetConfig.fixed[0]), new Set(expected));
  assert.equal(changesetConfig.fixed[0].length, expected.length);
});

test("the native target registry owns thirteen unique implementation packages", () => {
  assert.equal(NATIVE_TARGETS.length, 13);
  for (const key of ["triple", "target", "artifact", "packageName"]) {
    assert.equal(new Set(NATIVE_TARGETS.map(target => target[key])).size, 13, key);
  }
});

test("workspace manifests stay installable while packed manifests select one platform", () => {
  for (const target of NATIVE_TARGETS) {
    const workspace = nativePackageManifest(rootManifest, target);
    const packed = nativePackageManifest(rootManifest, target, { publishable: true });
    assert.equal(workspace.name, target.packageName);
    assert.equal(workspace.version, rootManifest.version);
    assert.equal(workspace.preferUnplugged, true);
    assert.equal(Object.hasOwn(workspace, "os"), false);
    assert.deepEqual(
      { os: packed.os, cpu: packed.cpu, libc: packed.libc },
      { ...nativePlatformMetadata(target), libc: packed.libc },
    );
    assert.deepEqual(
      packed.files.filter(filename => filename.endsWith(".node")),
      [target.artifact],
    );
  }
});
