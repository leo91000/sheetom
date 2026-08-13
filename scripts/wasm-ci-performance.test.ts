import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

const packageManifest = JSON.parse(
  await readFile(new URL("../package.json", import.meta.url), "utf8"),
);
const buildScript = await readFile(
  new URL("./build-wasm-engine.ts", import.meta.url),
  "utf8",
);
const workflow = await readFile(
  new URL("../.github/workflows/ci.yml", import.meta.url),
  "utf8",
);

function job(name: string): string {
  const marker = `  ${name}:\n`;
  const start = workflow.indexOf(marker);
  assert.notEqual(start, -1, `missing ${name} job`);
  const remainder = workflow.slice(start + marker.length);
  const next = remainder.search(/^  [a-z][a-z-]*:\n/mu);
  return next === -1
    ? workflow.slice(start)
    : workflow.slice(start, start + marker.length + next);
}

test("the WASM build uses the measured size-specialized pipeline", () => {
  assert.match(buildScript, /const rustOptimizationProfile = "wasm-release"/u);
  assert.match(buildScript, /const wasmOptimizationProfile = "-O1"/u);
  assert.match(buildScript, /const maximumRawBytes = 5_000_000/u);
  assert.match(buildScript, /const maximumGzipBytes = 850_000/u);
  assert.match(buildScript, /unoptimizedBytes \* 0\.97/u);
  assert.match(buildScript, /wasmOptimizationProfile,/u);
});

test("WASM CI does not repeat host workspace tests", () => {
  assert.doesNotMatch(packageManifest.scripts["wasm:check"], /cargo test/u);
  assert.match(packageManifest.scripts["native:core-check"], /cargo test --workspace/u);
});

test("WASM CI caches the exact Playwright browser cohort", () => {
  const wasmQuality = job("wasm-quality");
  assert.match(wasmQuality, /path: ~\/\.cache\/ms-playwright/u);
  assert.match(
    wasmQuality,
    /key: playwright-wasm-\$\{\{ runner\.os \}\}-\$\{\{ hashFiles\('package-lock\.json'\) \}\}/u,
  );
});
