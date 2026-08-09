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
const shorthandGrammarContractsBytes = await readFile(
  path.join(compatibilityRoot, "shorthand-grammar-contracts.json"),
);
const shorthandGrammarObservationsBytes = await readFile(
  path.join(compatibilityRoot, "shorthand-grammar-observations.json"),
);
const shorthandGrammarContracts = JSON.parse(shorthandGrammarContractsBytes.toString("utf8"));
const shorthandGrammarObservations = JSON.parse(
  shorthandGrammarObservationsBytes.toString("utf8"),
);
const shorthandGrammarCases = shorthandGrammarContracts.profiles.flatMap(
  profile => profile.cases,
);
if (
  shorthandGrammarContracts.profiles.length !== 23 ||
  shorthandGrammarCases.length !== 92 ||
  shorthandGrammarObservations.cases.length !== shorthandGrammarCases.length
) {
  throw new Error("Shorthand Grammar Branch evidence is incomplete");
}
for (let index = 0; index < shorthandGrammarCases.length; index += 1) {
  const grammarCase = shorthandGrammarCases[index];
  const observation = shorthandGrammarObservations.cases[index];
  if (observation?.id !== grammarCase?.id || observation.accepted !== grammarCase.accepted) {
    throw new Error(`Shorthand Grammar Branch evidence drifted at ${grammarCase?.id}`);
  }
}
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
const operationReportArgument = process.argv.find(argument =>
  argument.startsWith("--operation-report="),
);
if (!operationReportArgument) {
  throw new Error("Compatibility recording requires --operation-report=path/to/report.json");
}
const operationReportPath = path.resolve(operationReportArgument.slice("--operation-report=".length));
const operationReportBytes = await readFile(operationReportPath);
const operationReport = JSON.parse(operationReportBytes.toString("utf8"));
if (operationReport.schemaVersion !== 2 || !Array.isArray(operationReport.adapters)) {
  throw new Error("Operation Fixture evidence has an unsupported shape");
}
const operationFixtureAdapters = operationReport.adapters;
const expectedOperationAdapters = ["sheetom", "chromium", "firefox", "webkit"];
for (const adapter of expectedOperationAdapters) {
  const matches = operationFixtureAdapters.filter(evidence => evidence.adapter === adapter);
  if (matches.length !== 1) {
    throw new Error(`Operation Fixture evidence must contain one ${adapter} adapter result`);
  }
  const [evidence] = matches;
  if (evidence.passed !== resolutions.length || evidence.total !== resolutions.length) {
    throw new Error(`${adapter} Operation Fixture evidence is incomplete`);
  }
  if (
    typeof evidence.version !== "string" ||
    !Array.isArray(evidence.observations) ||
    evidence.observations.length !== resolutions.length
  ) {
    throw new Error(`${adapter} Operation Fixture observations are incomplete`);
  }
  const observedIds = new Set(evidence.observations.map(observation => observation.fixtureId));
  if (
    observedIds.size !== resolutions.length ||
    resolutions.some(resolution => !observedIds.has(resolution.fixtureId))
  ) {
    throw new Error(`${adapter} Operation Fixture observation IDs are incomplete`);
  }
}
if (operationFixtureAdapters.length !== expectedOperationAdapters.length) {
  throw new Error("Operation Fixture evidence contains an unknown adapter result");
}

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
  schemaVersion: 4,
  packageVersion: packageManifest.version,
  baseline: {
    wptCommit: wptLock.commit,
    syntaxEngineSet: {
      lightningcss: packageLock.packages["node_modules/lightningcss"].version,
      cssTree: packageLock.packages["node_modules/css-tree"].version,
      cssstyle: packageLock.packages["node_modules/cssstyle"].version,
      cssTokenizer: packageLock.packages["node_modules/@csstools/css-tokenizer"].version,
    },
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
      total: resolutions.length,
      sha256: createHash("sha256").update(operationReportBytes).digest("hex"),
      adapters: operationFixtureAdapters
    },
    shorthandGrammar: {
      profiles: shorthandGrammarContracts.profiles.length,
      passed: shorthandGrammarCases.length,
      total: shorthandGrammarCases.length,
      contractsSha256: createHash("sha256")
        .update(shorthandGrammarContractsBytes)
        .digest("hex"),
      observationsSha256: createHash("sha256")
        .update(shorthandGrammarObservationsBytes)
        .digest("hex")
    },
    nativeWpt
  },
  resolutions
};

await writeFile(output, `${JSON.stringify(report, null, 2)}\n`, { flag: "wx" });
console.log(`Recorded Baseline Draft at ${path.relative(repositoryRoot, output)}.`);
