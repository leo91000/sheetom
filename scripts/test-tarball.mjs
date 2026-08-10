import { execFileSync } from "node:child_process";
import { mkdtemp, mkdir, readdir, rm, writeFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";

const [artifactPath, runtime = "node"] = process.argv.slice(2);
if (!artifactPath) throw new Error("Usage: test-tarball.mjs <tarball-or-directory> [node|bun|deno]");

const artifact = path.resolve(artifactPath);
const tarball = artifact.endsWith(".tgz")
  ? artifact
  : await resolveSingleTarball(artifact);
const temporaryRoot = await mkdtemp(path.join(os.tmpdir(), "sheetom-consumer-"));
const packageDirectory = path.join(temporaryRoot, "consumer");

function runNpm(arguments_, options) {
  if (process.platform !== "win32") {
    execFileSync("npm", arguments_, options);
    return;
  }

  execFileSync(process.env.ComSpec ?? "cmd.exe", ["/d", "/s", "/c", "npm", ...arguments_], options);
}

async function resolveSingleTarball(directory) {
  const entries = (await readdir(directory))
    .filter(entry => entry.endsWith(".tgz"));
  if (entries.length !== 1) {
    throw new Error(`Expected one tarball in ${directory}, found ${entries.length}`);
  }
  return path.join(directory, entries[0]);
}

try {
  await mkdir(packageDirectory);
  runNpm(["init", "--yes"], { cwd: packageDirectory, stdio: "ignore" });
  runNpm(["install", "--ignore-scripts", tarball], {
    cwd: packageDirectory,
    stdio: "inherit",
  });

  const esmProbe = `
    import { createRequire } from "node:module";
    import path from "node:path";
    import { CSSStyleRule, CSSMediaRule, parseStyleSheet } from "sheetom";
    const require = createRequire(import.meta.url);
    const packageRoot = path.resolve(path.dirname(require.resolve("sheetom")), "..");
    const native = require(path.join(packageRoot, "native/index.cjs"));
    if (native.nativeEngineRevision() !== "lightningcss-1.33.0-c6a0c3ce-sheetom.2") {
      throw new Error("native engine revision mismatch");
    }
    const sheet = parseStyleSheet("@media screen { .x { width: 1px; } }");
    const media = sheet.cssRules[0];
    if (!(media instanceof CSSMediaRule)) throw new Error("specialized media rule missing");
    const rule = media.cssRules[0];
    if (!(rule instanceof CSSStyleRule)) throw new Error("style rule missing");
    rule.style.setProperty("padding", "2px");
    if (!sheet.serialize().includes("padding: 2px")) throw new Error("mutation failed");
  `;
  await writeFile(path.join(packageDirectory, "probe.mjs"), esmProbe);

  switch (runtime) {
    case "node":
      execFileSync(process.execPath, ["probe.mjs"], { cwd: packageDirectory, stdio: "inherit" });
      await writeFile(
        path.join(packageDirectory, "probe.cjs"),
        'const sheetom = require("sheetom"); if (!sheetom.CSSStyleSheet) throw new Error("CJS export missing");',
      );
      execFileSync(process.execPath, ["probe.cjs"], { cwd: packageDirectory, stdio: "inherit" });
      break;
    case "bun":
      execFileSync("bun", ["run", "probe.mjs"], { cwd: packageDirectory, stdio: "inherit" });
      break;
    case "deno":
      execFileSync(
        "deno",
        ["run", "--node-modules-dir=manual", "--allow-ffi", "--allow-sys", "probe.mjs"],
        { cwd: packageDirectory, stdio: "inherit" },
      );
      break;
    default:
      throw new Error(`Unknown runtime: ${runtime}`);
  }
} finally {
  await rm(temporaryRoot, { recursive: true, force: true });
}
