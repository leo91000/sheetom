import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { mkdtemp, mkdir, rm, writeFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import test from "node:test";

import { verifyStablePromotion } from "./verify-stable-promotion.ts";

async function createPackage(
  root: string,
  version: string,
  filename: string,
  marker = "same runtime",
): Promise<void> {
  const source = path.join(root, "source", "package");
  await mkdir(source, { recursive: true });
  await writeFile(path.join(source, "package.json"), `${JSON.stringify({
    name: "sheetom",
    version,
  })}\n`);
  await writeFile(path.join(source, "index.js"), `export const marker = ${JSON.stringify(marker)};\n`);
  execFileSync("tar", ["-czf", path.join(root, filename), "-C", path.dirname(source), "package"]);
  await rm(path.join(root, "source"), { recursive: true });
}

async function createArtifactSet(root: string, version: string, marker?: string): Promise<void> {
  const filename = `sheetom-${version}.tgz`;
  await createPackage(root, version, filename, marker);
  await writeFile(path.join(root, "sheetom-release-manifest.json"), `${JSON.stringify({
    schemaVersion: 1,
    version,
    packages: [{ name: "sheetom", version, role: "root", filename }],
  })}\n`);
}

test("accepts artifacts that differ only by the stable version", async () => {
  const root = await mkdtemp(path.join(os.tmpdir(), "sheetom-stable-promotion-"));
  try {
    const reference = path.join(root, "reference");
    const candidate = path.join(root, "candidate");
    await Promise.all([mkdir(reference), mkdir(candidate)]);
    await createArtifactSet(reference, "0.1.0-rc.8");
    await createArtifactSet(candidate, "0.1.0");
    await verifyStablePromotion({ referenceDirectory: reference, candidateDirectory: candidate });
  } finally {
    await rm(root, { recursive: true });
  }
});

test("rejects runtime changes hidden in a stable promotion", async () => {
  const root = await mkdtemp(path.join(os.tmpdir(), "sheetom-stable-promotion-"));
  try {
    const reference = path.join(root, "reference");
    const candidate = path.join(root, "candidate");
    await Promise.all([mkdir(reference), mkdir(candidate)]);
    await createArtifactSet(reference, "0.1.0-rc.8");
    await createArtifactSet(candidate, "0.1.0", "changed runtime");
    await assert.rejects(
      verifyStablePromotion({ referenceDirectory: reference, candidateDirectory: candidate }),
      /changed package\/index\.js beyond stable-version metadata/u,
    );
  } finally {
    await rm(root, { recursive: true });
  }
});
