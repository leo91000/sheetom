import { createHash } from "node:crypto";
import { readFile, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

import {
  validateWasmBackendEvidence,
  wasmBackendContractSha256,
} from "./wasm-backend-evidence.mjs";

const repositoryRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");

function argument(name) {
  const index = process.argv.indexOf(`--${name}`);
  const value = index === -1 ? undefined : process.argv[index + 1];
  if (!value) throw new Error(`--${name} requires a path`);
  return path.resolve(value);
}

async function readReport(name) {
  const bytes = await readFile(argument(name));
  const report = JSON.parse(bytes.toString("utf8"));
  if (report.schemaVersion !== 1) throw new Error(`${name} evidence has an invalid schema`);
  return { report, sha256: createHash("sha256").update(bytes).digest("hex") };
}

const direct = await readReport("direct");
const bundlers = await readReport("bundlers");
const performance = await readReport("performance");
const memory = await readReport("memory");
const evidence = {
  schemaVersion: 1,
  backend: "wasm",
  browsers: direct.report.observations,
  bundlers: bundlers.report.observations,
  performance: performance.report.observations,
  memory: memory.report,
  reports: {
    directSha256: direct.sha256,
    bundlersSha256: bundlers.sha256,
    performanceSha256: performance.sha256,
    memorySha256: memory.sha256,
  },
  contractSha256: await wasmBackendContractSha256(repositoryRoot),
};
validateWasmBackendEvidence(evidence);
await writeFile(argument("output"), `${JSON.stringify(evidence, null, 2)}\n`);
console.log("Recorded complete WASM backend evidence.");
