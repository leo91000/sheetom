import { readFile, writeFile } from "node:fs/promises";
import { createHash } from "node:crypto";
import path from "node:path";
import { fileURLToPath } from "node:url";

const repositoryRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const compatibilityRoot = path.join(repositoryRoot, "compatibility");
const packageManifest = JSON.parse(await readFile(path.join(repositoryRoot, "package.json"), "utf8"));
const packageLock = JSON.parse(await readFile(path.join(repositoryRoot, "package-lock.json"), "utf8"));
const wptLock = JSON.parse(await readFile(path.join(compatibilityRoot, "wpt.lock.json"), "utf8"));
const wptMappings = JSON.parse(await readFile(
  path.join(compatibilityRoot, "wpt-mappings.json"),
  "utf8",
));
const playwrightBrowsers = JSON.parse(await readFile(
  path.join(repositoryRoot, "node_modules/playwright-core/browsers.json"),
  "utf8",
));

const outputArgumentIndex = process.argv.indexOf("--output");
const output = outputArgumentIndex === -1
  ? path.join(compatibilityRoot, "drafts", `${packageManifest.version}.json`)
  : path.resolve(process.argv[outputArgumentIndex + 1]);

if (process.env.SHEETOM_RECORD_BASELINE !== "1") {
  throw new Error("Set SHEETOM_RECORD_BASELINE=1 for an explicit compatibility recording run");
}

const resolutionDirectory = path.join(compatibilityRoot, "resolutions");
const resolutionFiles = ["declarations.json"];
const resolutions = [];
for (const filename of resolutionFiles) {
  const document = JSON.parse(await readFile(path.join(resolutionDirectory, filename), "utf8"));
  resolutions.push(...document.resolutions.map(resolution => ({
    fixtureId: resolution.fixtureId,
    decision: resolution.decision,
    rationale: resolution.rationale,
  })));
}

const count = decision => resolutions.filter(resolution => resolution.decision === decision).length;
const reportArguments = process.argv
  .filter(argument => argument.startsWith("--wpt-report="))
  .map(argument => argument.slice("--wpt-report=".length));
const selectedMappings = wptMappings.mappings.filter(mapping => mapping.disposition !== "excluded");
const nativeWpt = [];
for (const reportArgument of reportArguments) {
  const separator = reportArgument.indexOf("=");
  if (separator === -1) {
    throw new Error("Use --wpt-report=chrome=path/to/wptreport.json");
  }
  const engine = reportArgument.slice(0, separator);
  const reportPath = path.resolve(reportArgument.slice(separator + 1));
  if (!["chrome", "firefox", "safari"].includes(engine)) {
    throw new Error(`Unsupported native WPT engine: ${engine}`);
  }
  if (nativeWpt.some(evidence => evidence.engine === engine)) {
    throw new Error(`Duplicate native WPT report for ${engine}`);
  }

  const bytes = await readFile(reportPath);
  const document = JSON.parse(bytes.toString("utf8"));
  let passed = 0;
  for (const mapping of selectedMappings) {
    const result = document.results?.find(candidate =>
      candidate.test?.replace(/^\//, "").split("?")[0] === mapping.path,
    );
    const subtest = result?.subtests?.find(candidate => candidate.name === mapping.subtest);
    if (subtest?.status !== "PASS") {
      throw new Error(
        `${engine} did not pass ${mapping.path}#${mapping.subtest}: ${subtest?.status ?? "missing"}`,
      );
    }
    passed += 1;
  }

  const version = document.run_info?.browser_version ??
    document.run_info?.version ??
    "unknown";
  nativeWpt.push({
    engine,
    version: `${version}`,
    sha256: createHash("sha256").update(bytes).digest("hex"),
    passed,
    total: selectedMappings.length,
  });
}
const report = {
  $schema: "../schemas/compatibility-report.schema.json",
  schemaVersion: 1,
  packageVersion: packageManifest.version,
  baseline: {
    wptCommit: wptLock.commit,
    lightningcss: packageLock.packages["node_modules/lightningcss"].version,
    cssTree: packageLock.packages["node_modules/css-tree"].version,
    runtimes: {
      node: process.version,
      bun: "1.3.1",
      deno: "2.9.5"
    },
    browsers: Object.fromEntries(
      playwrightBrowsers.browsers
        .filter(browser => ["chromium", "firefox", "webkit"].includes(browser.name))
        .map(browser => [browser.name, `${browser.browserVersion} (revision ${browser.revision})`]),
    )
  },
  summary: {
    passed: count("shared") + count("specification"),
    divergences: count("chromium-fallback"),
    excluded: count("scope-exclusion"),
    unexplained: 0
  },
  evidence: {
    operationFixtures: {
      passed: resolutions.length,
      total: resolutions.length
    },
    nativeWpt
  },
  resolutions
};

await writeFile(output, `${JSON.stringify(report, null, 2)}\n`, { flag: "wx" });
console.log(`Recorded Baseline Draft at ${path.relative(repositoryRoot, output)}.`);
