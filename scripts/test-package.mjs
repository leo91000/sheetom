import { execFileSync } from "node:child_process";
import { mkdtemp, mkdir, readFile, rm, writeFile } from "node:fs/promises";
import { createRequire } from "node:module";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

import { readNativeEngineRevision } from "./native-engine-revision.mjs";

const repositoryRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const require = createRequire(import.meta.url);
const typeScriptCli = require.resolve("typescript/bin/tsc");
const { resolveTarget, TARGET_BY_NAME } = require("../native/resolve-target.cjs");
const expectedEngineRevision = await readNativeEngineRevision(repositoryRoot);
const temporaryRoot = await mkdtemp(path.join(os.tmpdir(), "sheetom-package-"));
const packageDirectory = path.join(temporaryRoot, "package");

try {
  await mkdir(packageDirectory);
  const localTargetName = resolveTarget();
  const localTarget = TARGET_BY_NAME.get(localTargetName);
  if (!localTarget) throw new Error("Current platform has no native package target");
  const nativePackResult = JSON.parse(execFileSync(
    process.execPath,
    [
      "scripts/pack-native-packages.mjs",
      temporaryRoot,
      `--target=${localTargetName}`,
    ],
    { cwd: repositoryRoot, encoding: "utf8" },
  ));
  const nativeFilename = nativePackResult[0]?.filename;
  if (!nativeFilename) throw new Error("native npm pack did not produce a tarball");

  const packResult = JSON.parse(execFileSync(
    "npm",
    ["pack", "--json", "--pack-destination", temporaryRoot],
    { cwd: repositoryRoot, encoding: "utf8" },
  ));
  const filename = packResult[0]?.filename;
  if (!filename) throw new Error("npm pack did not produce a tarball");

  const tarball = path.join(temporaryRoot, filename);
  const rootNativeFiles = packResult[0].files.filter(file => file.path.endsWith(".node"));
  if (rootNativeFiles.length !== 0) {
    throw new Error(`root SheetOM tarball contains native addons: ${rootNativeFiles.map(file => file.path).join(", ")}`);
  }
  execFileSync("npm", ["init", "--yes"], { cwd: packageDirectory, stdio: "ignore" });
  execFileSync("npm", [
    "install",
    "--ignore-scripts",
    path.join(temporaryRoot, nativeFilename),
    tarball,
  ], {
    cwd: packageDirectory,
    stdio: "inherit",
  });
  const installedManifest = JSON.parse(await readFile(
    path.join(packageDirectory, "node_modules/sheetom/package.json"),
    "utf8",
  ));
  if (Object.keys(installedManifest.dependencies ?? {}).length > 0) {
    throw new Error("published SheetOM package must not install JavaScript runtime dependencies");
  }
  const installedNativeManifest = JSON.parse(await readFile(
    path.join(
      packageDirectory,
      "node_modules",
      "@sheetom",
      `native-${localTargetName}`,
      "package.json",
    ),
    "utf8",
  ));
  if (installedNativeManifest.name !== localTarget.packageName) {
    throw new Error("installed native package does not match the current platform");
  }

  const typeProbe = `
    import { CSSStyleRule, CSSStyleSheet, parseStyleSheet } from "sheetom";
    const constructed = new CSSStyleSheet();
    const parsed = parseStyleSheet(".x { color: red; }");
    const rule = parsed.cssRules[0];
    if (rule instanceof CSSStyleRule) {
      rule.style.backgroundColor = "blue";
      rule.style.webkitLineClamp = "2";
      rule.style.cssFloat = "inline-start";
      // @ts-expect-error Unknown names are not part of the pinned browser contract.
      rule.style.notACssProperty = "invalid";
    }
    // @ts-expect-error CSSOM creates style rules; consumers cannot construct one.
    new CSSStyleRule(".x");
    void constructed;
  `;
  await writeFile(path.join(packageDirectory, "probe-types.mts"), typeProbe);
  execFileSync(process.execPath, [
    typeScriptCli,
    "--noEmit",
    "--strict",
    "--skipLibCheck",
    "--target",
    "ES2022",
    "--module",
    "NodeNext",
    "--moduleResolution",
    "NodeNext",
    "probe-types.mts",
  ], { cwd: packageDirectory, stdio: "inherit" });

  const esmProbe = `
    import { createRequire } from "node:module";
    import path from "node:path";
    import { CSSFunctionRule, CSSStyleRule, parseStyleSheet } from "sheetom";
    const require = createRequire(import.meta.url);
    const packageRoot = path.resolve(path.dirname(require.resolve("sheetom")), "..");
    const native = require(path.join(packageRoot, "native/index.cjs"));
    if (native.nativeEngineRevision() !== ${JSON.stringify(expectedEngineRevision)}) {
      throw new Error("native engine revision mismatch");
    }
    const nativeTree = JSON.parse(native.parseRuleTreeJson("@media screen {.x {width:1px;}}"));
    if (nativeTree.kind !== "media" || nativeTree.children[0]?.kind !== "style") {
      throw new Error("native rule parser missing");
    }
    const sheet = parseStyleSheet(".x { width: 1px; }");
    const rule = sheet.cssRules[0];
    if (!(rule instanceof CSSStyleRule)) throw new Error("ESM style rule missing");
    rule.style.setProperty("padding", "2px");
    if (!sheet.serialize().includes("padding: 2px")) throw new Error("ESM mutation failed");
    const functionSheet = parseStyleSheet("@function --double(--x <number>: 1) returns <number> { result: calc(var(--x) * 2); }");
    if (!(functionSheet.cssRules[0] instanceof CSSFunctionRule)) {
      throw new Error("ESM custom function rule missing");
    }
    if (functionSheet.cssRules[0].getParameters()[0]?.defaultValue !== "1") {
      throw new Error("ESM custom function parameter missing");
    }
  `;
  await writeFile(path.join(packageDirectory, "probe.mjs"), esmProbe);
  execFileSync(process.execPath, ["probe.mjs"], { cwd: packageDirectory, stdio: "inherit" });

  const cjsProbe = `
    const { CSSFunctionRule, CSSStyleRule, parseStyleSheet } = require("sheetom");
    const sheet = parseStyleSheet(".x { color: red; }");
    if (!(sheet.cssRules[0] instanceof CSSStyleRule)) throw new Error("CJS style rule missing");
    const functionSheet = parseStyleSheet("@function --f() { result: 1; }");
    if (!(functionSheet.cssRules[0] instanceof CSSFunctionRule)) {
      throw new Error("CJS custom function rule missing");
    }
  `;
  await writeFile(path.join(packageDirectory, "probe.cjs"), cjsProbe);
  execFileSync(process.execPath, ["probe.cjs"], { cwd: packageDirectory, stdio: "inherit" });

  const packedPackage = packResult[0];
  const packageManifest = JSON.parse(await readFile(path.join(repositoryRoot, "package.json"), "utf8"));
  console.log(JSON.stringify({
    name: packageManifest.name,
    version: packageManifest.version,
    filename,
    integrity: packedPackage.integrity,
    size: packedPackage.size,
    unpackedSize: packedPackage.unpackedSize,
  }, null, 2));
} finally {
  await rm(temporaryRoot, { recursive: true, force: true });
}
