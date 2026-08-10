import { createHash } from "node:crypto";
import { execFileSync } from "node:child_process";
import { readFile } from "node:fs/promises";
import path from "node:path";

const sourcePaths = [
  ".cargo/config.toml",
  "Cargo.lock",
  "Cargo.toml",
  "crates/sheetom-core",
  "crates/sheetom-native",
  "native",
  "vendor/lightningcss",
];

export async function nativeEngineEvidence(repositoryRoot) {
  const trackedFiles = execFileSync(
    "git",
    ["ls-files", "-z", "--", ...sourcePaths],
    { cwd: repositoryRoot },
  );
  const files = trackedFiles
    .toString("utf8")
    .split("\0")
    .filter(Boolean);
  if (files.length === 0) throw new Error("Native engine source manifest is empty");

  const sourceManifest = createHash("sha256");
  for (const filename of files) {
    const bytes = await readFile(path.join(repositoryRoot, filename));
    sourceManifest.update(filename);
    sourceManifest.update("\0");
    sourceManifest.update(bytes);
    sourceManifest.update("\0");
  }

  const coreSource = await readFile(
    path.join(repositoryRoot, "crates/sheetom-core/src/lib.rs"),
    "utf8",
  );
  const revision = coreSource.match(
    /pub const ENGINE_REVISION: &str = "([^"]+)";/u,
  )?.[1];
  if (!revision) throw new Error("Native engine revision is missing from sheetom-core");

  return {
    revision,
    upstream: {
      repository: "https://github.com/parcel-bundler/lightningcss",
      version: "1.33.0",
      commit: "c6a0c3cebf3395635e61075d2c81a96a710d4910",
    },
    sourceManifestSha256: sourceManifest.digest("hex"),
    sourceFileCount: files.length,
  };
}
