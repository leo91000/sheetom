import { readFile, writeFile } from "node:fs/promises";
import { pathToFileURL } from "node:url";

const exportPattern = /^export (class|function) ([A-Za-z_$][\w$]*)/gmu;
const terminalExportPattern = /^export \{ initSync, __wbg_init as default \};\s*$/mu;

export function parameterizeWasmBindgenGlue(source) {
  if (!source.includes("let wasmModule, wasmInstance, wasm;")) {
    throw new Error("wasm-bindgen glue no longer owns the expected module state");
  }
  if (!source.includes("async function __wbg_init(module_or_path)")) {
    throw new Error("wasm-bindgen glue no longer exposes the expected initializer");
  }
  if (!terminalExportPattern.test(source)) {
    throw new Error("wasm-bindgen glue has an unexpected terminal export");
  }

  const exportedNames = [...source.matchAll(exportPattern)].map(match => match[2]);
  if (!exportedNames.includes("WasmDeclarationState") || !exportedNames.includes("engineAbiIdentity")) {
    throw new Error("wasm-bindgen glue lacks required SheetOM exports");
  }

  const implementation = source
    .replace(exportPattern, "$1 $2")
    .replace(terminalExportPattern, "")
    .split("\n")
    .map(line => `  ${line}`)
    .join("\n");

  return [
    "// Generated from pinned wasm-bindgen output; do not edit.",
    "export async function createWasmBindings(module_or_path) {",
    implementation,
    "  await __wbg_init({ module_or_path });",
    `  return Object.freeze({ ${exportedNames.join(", ")} });`,
    "}",
    "",
  ].join("\n");
}

async function main() {
  const [input, output] = process.argv.slice(2);
  if (!input || !output) {
    throw new Error("Usage: parameterize-wasm-bindgen.ts <input.js> <output.js>");
  }
  const source = await readFile(input, "utf8");
  await writeFile(output, parameterizeWasmBindgenGlue(source));
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  await main();
}
