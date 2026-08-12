import { isDeepStrictEqual } from "node:util";
import { execFileSync } from "node:child_process";
import { mkdtemp, readFile, readdir, rm, writeFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

import { chromium, firefox, webkit } from "playwright";

const repositoryRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
if (process.env.SHEETOM_RECORD_BASELINE !== "1") {
  throw new Error("Set SHEETOM_RECORD_BASELINE=1 for an explicit compatibility recording run");
}

const outputArgumentIndex = process.argv.indexOf("--output");
const output = outputArgumentIndex === -1
  ? path.join(repositoryRoot, "compatibility/drafts/operation-fixtures.json")
  : path.resolve(process.argv[outputArgumentIndex + 1]);
const resolutionDocument = JSON.parse(await readFile(
  path.join(repositoryRoot, "compatibility/resolutions/declarations.json"),
  "utf8",
));
const resolutions = new Map(
  resolutionDocument.resolutions.map(resolution => [resolution.fixtureId, resolution]),
);

async function fixtureFiles(directory) {
  const files = [];
  for (const entry of await readdir(directory, { withFileTypes: true })) {
    const entryPath = path.join(directory, entry.name);
    if (entry.isDirectory()) {
      files.push(...await fixtureFiles(entryPath));
      continue;
    }
    if (entry.name.endsWith(".json")) files.push(entryPath);
  }
  return files;
}

const fixtures = await Promise.all(
  (await fixtureFiles(path.join(repositoryRoot, "compatibility/fixtures")))
    .sort()
    .map(async filename => JSON.parse(await readFile(filename, "utf8"))),
);
const fixtureIds = new Set(fixtures.map(fixture => fixture.id));
if (
  fixtureIds.size !== resolutions.size ||
  [...resolutions.keys()].some(fixtureId => !fixtureIds.has(fixtureId))
) {
  throw new Error("Operation Fixture and Compatibility Resolution IDs differ");
}

const reportDirectory = await mkdtemp(path.join(os.tmpdir(), "sheetom-operation-evidence-"));
const vitest = path.join(repositoryRoot, "node_modules/vitest/vitest.mjs");

async function recordSheetOM() {
  const reportPath = path.join(reportDirectory, "sheetom-vitest.json");
  const observationsPath = path.join(reportDirectory, "sheetom-observations.json");
  execFileSync(
    process.execPath,
    [
      vitest,
      "run",
      "--project",
      "unit",
      "tests/conformance-fixtures.test.ts",
      "--reporter=json",
      `--outputFile=${reportPath}`,
    ],
    {
      cwd: repositoryRoot,
      env: {
        ...process.env,
        SHEETOM_OPERATION_OBSERVATIONS_PATH: observationsPath,
      },
      stdio: "inherit",
    },
  );
  const report = JSON.parse(await readFile(reportPath, "utf8"));
  if (
    report.numTotalTests !== fixtures.length ||
    report.numPassedTests !== fixtures.length ||
    report.numFailedTests !== 0 ||
    report.numPendingTests !== 0
  ) {
    throw new Error(`SheetOM executed ${report.numTotalTests ?? 0}/${fixtures.length} fixtures`);
  }
  const observations = JSON.parse(await readFile(observationsPath, "utf8"));
  return {
    adapter: "sheetom",
    version: JSON.parse(await readFile(path.join(repositoryRoot, "package.json"), "utf8")).version,
    passed: fixtures.length,
    total: fixtures.length,
    observations,
  };
}

async function executeNativeFixture(page, fixture) {
  return page.evaluate(async currentFixture => {
    function isBoundaryTag(value) {
      return typeof value === "object" && value !== null && "$type" in value;
    }
    function decode(value) {
      if (!isBoundaryTag(value)) return value;
      switch (value.$type) {
        case "undefined": return undefined;
        case "nan": return Number.NaN;
        case "positive-infinity": return Number.POSITIVE_INFINITY;
        case "negative-infinity": return Number.NEGATIVE_INFINITY;
        case "bigint": return BigInt(value.value ?? "0");
        case "symbol": return Symbol(value.value);
        case "throwing-string-coercion": return {
          toString() {
            throw new Error(value.value ?? "string coercion failed");
          },
        };
        default: throw new Error(`Unknown Boundary Value: ${value.$type}`);
      }
    }
    function encode(value) {
      if (value === undefined) return { $type: "undefined" };
      if (typeof value === "number" && Number.isNaN(value)) return { $type: "nan" };
      if (value === Number.POSITIVE_INFINITY) return { $type: "positive-infinity" };
      if (value === Number.NEGATIVE_INFINITY) return { $type: "negative-infinity" };
      if (typeof value === "bigint") return { $type: "bigint", value: value.toString() };
      if (typeof value === "symbol") return { $type: "symbol", value: value.description };
      return value;
    }
    function encodeHandle(value, handles) {
      if (value === null) return null;
      for (const [handle, candidate] of handles) {
        if (candidate === value) return handle;
      }
      return "$untracked";
    }
    function observeTarget(observation, target, requested, handles) {
      if (typeof target !== "object" || target === null) return;
      if (requested.includes("cssText")) observation.cssText = target.cssText;
      if (requested.includes("length")) observation.length = target.length;
      if (requested.includes("items")) {
        const length = typeof target.length === "number" ? target.length : 0;
        observation.items = typeof target.item === "function"
          ? Array.from({ length }, (_, index) => target.item(index))
          : [];
      }
      if (requested.includes("parentRule")) {
        observation.parentRule = encodeHandle(target.parentRule, handles);
      }
      if (requested.includes("parentStyleSheet")) {
        observation.parentStyleSheet = encodeHandle(target.parentStyleSheet, handles);
      }
    }
    function invoke(operation, target, args) {
      switch (operation.op) {
        case "constructStyleSheet": return new CSSStyleSheet();
        case "constructStyleRule": {
          const sheet = new CSSStyleSheet();
          sheet.insertRule(`${args[0]} {}`);
          return sheet.cssRules[0];
        }
        case "getStyle": return target.style;
        case "replaceSync": return Reflect.apply(target.replaceSync, target, args);
        case "getRule": return target.cssRules[args[0]];
        case "insertRule": return Reflect.apply(target.insertRule, target, args);
        case "deleteRule": return Reflect.apply(target.deleteRule, target, args);
        case "identity": return target;
        case "setProperty": return Reflect.apply(target.setProperty, target, args);
        case "removeProperty": return Reflect.apply(target.removeProperty, target, args);
        case "getPropertyValue": return Reflect.apply(target.getPropertyValue, target, args);
        case "getPropertyPriority": return Reflect.apply(target.getPropertyPriority, target, args);
        case "setCssText":
          target.cssText = args[0];
          return undefined;
        default: throw new Error(`Unsupported browser fixture operation: ${operation.op}`);
      }
    }

    const handles = new Map([["$root", null]]);
    const observations = [];
    for (const operation of currentFixture.operations) {
      if (!handles.has(operation.target)) {
        throw new Error(`Unknown fixture handle: ${operation.target}`);
      }
      const target = handles.get(operation.target);
      const args = operation.args.map(decode);
      const observation = {};
      let result;
      try {
        result = await invoke(operation, target, args);
        observation.exception = null;
      } catch (error) {
        observation.exception = {
          name: error instanceof Error ? error.name : "UnknownError",
        };
      }
      if (operation.handle && observation.exception === null) {
        handles.set(operation.handle, result);
      }
      const requested = operation.observe ?? [];
      if (requested.includes("return") && observation.exception === null) {
        observation.return = encode(result);
      }
      if (!requested.includes("exception")) delete observation.exception;
      observeTarget(observation, target, requested, handles);
      observations.push(observation);
    }
    return observations;
  }, fixture);
}

async function recordBrowser(adapter, browserType) {
  const browser = await browserType.launch({ headless: true });
  try {
    const page = await browser.newPage();
    const observations = [];
    for (const fixture of fixtures) {
      const operations = await executeNativeFixture(page, fixture);
      const resolution = resolutions.get(fixture.id);
      if (!resolution) throw new Error(`Missing resolution for ${fixture.id}`);
      if (
        (resolution.decision !== "chromium-fallback" || adapter === "chromium") &&
        !isDeepStrictEqual(operations, resolution.expected)
      ) {
        throw new Error(`${adapter} observation differs for ${fixture.id}`);
      }
      observations.push({ fixtureId: fixture.id, operations });
    }
    return {
      adapter,
      version: browser.version(),
      passed: fixtures.length,
      total: fixtures.length,
      observations,
    };
  } finally {
    await browser.close();
  }
}

try {
  const adapters = [
    await recordSheetOM(),
    await recordBrowser("chromium", chromium),
    await recordBrowser("firefox", firefox),
    await recordBrowser("webkit", webkit),
  ];
  await writeFile(output, `${JSON.stringify({ schemaVersion: 2, adapters }, null, 2)}\n`, {
    flag: "wx",
  });
  console.log(`Recorded Operation Fixture evidence at ${path.relative(repositoryRoot, output)}.`);
} finally {
  await rm(reportDirectory, { recursive: true, force: true });
}
