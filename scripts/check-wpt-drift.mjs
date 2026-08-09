import { createHash } from "node:crypto";
import { readFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const repositoryRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const compatibilityRoot = path.join(repositoryRoot, "compatibility");
const lock = JSON.parse(await readFile(path.join(compatibilityRoot, "wpt.lock.json"), "utf8"));
const manifest = JSON.parse(await readFile(path.join(compatibilityRoot, "wpt-mappings.json"), "utf8"));
const mappingsByPath = new Map();

for (const mapping of manifest.mappings) {
  const mappings = mappingsByPath.get(mapping.path) ?? [];
  mappings.push(mapping);
  mappingsByPath.set(mapping.path, mappings);
}

function gitBlobSha(content) {
  const header = Buffer.from(`blob ${content.byteLength}\0`);
  return createHash("sha1").update(header).update(content).digest("hex");
}

for (const [sourcePath, mappings] of mappingsByPath) {
  const url = `https://raw.githubusercontent.com/web-platform-tests/wpt/${lock.commit}/${sourcePath}`;
  const response = await fetch(url);
  if (!response.ok) throw new Error(`Unable to read pinned WPT source ${sourcePath}: ${response.status}`);
  const content = Buffer.from(await response.arrayBuffer());
  const source = content.toString("utf8");
  const blobSha = gitBlobSha(content);

  for (const mapping of mappings) {
    if (mapping.blobSha !== blobSha) {
      throw new Error(`${mapping.id} is stale: expected blob ${mapping.blobSha}, received ${blobSha}`);
    }
    if (!source.includes(mapping.subtest)) {
      throw new Error(`${mapping.id} subtest title is absent from ${sourcePath}`);
    }
  }
}

console.log(`Verified ${manifest.mappings.length} WPT mappings at ${lock.commit}.`);
