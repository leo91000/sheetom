import { execFileSync } from "node:child_process";
import { mkdtemp, mkdir, readFile, readdir, rm, writeFile } from "node:fs/promises";
import { createRequire } from "node:module";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

import { readNativeEngineRevision } from "./native-engine-revision.ts";

const [artifactPath, runtime = "node"] = process.argv.slice(2);
if (!artifactPath) throw new Error("Usage: test-tarball.ts <tarball-or-directory> [node|bun|deno]");

const repositoryRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const require = createRequire(import.meta.url);
const { resolveTarget } = require("../native/resolve-target.cjs");
const expectedEngineRevision = await readNativeEngineRevision(repositoryRoot);
const artifact = path.resolve(artifactPath);
const localTarget = resolveTarget();
if (!localTarget) throw new Error("Current platform has no native package target");
const tarballs = await resolveTarballs(artifact);
const temporaryRoot = await mkdtemp(path.join(os.tmpdir(), "sheetom-consumer-"));
const packageDirectory = path.join(temporaryRoot, "consumer");

function runNpm(arguments_, options) {
  if (process.platform !== "win32") {
    execFileSync("npm", arguments_, options);
    return;
  }

  execFileSync(process.env.ComSpec ?? "cmd.exe", ["/d", "/s", "/c", "npm", ...arguments_], options);
}

async function resolveTarballs(input) {
  const directory = input.endsWith(".tgz") ? path.dirname(input) : input;
  const entries = (await readdir(directory)).filter(entry => entry.endsWith(".tgz"));
  const inputName = path.basename(input);
  const root = input.endsWith(".tgz") && /^sheetom-\d/.test(inputName)
    ? inputName
    : entries.find(entry => /^sheetom-\d/.test(entry));
  const native = entries.find(entry => entry.startsWith(`sheetom-native-${localTarget}-`));
  if (!root || !native) {
    throw new Error(
      `Expected root and @sheetom/native-${localTarget} tarballs in ${directory}`,
    );
  }
  return [path.join(directory, native), path.join(directory, root)];
}

try {
  await mkdir(packageDirectory);
  runNpm(["init", "--yes"], { cwd: packageDirectory, stdio: "ignore" });
  runNpm(["install", "--ignore-scripts", ...tarballs], {
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
      `native-${localTarget}`,
      "package.json",
    ),
    "utf8",
  ));
  if (installedNativeManifest.name !== `@sheetom/native-${localTarget}`) {
    throw new Error("installed native package does not match the current platform");
  }

  const esmProbe = `
    import { createRequire } from "node:module";
    import path from "node:path";
    import { CSSFunctionRule, CSSStyleRule, CSSMediaRule, parseStyleSheet } from "sheetom";
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
    const sheet = parseStyleSheet("@media screen { .x { width: 1px; } }");
    const media = sheet.cssRules[0];
    if (!(media instanceof CSSMediaRule)) throw new Error("specialized media rule missing");
    const rule = media.cssRules[0];
    if (!(rule instanceof CSSStyleRule)) throw new Error("style rule missing");
    rule.style.setProperty("padding", "2px");
    if (!sheet.serialize().includes("padding: 2px")) throw new Error("mutation failed");
    const functionSheet = parseStyleSheet("@function --double(--x <number>: 1) returns <number> { result: calc(var(--x) * 2); }");
    const functionRule = functionSheet.cssRules[0];
    if (!(functionRule instanceof CSSFunctionRule)) throw new Error("custom function rule missing");
    if (functionRule.getParameters()[0]?.defaultValue !== "1") {
      throw new Error("custom function parameter missing");
    }
  `;
  await writeFile(path.join(packageDirectory, "probe.mjs"), esmProbe);

  switch (runtime) {
    case "node":
      execFileSync(process.execPath, ["probe.mjs"], { cwd: packageDirectory, stdio: "inherit" });
      await writeFile(
        path.join(packageDirectory, "probe.cjs"),
        `
          const { CSSFunctionRule, CSSStyleRule, CSSStyleSheet, parseStyleSheet } = require("sheetom");
          const sheet = new CSSStyleSheet();
          sheet.insertRule(".x {}");
          const rule = sheet.cssRules[0];
          if (!(rule instanceof CSSStyleRule)) throw new Error("CJS style rule missing");
          rule.style.setProperty("background", "image-set(url(a.png) 1x, url(b.png) 2x) center/cover no-repeat red");
          if (!sheet.serialize().includes("image-set(")) throw new Error("CJS native mutation failed");
          const functionSheet = parseStyleSheet("@function --f() { result: 1; }");
          if (!(functionSheet.cssRules[0] instanceof CSSFunctionRule)) {
            throw new Error("CJS custom function rule missing");
          }
        `,
      );
      execFileSync(process.execPath, ["probe.cjs"], { cwd: packageDirectory, stdio: "inherit" });
      break;
    case "bun":
      execFileSync("bun", ["run", "probe.mjs"], { cwd: packageDirectory, stdio: "inherit" });
      break;
    case "deno":
      execFileSync(
        "deno",
        [
          "run",
          "--node-modules-dir=manual",
          "--allow-ffi",
          "--allow-read",
          "--allow-sys",
          "probe.mjs",
        ],
        { cwd: packageDirectory, stdio: "inherit" },
      );
      break;
    default:
      throw new Error(`Unknown runtime: ${runtime}`);
  }
} finally {
  await rm(temporaryRoot, { recursive: true, force: true });
}
