import { readFile } from "node:fs/promises";
import path from "node:path";
import { pathToFileURL } from "node:url";

/** Explicit installed prefix; never silently fall back to workspace builds. */
export function backendEntry(backend: "native" | "wasm", packageRoot = process.env.SHEETOM_PACKAGE_ROOT): URL {
  if (backend !== "native" && backend !== "wasm") throw new Error("Unknown validation backend");
  if (packageRoot !== undefined) {
    if (!packageRoot.trim()) throw new Error("SHEETOM_PACKAGE_ROOT must be a nonempty installation prefix");
    return pathToFileURL(path.resolve(packageRoot, "node_modules", backend === "native" ? "sheetom" : "@sheetom/wasm", "dist/index.js"));
  }
  return new URL(backend === "native" ? "../dist/index.js" : "../packages/wasm/dist/index.js", import.meta.url);
}

export async function loadValidationBackend(backend: "native" | "wasm") {
  const entry = backendEntry(backend);
  const api = await import(entry.href);
  if (backend === "native") return api;
  const bytes = await readFile(new URL("sheetom_wasm_bg.wasm", entry));
  return api.createSheetOM(new Uint8Array(bytes).buffer);
}
