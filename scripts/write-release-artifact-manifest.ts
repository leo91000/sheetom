import { readFile } from "node:fs/promises";
import path from "node:path";

import { writeReleaseArtifactManifest } from "./release-artifact-set.ts";

const directory = path.resolve(process.argv[2] ?? "package-artifact");
const rootManifest = JSON.parse(await readFile("package.json", "utf8"));
const manifest = await writeReleaseArtifactManifest(directory, rootManifest);
console.log(
  `Recorded ${manifest.packages.length} release packages (${manifest.totalSize} bytes).`,
);
