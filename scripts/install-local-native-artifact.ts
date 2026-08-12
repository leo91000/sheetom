import { copyFile, mkdir, stat } from "node:fs/promises";
import { createRequire } from "node:module";
import path from "node:path";
import { fileURLToPath } from "node:url";

const repositoryRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const require = createRequire(import.meta.url);
const { resolveTarget, TARGET_BY_NAME } = require("../native/resolve-target.cjs");
const target = resolveTarget();
const metadata = target ? TARGET_BY_NAME.get(target) : null;
if (!metadata) throw new Error(`Unsupported local native target: ${process.platform}/${process.arch}`);

const source = path.join(repositoryRoot, "native", metadata.artifact);
const destinationDirectory = path.join(repositoryRoot, "packages", `native-${metadata.target}`);
const destination = path.join(destinationDirectory, metadata.artifact);
if ((await stat(source)).size < 1_000_000) {
  throw new Error(`Native artifact is unexpectedly small: ${source}`);
}
await mkdir(destinationDirectory, { recursive: true });
await copyFile(source, destination);
console.log(`Installed ${metadata.artifact} into ${metadata.packageName}.`);
