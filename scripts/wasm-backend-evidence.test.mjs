import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

import {
  validateWasmBackendEvidence,
  WASM_EVIDENCE_BROWSERS,
  WASM_EVIDENCE_BUNDLERS,
} from "./wasm-backend-evidence.mjs";

function completeEvidence() {
  const browserObservation = browser => ({
    browser,
    version: "1",
    mainThread: true,
    worker: true,
    streaming: true,
    bufferedFallback: true,
    independentInstances: true,
  });
  return {
    schemaVersion: 1,
    backend: "wasm",
    contractSha256: "a".repeat(64),
    reports: Object.fromEntries(
      ["direct", "bundlers", "performance", "memory"]
        .map(name => [`${name}Sha256`, "b".repeat(64)]),
    ),
    browsers: WASM_EVIDENCE_BROWSERS.map(browserObservation),
    bundlers: WASM_EVIDENCE_BUNDLERS.flatMap(bundler =>
      WASM_EVIDENCE_BROWSERS.map(browser => ({
        bundler,
        browser,
        version: "1",
        mainThread: true,
        worker: true,
      })),
    ),
    performance: WASM_EVIDENCE_BROWSERS.map(browser => ({
      browser,
      version: "1",
      stylesheetCount: 21,
      ruleCount: 11_000,
      initializationMilliseconds: 10,
      totalMilliseconds: 1_000,
      serializationMilliseconds: 100,
      secondSerializationMilliseconds: 10,
    })),
    memory: {
      cycles: 36,
      rssGrowth: 1,
      secondCycleExternalGrowth: 1,
      secondCycleHeapGrowth: 1,
    },
  };
}

test("complete WASM backend evidence crosses one validation seam", () => {
  assert.doesNotThrow(() => validateWasmBackendEvidence(completeEvidence()));
});

test("WASM backend evidence fails closed on missing consumers and resource regressions", () => {
  const missingBundler = completeEvidence();
  missingBundler.bundlers.pop();
  assert.throws(() => validateWasmBackendEvidence(missingBundler), /case count/u);

  const leaking = completeEvidence();
  leaking.memory.secondCycleExternalGrowth = 64 * 1024 * 1024;
  assert.throws(() => validateWasmBackendEvidence(leaking), /memory-soak/u);
});

test("CI keeps the WASM tarball and evidence in separate artifacts", async () => {
  const workflow = await readFile(new URL("../.github/workflows/ci.yml", import.meta.url), "utf8");
  const packageUpload = workflow.match(
    /name: sheetom-wasm-package\n\s+path: ([^\n]+)\n\s+if-no-files-found: error/u,
  );
  const evidenceUpload = workflow.match(
    /name: wasm-backend-evidence\n\s+path: ([^\n]+)\n\s+if-no-files-found: error/u,
  );
  assert.equal(packageUpload?.[1]?.trim(), "wasm-package/*.tgz");
  assert.equal(
    evidenceUpload?.[1]?.trim(),
    "${{ runner.temp }}/wasm-backend-evidence.json",
  );
});
