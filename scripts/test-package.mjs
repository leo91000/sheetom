import { execFileSync } from "node:child_process";
import { mkdtemp, mkdir, readFile, rm, writeFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

const repositoryRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const temporaryRoot = await mkdtemp(path.join(os.tmpdir(), "sheetom-package-"));
const packageDirectory = path.join(temporaryRoot, "package");

try {
  await mkdir(packageDirectory);
  const packResult = JSON.parse(execFileSync(
    "npm",
    ["pack", "--json", "--pack-destination", temporaryRoot],
    { cwd: repositoryRoot, encoding: "utf8" },
  ));
  const filename = packResult[0]?.filename;
  if (!filename) throw new Error("npm pack did not produce a tarball");

  const tarball = path.join(temporaryRoot, filename);
  execFileSync("npm", ["init", "--yes"], { cwd: packageDirectory, stdio: "ignore" });
  execFileSync("npm", ["install", "--ignore-scripts", tarball], {
    cwd: packageDirectory,
    stdio: "inherit",
  });

  const esmProbe = `
    import { createRequire } from "node:module";
    import path from "node:path";
    import { CSSStyleRule, parseStyleSheet } from "sheetom";
    const require = createRequire(import.meta.url);
    const packageRoot = path.resolve(path.dirname(require.resolve("sheetom")), "..");
    const native = require(path.join(packageRoot, "native/index.cjs"));
    if (native.nativeEngineRevision() !== "lightningcss-1.33.0-c6a0c3ce-sheetom.8") {
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
  `;
  await writeFile(path.join(packageDirectory, "probe.mjs"), esmProbe);
  execFileSync(process.execPath, ["probe.mjs"], { cwd: packageDirectory, stdio: "inherit" });

  const cjsProbe = `
    const { CSSStyleRule, parseStyleSheet } = require("sheetom");
    const sheet = parseStyleSheet(".x { color: red; }");
    if (!(sheet.cssRules[0] instanceof CSSStyleRule)) throw new Error("CJS style rule missing");
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
