import assert from "node:assert/strict";
import { readFile, writeFile } from "node:fs/promises";

import { createSheetOM } from "../packages/wasm/dist/index.js";

if (typeof globalThis.gc !== "function") {
  throw new Error("WASM memory soak requires node --expose-gc");
}

const bytes = await readFile(new URL(
  "../packages/wasm/dist/sheetom_wasm_bg.wasm",
  import.meta.url,
));
const module = await WebAssembly.compile(bytes);

async function collect() {
  for (let index = 0; index < 4; index += 1) {
    globalThis.gc();
    await new Promise(resolve => setImmediate(resolve));
  }
  return process.memoryUsage();
}

async function cycle(count) {
  for (let index = 0; index < count; index += 1) {
    const api = await createSheetOM(module);
    const sheet = new api.CSSStyleSheet();
    sheet.replaceSync(`.rule-${index} { background: image-set(url(a.png) 1x, url(b.png) 2x) red; }`);
    sheet.cssRules[0].style.setProperty("padding", "72px var(--space, var(--space,");
    assert.match(sheet.serialize(), /image-set\(/u);
  }
}

await cycle(4);
const baseline = await collect();
await cycle(16);
const first = await collect();
await cycle(16);
const second = await collect();

const rssGrowth = Math.max(0, second.rss - baseline.rss);
const secondCycleExternalGrowth = Math.max(0, second.external - first.external);
const secondCycleHeapGrowth = Math.max(0, second.heapUsed - first.heapUsed);
assert.ok(rssGrowth <= 256 * 1024 * 1024, `WASM RSS grew by ${rssGrowth} bytes`);
assert.ok(
  secondCycleExternalGrowth <= 32 * 1024 * 1024,
  `WASM external memory grew by ${secondCycleExternalGrowth} bytes in the second cycle`,
);
assert.ok(
  secondCycleHeapGrowth <= 16 * 1024 * 1024,
  `WASM heap grew by ${secondCycleHeapGrowth} bytes in the second cycle`,
);

const report = {
  schemaVersion: 1,
  cycles: 36,
  rssGrowth,
  secondCycleExternalGrowth,
  secondCycleHeapGrowth,
};
console.log(JSON.stringify(report, null, 2));
const outputIndex = process.argv.indexOf("--output");
if (outputIndex !== -1) {
  const output = process.argv[outputIndex + 1];
  if (!output) throw new Error("--output requires a path");
  await writeFile(output, `${JSON.stringify(report, null, 2)}\n`);
}
