import { createHash } from "node:crypto";
import { execFileSync } from "node:child_process";
import { readFile, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

export const ENGINE_ABI_VERSION = 1;

const syntaxEnginePaths = [
  ".cargo/config.toml",
  "Cargo.lock",
  "Cargo.toml",
  "crates/sheetom-core",
  "vendor/README.md",
  "vendor/cssparser",
  "vendor/lightningcss",
];

export async function computeEngineAbiIdentity(repositoryRoot) {
  const packageManifest = JSON.parse(await readFile(
    path.join(repositoryRoot, "package.json"),
    "utf8",
  ));
  if (typeof packageManifest.version !== "string" || packageManifest.version === "") {
    throw new Error("package.json does not contain a SheetOM version");
  }

  const tracked = execFileSync("git", ["ls-files", "-z", "--", ...syntaxEnginePaths], {
    cwd: repositoryRoot,
  }).toString("utf8").split("\0").filter(Boolean).sort();
  if (tracked.length === 0) throw new Error("Syntax Engine Set source manifest is empty");

  const digest = createHash("sha256");
  for (const filename of tracked) {
    digest.update(filename);
    digest.update("\0");
    digest.update(await readFile(path.join(repositoryRoot, filename)));
    digest.update("\0");
  }

  return {
    abiVersion: ENGINE_ABI_VERSION,
    sheetomVersion: packageManifest.version,
    syntaxEngineSetSha256: digest.digest("hex"),
  };
}

export async function recordEngineAbiIdentity(repositoryRoot, { check = false } = {}) {
  const identity = await computeEngineAbiIdentity(repositoryRoot);
  const outputPath = path.join(repositoryRoot, "engine-abi.json");
  const expected = `${JSON.stringify(identity, null, 2)}\n`;
  if (!check) {
    await writeFile(outputPath, expected);
    return identity;
  }

  const actual = await readFile(outputPath, "utf8").catch(() => "");
  if (actual !== expected) {
    throw new Error("engine-abi.json is stale; run npm run record:engine-abi");
  }
  return identity;
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  const repositoryRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
  const check = process.argv.includes("--check");
  if (!check && !process.argv.includes("--record")) {
    throw new Error("Usage: engine-abi.ts --check|--record");
  }
  const identity = await recordEngineAbiIdentity(repositoryRoot, { check });
  console.log(JSON.stringify(identity));
}
