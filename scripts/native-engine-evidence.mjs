import { createHash } from "node:crypto";
import { execFileSync } from "node:child_process";
import { readFile } from "node:fs/promises";
import path from "node:path";

import { readNativeEngineRevision } from "./native-engine-revision.mjs";

const sourcePaths = [
  ".cargo/config.toml",
  "Cargo.lock",
  "Cargo.toml",
  "crates/sheetom-core",
  "crates/sheetom-native",
  "native",
  "vendor/README.md",
  "vendor/cssparser",
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

  return {
    revision: await readNativeEngineRevision(repositoryRoot),
    upstream: {
      repository: "https://github.com/parcel-bundler/lightningcss",
      version: "1.33.0",
      commit: "c6a0c3cebf3395635e61075d2c81a96a710d4910",
    },
    cssSyntax: {
      repository: "https://github.com/servo/rust-cssparser",
      version: "0.37.0",
      commit: "4c49486494fb24dc01390e3baca9698ef1744c71",
    },
    sourceManifestSha256: sourceManifest.digest("hex"),
    sourceFileCount: files.length,
  };
}
