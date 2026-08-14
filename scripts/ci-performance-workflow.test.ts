import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

const workflow = await readFile(new URL("../.github/workflows/ci.yml", import.meta.url), "utf8");

function job(name) {
  const marker = `  ${name}:\n`;
  const start = workflow.indexOf(marker);
  assert.notEqual(start, -1, `missing ${name} job`);
  const next = workflow.slice(start + marker.length).search(/^  [a-z][a-z-]*:\n/mu);
  return next === -1
    ? workflow.slice(start)
    : workflow.slice(start, start + marker.length + next);
}

test("performance runtimes build in parallel and benchmark on one runner", () => {
  const candidate = job("performance-candidate");
  const baseline = job("performance-baseline");
  const comparison = job("performance");

  assert.match(candidate, /npm run native:build/u);
  assert.match(candidate, /sheetom-performance-candidate/u);
  assert.match(candidate, /packages\/native-linux-x64-gnu/u);
  assert.match(baseline, /npm run native:build/u);
  assert.match(baseline, /sheetom-performance-baseline/u);
  assert.match(
    comparison,
    /needs: \[changes, performance-candidate, performance-baseline\]/u,
  );
  assert.doesNotMatch(comparison, /npm run native:build/u);
  assert.match(comparison, /Link split native packages into each benchmark runtime/u);
  assert.match(comparison, /performance-candidate\/dist\/index\.js/u);
  assert.match(comparison, /performance-baseline\/dist\/index\.js/u);
});

test("the shared benchmark harness accepts an explicit WASM backend", async () => {
  const harness = await readFile(new URL("./benchmark.ts", import.meta.url), "utf8");
  assert.match(harness, /--wasm-binary/u);
  assert.match(harness, /createSheetOM/u);
  assert.match(harness, /applyMutations/u);
  assert.match(harness, /runLargeSerializationWorkload/u);
  assert.match(harness, /--large-serialization-only/u);
  assert.match(harness, /process\.resourceUsage\(\)\.maxRSS/u);
});
