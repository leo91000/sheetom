import { readFile, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const cargoManifests = [
  "crates/sheetom-core/Cargo.toml",
  "crates/sheetom-native/Cargo.toml",
];

export function replaceCargoPackageVersion(source, version) {
  const packageSection = source.indexOf("[package]");
  if (packageSection === -1) throw new Error("Cargo manifest has no [package] section");
  const nextSection = source.indexOf("\n[", packageSection + "[package]".length);
  const sectionEnd = nextSection === -1 ? source.length : nextSection;
  const section = source.slice(packageSection, sectionEnd);
  const matches = [...section.matchAll(/^version = "[^"]+"$/gmu)];
  if (matches.length !== 1 || matches[0].index === undefined) {
    throw new Error("Cargo [package] section must contain exactly one version");
  }

  const versionStart = packageSection + matches[0].index;
  const versionEnd = versionStart + matches[0][0].length;
  return `${source.slice(0, versionStart)}version = "${version}"${source.slice(versionEnd)}`;
}

export function replaceCargoLockVersions(source, version) {
  const packageNames = new Set(["sheetom-core", "sheetom-native"]);
  const replaced = new Set();
  const updated = source.replace(
    /\[\[package\]\]\n[\s\S]*?(?=\n\[\[package\]\]|$)/gu,
    block => {
      const name = block.match(/^name = "([^"]+)"$/mu)?.[1];
      if (!name || !packageNames.has(name)) return block;
      const versions = [...block.matchAll(/^version = "[^"]+"$/gmu)];
      if (versions.length !== 1) {
        throw new Error(`Cargo.lock package ${name} must contain exactly one version`);
      }
      replaced.add(name);
      return block.replace(/^version = "[^"]+"$/mu, `version = "${version}"`);
    },
  );
  const missing = [...packageNames].filter(name => !replaced.has(name));
  if (missing.length > 0) {
    throw new Error(`Cargo.lock is missing workspace packages: ${missing.join(", ")}`);
  }
  return updated;
}

export async function syncCargoVersion(repositoryRoot) {
  const packageManifest = JSON.parse(
    await readFile(path.join(repositoryRoot, "package.json"), "utf8"),
  );
  const version = packageManifest.version;
  if (typeof version !== "string" || version === "") {
    throw new Error("package.json does not contain a version");
  }

  for (const filename of cargoManifests) {
    const manifestPath = path.join(repositoryRoot, filename);
    const source = await readFile(manifestPath, "utf8");
    await writeFile(manifestPath, replaceCargoPackageVersion(source, version));
  }
  const cargoLockPath = path.join(repositoryRoot, "Cargo.lock");
  const cargoLock = await readFile(cargoLockPath, "utf8");
  await writeFile(cargoLockPath, replaceCargoLockVersions(cargoLock, version));
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  await syncCargoVersion(path.resolve(path.dirname(fileURLToPath(import.meta.url)), ".."));
}
