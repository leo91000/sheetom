import { performance } from "node:perf_hooks";
import { writeFile } from "node:fs/promises";
import path from "node:path";
import { pathToFileURL } from "node:url";

const moduleIndex = process.argv.indexOf("--module");
if (moduleIndex !== -1 && !process.argv[moduleIndex + 1]) {
  throw new Error("--module requires a path");
}
const modulePath = moduleIndex === -1
  ? new URL("../dist/index.js", import.meta.url)
  : pathToFileURL(path.resolve(process.argv[moduleIndex + 1]));
const { CSSStyleRule, parseStyleSheet } = await import(modulePath.href);

const ruleCount = 10_000;
const mutationCount = 10_000;
const source = Array.from(
  { length: ruleCount },
  (_, index) => `.r${index}{width:${index % 100}px;color:rgb(${index % 255} 0 0)}`,
).join("\n");

const memoryBefore = process.memoryUsage().rss;
const parseStarted = performance.now();
const sheet = parseStyleSheet(source);
const parseMilliseconds = performance.now() - parseStarted;

const firstRule = sheet.cssRules[0];
if (!(firstRule instanceof CSSStyleRule)) throw new Error("Reference workload did not produce a style rule");

const mutationStarted = performance.now();
for (let index = 0; index < mutationCount; index += 1) {
  firstRule.style.setProperty("padding-left", `${index % 100}px`);
}
const mutationMilliseconds = performance.now() - mutationStarted;

const serializationStarted = performance.now();
const serialized = sheet.serialize();
const serializationMilliseconds = performance.now() - serializationStarted;
const memoryAfter = process.memoryUsage().rss;

const report = {
  runtime: process.version,
  workload: { sourceBytes: Buffer.byteLength(source), ruleCount, mutationCount },
  results: {
    parseMilliseconds,
    mutationMilliseconds,
    serializationMilliseconds,
    rssDeltaBytes: Math.max(0, memoryAfter - memoryBefore),
    outputBytes: Buffer.byteLength(serialized),
  },
};
const serializedReport = `${JSON.stringify(report, null, 2)}\n`;
const outputIndex = process.argv.indexOf("--output");
if (outputIndex !== -1) {
  const output = process.argv[outputIndex + 1];
  if (!output) throw new Error("--output requires a path");
  await writeFile(output, serializedReport);
}
console.log(serializedReport);
