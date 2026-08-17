import { execFileSync } from "node:child_process";
import { copyFile, mkdir, readFile, rm, stat, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { gzipSync } from "node:zlib";

const repositoryRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const cargoTargetDirectory = path.resolve(
  repositoryRoot,
  process.env.CARGO_TARGET_DIR ?? "target",
);
const generatedDirectory = path.join(repositoryRoot, "packages/wasm/generated");
const distributionDirectory = path.join(repositoryRoot, "packages/wasm/dist");
const wasmBindgenVersion = "0.2.127";
const wasmOptimizationProfile = "-O2";
const maximumRawBytes = 5_000_000;
const maximumGzipBytes = 1_400_000;
const { parameterizeWasmBindgenGlue } = await import("./parameterize-wasm-bindgen.ts");

const installedVersion = execFileSync("wasm-bindgen", ["--version"], {
  encoding: "utf8",
}).trim();
if (installedVersion !== `wasm-bindgen ${wasmBindgenVersion}`) {
  throw new Error(
    `Expected wasm-bindgen ${wasmBindgenVersion}; received ${installedVersion}`,
  );
}

await rm(generatedDirectory, { recursive: true, force: true });
await rm(distributionDirectory, { recursive: true, force: true });
await mkdir(generatedDirectory, { recursive: true });
await mkdir(distributionDirectory, { recursive: true });

execFileSync(
  "cargo",
  [
    "build",
    "--release",
    "--package",
    "sheetom-wasm",
    "--target",
    "wasm32-unknown-unknown",
  ],
  { cwd: repositoryRoot, stdio: "inherit" },
);
execFileSync(
  "wasm-bindgen",
  [
    "--target",
    "web",
    "--out-dir",
    generatedDirectory,
    "--out-name",
    "sheetom_wasm",
    path.join(
      cargoTargetDirectory,
      "wasm32-unknown-unknown/release/sheetom_wasm.wasm",
    ),
  ],
  { cwd: repositoryRoot, stdio: "inherit" },
);

const generatedGlue = path.join(generatedDirectory, "sheetom_wasm.js");
await writeFile(
  path.join(generatedDirectory, "sheetom_wasm_factory.js"),
  parameterizeWasmBindgenGlue(await readFile(generatedGlue, "utf8")),
);

const generatedWasm = path.join(generatedDirectory, "sheetom_wasm_bg.wasm");
const optimizedWasm = path.join(generatedDirectory, "sheetom_wasm_bg.optimized.wasm");
const unoptimizedBytes = (await stat(generatedWasm)).size;
execFileSync(
  path.join(repositoryRoot, "node_modules", ".bin", "wasm-opt"),
  [
    generatedWasm,
    wasmOptimizationProfile,
    "--enable-bulk-memory",
    "--enable-nontrapping-float-to-int",
    "--enable-sign-ext",
    "-o",
    optimizedWasm,
  ],
  { cwd: repositoryRoot, stdio: "inherit" },
);
const optimizedBytes = (await stat(optimizedWasm)).size;
if (optimizedBytes >= unoptimizedBytes * 0.95) {
  throw new Error(
    `wasm-opt did not reduce the engine by at least 5%: ${unoptimizedBytes} -> ${optimizedBytes}`,
  );
}
const gzipBytes = gzipSync(await readFile(optimizedWasm), { level: 9 }).byteLength;
if (optimizedBytes > maximumRawBytes || gzipBytes > maximumGzipBytes) {
  throw new Error(
    `WebAssembly engine exceeds its size budgets: raw ${optimizedBytes}/${maximumRawBytes}, ` +
      `gzip ${gzipBytes}/${maximumGzipBytes}`,
  );
}
await rm(generatedWasm);
await copyFile(optimizedWasm, generatedWasm);
await rm(optimizedWasm);
console.log(
  JSON.stringify(
    {
      wasmOptimizationProfile,
      unoptimizedBytes,
      optimizedBytes,
      gzipBytes,
      maximumRawBytes,
      maximumGzipBytes,
    },
    null,
    2,
  ),
);
