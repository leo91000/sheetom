import { readFile } from "node:fs/promises";
import path from "node:path";

export async function readNativeEngineRevision(repositoryRoot) {
  const coreSource = await readFile(
    path.join(repositoryRoot, "crates/sheetom-core/src/lib.rs"),
    "utf8",
  );
  const revision = coreSource.match(
    /pub const ENGINE_REVISION: &str = "([^"]+)";/u,
  )?.[1];
  if (!revision) throw new Error("Native engine revision is missing from sheetom-core");
  return revision;
}
