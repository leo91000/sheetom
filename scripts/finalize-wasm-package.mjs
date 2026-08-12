import { copyFile, readFile, stat } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const repositoryRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const generatedDirectory = path.join(repositoryRoot, "packages/wasm/generated");
const distributionDirectory = path.join(repositoryRoot, "packages/wasm/dist");

for (const filename of ["sheetom_wasm.js", "sheetom_wasm_bg.wasm"]) {
  await copyFile(
    path.join(generatedDirectory, filename),
    path.join(distributionDirectory, filename),
  );
}

const glue = await readFile(path.join(distributionDirectory, "sheetom_wasm.js"), "utf8");
if (!glue.includes("WebAssembly.instantiateStreaming")) {
  throw new Error("wasm-bindgen glue no longer provides streaming instantiation");
}
const javascript = await Promise.all(
  ["binding-registry.js", "facade.js", "index.js", "sheetom_wasm.js"]
    .map(filename => readFile(path.join(distributionDirectory, filename), "utf8")),
);
const runtimeSource = javascript.join("\n");
for (const forbidden of [
  "@sheetom/native-",
  "sheetom-native.",
  "node:module",
  "node:fs",
]) {
  if (runtimeSource.includes(forbidden)) {
    throw new Error(`WebAssembly package contains forbidden native runtime text: ${forbidden}`);
  }
}
if (/\beval\s*\(|\bnew\s+Function\s*\(/u.test(runtimeSource)) {
  throw new Error("WebAssembly package requires unsafe JavaScript evaluation");
}
const wasmBytes = (await stat(path.join(distributionDirectory, "sheetom_wasm_bg.wasm"))).size;
if (wasmBytes < 1_000_000) throw new Error(`WebAssembly engine is unexpectedly small: ${wasmBytes}`);

console.log(JSON.stringify({ wasmBytes }, null, 2));
