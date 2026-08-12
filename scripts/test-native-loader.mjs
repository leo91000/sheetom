import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { copyFile, mkdir, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { createRequire } from "node:module";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

import { readNativeEngineRevision } from "./native-engine-revision.mjs";

const require = createRequire(import.meta.url);
const repositoryRoot = fileURLToPath(new URL("..", import.meta.url));
const { detectLinuxLibc, resolveTarget, SUPPORTED_TARGETS } = require(
  `${repositoryRoot}/native/resolve-target.cjs`,
);

assert.equal(SUPPORTED_TARGETS.size, 8);
assert.equal(resolveTarget({ platform: "darwin", arch: "arm64" }), "darwin-arm64");
assert.equal(resolveTarget({ platform: "darwin", arch: "x64" }), "darwin-x64");
assert.equal(resolveTarget({ platform: "win32", arch: "x64" }), "win32-x64-msvc");
assert.equal(resolveTarget({ platform: "win32", arch: "arm64" }), "win32-arm64-msvc");
assert.equal(resolveTarget({ platform: "freebsd", arch: "x64" }), null);
assert.equal(resolveTarget({ platform: "linux", arch: "riscv64" }), null);

const gnuReport = { getReport: () => ({ header: { glibcVersionRuntime: "2.39" } }) };
const muslReport = {
  getReport: () => ({ header: {}, sharedObjects: ["/lib/ld-musl-x86_64.so.1"] }),
};
assert.equal(detectLinuxLibc({ report: gnuReport }), "gnu");
assert.equal(detectLinuxLibc({ report: muslReport }), "musl");
assert.equal(
  detectLinuxLibc({ report: null, readFile: () => "musl libc (x86_64)" }),
  "musl",
);
assert.equal(
  detectLinuxLibc({ report: null, readFile: () => "ldd (GNU libc)" }),
  "gnu",
);
assert.equal(
  detectLinuxLibc({ report: null, readFile: () => { throw new Error("missing"); } }),
  null,
);
assert.equal(
  resolveTarget({ platform: "linux", arch: "x64" }, { report: gnuReport }),
  "linux-x64-gnu",
);
assert.equal(
  resolveTarget({ platform: "linux", arch: "arm64" }, { report: muslReport }),
  "linux-arm64-musl",
);

const binding = require(`${repositoryRoot}/native/index.cjs`);
assert.equal(typeof binding.NativeDeclarationState, "function");
assert.deepEqual(
  JSON.parse(binding.engineAbiIdentity()),
  JSON.parse(await readFile(`${repositoryRoot}/engine-abi.json`, "utf8")),
);
assert.equal(
  binding.nativeEngineRevision(),
  await readNativeEngineRevision(repositoryRoot),
);

async function assertLoaderFailure(expectedCode, installBinding) {
  const temporaryDirectory = await mkdtemp(path.join(os.tmpdir(), "sheetom-loader-"));
  try {
    await copyFile(
      `${repositoryRoot}/native/index.cjs`,
      path.join(temporaryDirectory, "index.cjs"),
    );
    await copyFile(
      `${repositoryRoot}/native/resolve-target.cjs`,
      path.join(temporaryDirectory, "resolve-target.cjs"),
    );
    if (installBinding) await installBinding(temporaryDirectory);

    const probe = `
      try {
        require(${JSON.stringify(path.join(temporaryDirectory, "index.cjs"))});
        process.exitCode = 2;
      } catch (error) {
        if (error.code !== ${JSON.stringify(expectedCode)}) {
          console.error(error);
          process.exitCode = 3;
        }
      }
    `;
    execFileSync(process.execPath, ["--eval", probe], { stdio: "inherit" });
  } finally {
    await rm(temporaryDirectory, { recursive: true, force: true });
  }
}

const localTarget = resolveTarget();
assert.ok(localTarget);
await assertLoaderFailure("SHEETOM_NATIVE_BINDING_MISSING");
await assertLoaderFailure("SHEETOM_NATIVE_BINDING_LOAD_FAILED", async directory => {
  await writeFile(path.join(directory, `sheetom-native.${localTarget}.node`), "not an addon");
});

async function assertFacadeRejectsIdentity(identity, expectedCode) {
  const temporaryDirectory = await mkdtemp(path.join(os.tmpdir(), "sheetom-engine-abi-"));
  try {
    const distDirectory = path.join(temporaryDirectory, "dist");
    const nativeDirectory = path.join(temporaryDirectory, "native");
    await mkdir(distDirectory);
    await mkdir(nativeDirectory);
    await copyFile(`${repositoryRoot}/dist/index.cjs`, path.join(distDirectory, "index.cjs"));
    await writeFile(
      path.join(nativeDirectory, "index.cjs"),
      `module.exports = { engineAbiIdentity: () => ${JSON.stringify(identity)} };\n`,
    );
    const probe = `
      try {
        require(${JSON.stringify(path.join(distDirectory, "index.cjs"))});
        process.exitCode = 2;
      } catch (error) {
        if (error.code !== ${JSON.stringify(expectedCode)}) {
          console.error(error);
          process.exitCode = 3;
        }
      }
    `;
    execFileSync(process.execPath, ["--eval", probe], { stdio: "inherit" });
  } finally {
    await rm(temporaryDirectory, { recursive: true, force: true });
  }
}

const expectedIdentity = JSON.parse(await readFile(`${repositoryRoot}/engine-abi.json`, "utf8"));
await assertFacadeRejectsIdentity(
  JSON.stringify({ ...expectedIdentity, sheetomVersion: "0.0.0-incompatible" }),
  "SHEETOM_ENGINE_ABI_MISMATCH",
);
await assertFacadeRejectsIdentity("not-json", "SHEETOM_ENGINE_ABI_INVALID");

console.log("Native loader selected and loaded the exact local binding.");
