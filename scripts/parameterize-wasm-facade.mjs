import { readFile, writeFile } from "node:fs/promises";
import { pathToFileURL } from "node:url";

const bindingPattern = /^const engineBinding = globalThis\.__SHEETOM_WASM_ENGINE_BINDING__;$/mu;
const exportPattern = /^export \{ ([^}]+) \};\s*$/mu;

export function parameterizeWasmFacade(source) {
  if (!bindingPattern.test(source)) {
    throw new Error("WASM facade bundle lacks the build-only Engine Binding marker");
  }
  if (/^import\s/mu.test(source)) {
    throw new Error("WASM facade bundle contains an unexpected runtime import");
  }

  const exportMatch = source.match(exportPattern);
  if (!exportMatch?.[1]) throw new Error("WASM facade bundle has an unexpected export surface");
  const exportedNames = exportMatch[1].split(", ").filter(Boolean);
  if (!exportedNames.includes("CSSStyleSheet") || !exportedNames.includes("parseStyleSheet")) {
    throw new Error("WASM facade bundle lacks required SheetOM exports");
  }

  const implementation = source
    .replace(bindingPattern, "")
    .replace(exportPattern, "")
    .replace(/^\/\/# sourceMappingURL=.*$/gmu, "")
    .split("\n")
    .map(line => `  ${line}`)
    .join("\n");

  return [
    "// Generated from the shared SheetOM facade; do not edit.",
    "export function createSheetOMFacade(engineBinding) {",
    implementation,
    `  return Object.freeze({ ${exportedNames.join(", ")} });`,
    "}",
    "",
  ].join("\n");
}

async function main() {
  const [input, output] = process.argv.slice(2);
  if (!input || !output) {
    throw new Error("Usage: parameterize-wasm-facade.mjs <input.js> <output.js>");
  }
  await writeFile(output, parameterizeWasmFacade(await readFile(input, "utf8")));
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  await main();
}
