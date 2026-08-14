import { execFileSync } from "node:child_process";
import { performance } from "node:perf_hooks";
import { readFile, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const moduleIndex = process.argv.indexOf("--module");
if (moduleIndex !== -1 && !process.argv[moduleIndex + 1]) {
  throw new Error("--module requires a path");
}
const modulePath = moduleIndex === -1
  ? new URL("../dist/index.js", import.meta.url)
  : pathToFileURL(path.resolve(process.argv[moduleIndex + 1]));
const importedModule = await import(modulePath.href);
const wasmBinaryIndex = process.argv.indexOf("--wasm-binary");
let publicApi = importedModule;
if (wasmBinaryIndex !== -1) {
  const wasmBinaryPath = process.argv[wasmBinaryIndex + 1];
  if (!wasmBinaryPath) throw new Error("--wasm-binary requires a path");
  if (typeof importedModule.createSheetOM !== "function") {
    throw new Error("--wasm-binary requires a module exporting createSheetOM");
  }
  const bytes = await readFile(path.resolve(wasmBinaryPath));
  publicApi = await importedModule.createSheetOM(bytes.buffer.slice(
    bytes.byteOffset,
    bytes.byteOffset + bytes.byteLength,
  ));
}
const { CSSStyleRule, CSSStyleSheet, parseStyleSheet } = publicApi;

function median(values) {
  const sorted = [...values].sort((left, right) => left - right);
  return sorted[Math.floor(sorted.length / 2)];
}

function styleRulesIn(ruleList) {
  const styleRules = [];
  for (let index = 0; index < ruleList.length; index += 1) {
    const rule = ruleList[index];
    if (!rule) continue;
    if (rule instanceof CSSStyleRule) styleRules.push(rule);
    if ("cssRules" in rule) styleRules.push(...styleRulesIn(rule.cssRules));
  }
  return styleRules;
}

function referenceRule(selector) {
  return `${selector}{width:10px;height:20px;color:rgb(10 20 30);padding:1px 2px;margin:3px}`;
}

function publisherSource(prefix, ruleCount) {
  const directCount = Math.floor(ruleCount / 2);
  const direct = Array.from(
    { length: directCount },
    (_, index) => referenceRule(`.${prefix}-direct-${index}`),
  ).join("\n");
  const responsive = Array.from(
    { length: ruleCount - directCount },
    (_, index) => referenceRule(`.${prefix}-responsive-${index}:hover`),
  ).join("\n");
  return `@layer ${prefix}{${direct}@media (max-width:767px){${responsive}}}`;
}

function runStressWorkload() {
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
  if (!(firstRule instanceof CSSStyleRule)) {
    throw new Error("Stress workload did not produce a style rule");
  }

  const mutationStarted = performance.now();
  for (let index = 0; index < mutationCount; index += 1) {
    firstRule.style.setProperty("padding-left", `${index % 100}px`);
  }
  const mutationMilliseconds = performance.now() - mutationStarted;

  const serializationStarted = performance.now();
  const serialized = sheet.serialize();
  const serializationMilliseconds = performance.now() - serializationStarted;
  const memoryAfter = process.memoryUsage().rss;

  return {
    workload: { sourceBytes: Buffer.byteLength(source), ruleCount, mutationCount },
    results: {
      parseMilliseconds,
      mutationMilliseconds,
      serializationMilliseconds,
      rssDeltaBytes: Math.max(0, memoryAfter - memoryBefore),
      outputBytes: Buffer.byteLength(serialized),
    },
  };
}

function runPublisherSample(sharedSource, pageSources) {
  const memoryBefore = process.memoryUsage().rss;
  const totalStarted = performance.now();
  const parseStarted = performance.now();
  const sheets = [
    parseStyleSheet(sharedSource, { diagnostics: true }),
    ...pageSources.map(source => parseStyleSheet(source, { diagnostics: true })),
  ];
  const parseMilliseconds = performance.now() - parseStarted;

  let mutationCount = 0;
  let acceptedMutationCount = 0;
  let rejectedMutationCount = 0;
  const mutationStarted = performance.now();
  for (const sheet of sheets) {
    const rules = styleRulesIn(sheet.cssRules);
    for (let index = 0; index < rules.length; index += 1) {
      const rule = rules[index];
      if (!rule) continue;
      const operations = [
        { kind: "set", property: "width", value: `${index % 200}px` },
        { kind: "set", property: "height", value: `${(index + 20) % 200}px` },
        { kind: "set", property: "color", value: `rgb(${index % 255} 20 30)` },
        { kind: "set", property: "padding-left", value: `${index % 20}px` },
        { kind: "set", property: "--publisher-token", value: `${index}` },
        { kind: "set", property: "width", value: "20px; color: red" },
        { kind: "remove", property: "margin-left" },
      ];
      if (typeof rule.style.applyMutations === "function") {
        const results = rule.style.applyMutations(operations);
        for (const result of results) {
          if (result.kind === "remove" || result.accepted) acceptedMutationCount += 1;
          else rejectedMutationCount += 1;
        }
        sheet.takeDiagnostics();
      } else {
        for (const operation of operations) {
          sheet.takeDiagnostics();
          if (operation.kind === "remove") {
            rule.style.removeProperty(operation.property);
            acceptedMutationCount += 1;
          } else {
            rule.style.setProperty(operation.property, operation.value);
            const diagnostics = sheet.takeDiagnostics();
            if (diagnostics.length === 0) acceptedMutationCount += 1;
            else rejectedMutationCount += 1;
          }
        }
      }
      mutationCount += operations.length;
    }
    const insertedIndex = sheet.insertRule(".publisher-temporary { opacity: 0.5; }");
    sheet.deleteRule(insertedIndex);
    mutationCount += 2;
  }
  const mutationMilliseconds = performance.now() - mutationStarted;

  const serializationStarted = performance.now();
  const serialized = sheets.map(sheet => sheet.serialize());
  const serializationMilliseconds = performance.now() - serializationStarted;
  const secondSerializationStarted = performance.now();
  const secondSerialized = sheets.map(sheet => sheet.serialize());
  const secondSerializationMilliseconds = performance.now() - secondSerializationStarted;
  if (serialized.some((value, index) => value !== secondSerialized[index])) {
    throw new Error("Publisher workload serialization is not idempotent");
  }

  return {
    results: {
      totalMilliseconds: performance.now() - totalStarted,
      parseMilliseconds,
      mutationMilliseconds,
      serializationMilliseconds,
      secondSerializationMilliseconds,
      rssDeltaBytes: Math.max(0, process.memoryUsage().rss - memoryBefore),
      outputBytes: serialized.reduce((total, value) => total + Buffer.byteLength(value), 0),
    },
    mutationCount,
    acceptedMutationCount,
    rejectedMutationCount,
  };
}

function runPublisherWorkload() {
  const pageSheetCount = 20;
  const pageRuleCount = 500;
  const sharedRuleCount = 1_000;
  const sampleCount = 3;
  const sharedSource = publisherSource("shared", sharedRuleCount);
  const pageSources = Array.from(
    { length: pageSheetCount },
    (_, index) => publisherSource(`page-${index}`, pageRuleCount),
  );

  runPublisherSample(sharedSource, pageSources);
  const samples = Array.from(
    { length: sampleCount },
    () => runPublisherSample(sharedSource, pageSources),
  );
  const metricNames = [
    "totalMilliseconds",
    "parseMilliseconds",
    "mutationMilliseconds",
    "serializationMilliseconds",
    "secondSerializationMilliseconds",
    "outputBytes",
  ];
  const results = Object.fromEntries(metricNames.map(metric => [
    metric,
    median(samples.map(sample => sample.results[metric])),
  ]));
  results.rssDeltaBytes = Math.max(
    ...samples.map(sample => sample.results.rssDeltaBytes),
  );

  return {
    workload: {
      declarationCount: (sharedRuleCount + pageSheetCount * pageRuleCount) * 5,
      mutationCount: samples[0].mutationCount,
      acceptedMutationCount: samples[0].acceptedMutationCount,
      rejectedMutationCount: samples[0].rejectedMutationCount,
      pageRuleCount,
      pageSheetCount,
      sampleCount,
      sharedRuleCount,
      sourceBytes: Buffer.byteLength(sharedSource) +
        pageSources.reduce((total, source) => total + Buffer.byteLength(source), 0),
      stylesheetCount: pageSheetCount + 1,
      warmupCount: 1,
    },
    results,
  };
}

function runLargeSerializationWorkload() {
  const ruleCount = 12_000;
  const payload = `"${"x".repeat(2_040)}"`;
  const sheet = new CSSStyleSheet();
  const operations = [{ kind: "set", property: "--payload", value: payload }];
  for (let index = 0; index < ruleCount; index += 1) {
    const inserted = sheet.insertRule(`.large-${index} {}`, sheet.cssRules.length);
    const rule = sheet.cssRules[inserted];
    if (!(rule instanceof CSSStyleRule)) {
      throw new Error("Large serialization workload did not produce a style rule");
    }
    if (typeof rule.style.applyMutations === "function") {
      rule.style.applyMutations(operations);
    } else {
      rule.style.setProperty("--payload", payload);
    }
  }

  const rssBeforeSerialization = process.memoryUsage().rss;
  const peakRssBeforeSerialization = process.resourceUsage().maxRSS * 1024;
  const serializationStarted = performance.now();
  const serialized = sheet.serialize();
  const serializationMilliseconds = performance.now() - serializationStarted;
  const rssAfterSerialization = process.memoryUsage().rss;
  const secondSerializationStarted = performance.now();
  const secondSerialized = sheet.serialize();
  const secondSerializationMilliseconds = performance.now() - secondSerializationStarted;
  if (serialized !== secondSerialized) {
    throw new Error("Large serialization workload is not idempotent");
  }

  return {
    workload: {
      outputBytes: Buffer.byteLength(serialized),
      ruleCount,
    },
    results: {
      serializationMilliseconds,
      secondSerializationMilliseconds,
      rssGrowthBytes: Math.max(0, rssAfterSerialization - rssBeforeSerialization),
      peakRssBytes: process.resourceUsage().maxRSS * 1024,
      peakRssGrowthBytes: Math.max(
        0,
        process.resourceUsage().maxRSS * 1024 - peakRssBeforeSerialization,
      ),
    },
  };
}

function runLargeSerializationInSubprocess() {
  const childArguments = [fileURLToPath(import.meta.url)];
  for (let index = 2; index < process.argv.length; index += 1) {
    const argument = process.argv[index];
    if (argument === "--output") {
      index += 1;
      continue;
    }
    if (argument !== "--large-serialization-only") childArguments.push(argument);
  }
  childArguments.push("--large-serialization-only");
  return JSON.parse(execFileSync(process.execPath, childArguments, {
    encoding: "utf8",
    maxBuffer: 1024 * 1024,
  }));
}

if (process.argv.includes("--large-serialization-only")) {
  console.log(JSON.stringify(runLargeSerializationWorkload()));
} else {
  const stress = runStressWorkload();
  const publisher = runPublisherWorkload();
  const report = {
    runtime: process.version,
    workload: stress.workload,
    results: stress.results,
    publisher,
    largeSerialization: runLargeSerializationInSubprocess(),
  };
  const serializedReport = `${JSON.stringify(report, null, 2)}\n`;
  const outputIndex = process.argv.indexOf("--output");
  if (outputIndex !== -1) {
    const output = process.argv[outputIndex + 1];
    if (!output) throw new Error("--output requires a path");
    await writeFile(output, serializedReport);
  }
  console.log(serializedReport);
}
