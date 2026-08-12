import { createHash } from "node:crypto";
import { readFile } from "node:fs/promises";
import path from "node:path";

export const WASM_EVIDENCE_BROWSERS = Object.freeze(["chromium", "firefox", "webkit"]);
export const WASM_EVIDENCE_BUNDLERS = Object.freeze(["esbuild", "rollup", "vite", "webpack"]);

const CONTRACT_FILES = Object.freeze([
  "scripts/build-wasm-engine.mjs",
  "scripts/finalize-wasm-package.mjs",
  "scripts/parameterize-wasm-bindgen.mjs",
  "scripts/parameterize-wasm-facade.mjs",
  "scripts/test-wasm-backend.mjs",
  "scripts/test-wasm-browsers.mjs",
  "scripts/test-wasm-bundlers.mjs",
  "scripts/test-wasm-browser-performance.mjs",
  "scripts/test-wasm-memory-soak.mjs",
]);

export async function wasmBackendContractSha256(repositoryRoot) {
  const hash = createHash("sha256");
  for (const filename of CONTRACT_FILES) {
    hash.update(filename);
    hash.update("\0");
    hash.update(await readFile(path.join(repositoryRoot, filename)));
    hash.update("\0");
  }
  return hash.digest("hex");
}

export function validateWasmBackendEvidence(evidence) {
  if (evidence?.schemaVersion !== 1 || evidence.backend !== "wasm") {
    throw new Error("WASM backend evidence has an invalid identity");
  }
  if (!/^[0-9a-f]{64}$/u.test(evidence.contractSha256 ?? "")) {
    throw new Error("WASM backend evidence has no contract hash");
  }
  for (const hash of Object.values(evidence.reports ?? {})) {
    if (!/^[0-9a-f]{64}$/u.test(hash)) throw new Error("WASM report hash is invalid");
  }
  if (Object.keys(evidence.reports ?? {}).length !== 4) {
    throw new Error("WASM backend evidence must identify four executed reports");
  }

  const directBrowsers = evidence.browsers?.map(entry => entry.browser) ?? [];
  if (JSON.stringify(directBrowsers) !== JSON.stringify(WASM_EVIDENCE_BROWSERS)) {
    throw new Error(`Direct WASM evidence must cover ${WASM_EVIDENCE_BROWSERS.join(", ")}`);
  }
  if (evidence.browsers.some(entry =>
    !entry.mainThread || !entry.worker || !entry.streaming ||
    !entry.bufferedFallback || !entry.independentInstances || !entry.version
  )) {
    throw new Error("Direct WASM evidence is incomplete");
  }

  const observedBundlerCases = new Set(
    (evidence.bundlers ?? []).map(entry => `${entry.bundler}:${entry.browser}`),
  );
  if (observedBundlerCases.size !== WASM_EVIDENCE_BUNDLERS.length * WASM_EVIDENCE_BROWSERS.length) {
    throw new Error("WASM bundler evidence has an invalid case count");
  }
  for (const bundler of WASM_EVIDENCE_BUNDLERS) {
    for (const browser of WASM_EVIDENCE_BROWSERS) {
      if (!observedBundlerCases.has(`${bundler}:${browser}`)) {
        throw new Error(`Bundler evidence lacks ${bundler} in ${browser}`);
      }
    }
  }
  if (evidence.bundlers.some(entry => !entry.mainThread || !entry.worker || !entry.version)) {
    throw new Error("Bundler thread evidence is incomplete");
  }

  if (
    evidence.performance?.length !== WASM_EVIDENCE_BROWSERS.length ||
    evidence.performance.some(entry =>
      !entry.version || entry.stylesheetCount !== 21 || entry.ruleCount !== 11_000 ||
      entry.totalMilliseconds > 30_000 || entry.initializationMilliseconds > 10_000 ||
      entry.serializationMilliseconds > 10_000 || entry.secondSerializationMilliseconds > 5_000
    )
  ) {
    throw new Error("WASM Publisher performance evidence is incomplete");
  }
  if (
    evidence.memory?.cycles !== 36 ||
    evidence.memory.rssGrowth > 256 * 1024 * 1024 ||
    evidence.memory.secondCycleExternalGrowth > 32 * 1024 * 1024 ||
    evidence.memory.secondCycleHeapGrowth > 16 * 1024 * 1024
  ) {
    throw new Error("WASM memory-soak evidence exceeds its reviewed limits");
  }
}
