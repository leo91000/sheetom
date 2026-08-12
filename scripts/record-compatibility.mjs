import { readFile, writeFile } from "node:fs/promises";
import { createHash } from "node:crypto";
import path from "node:path";
import { fileURLToPath } from "node:url";

import { nativeEngineEvidence } from "./native-engine-evidence.mjs";

const repositoryRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const compatibilityRoot = path.join(repositoryRoot, "compatibility");
const packageManifest = JSON.parse(await readFile(path.join(repositoryRoot, "package.json"), "utf8"));
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
const shorthandCapabilitiesBytes = await readFile(
  path.join(compatibilityRoot, "shorthand-capabilities.json"),
);
const nativeGrammarInventoryBytes = await readFile(
  path.join(compatibilityRoot, "native-grammar-inventory.json"),
);
const valueCapabilitiesBytes = await readFile(
  path.join(compatibilityRoot, "value-capabilities.json"),
);
const numberResultMathCapabilitiesBytes = await readFile(
  path.join(compatibilityRoot, "number-result-math-capabilities.json"),
);
const numericPropertyContractsBytes = await readFile(
  path.join(compatibilityRoot, "numeric-property-contracts.json"),
);
const propertyValueObservationsBytes = await readFile(
  path.join(compatibilityRoot, "property-value-observations.json"),
);
const propertyValueProbesBytes = await readFile(
  path.join(compatibilityRoot, "property-value-probes.json"),
);
const relativeColorCapabilitiesBytes = await readFile(
  path.join(compatibilityRoot, "relative-color-capabilities.json"),
);
const geometricContractsBytes = await readFile(
  path.join(compatibilityRoot, "browser-geometric-contracts.json"),
);
const geometricGeneratorBytes = await readFile(
  path.join(repositoryRoot, "scripts/browser-geometric-differential.mjs"),
);
const shorthandGrammarContracts = JSON.parse(shorthandGrammarContractsBytes.toString("utf8"));
const shorthandGrammarObservations = JSON.parse(
  shorthandGrammarObservationsBytes.toString("utf8"),
);
const shorthandGrammarCases = shorthandGrammarContracts.profiles.flatMap(
  profile => profile.cases,
);
const shorthandCapabilities = JSON.parse(shorthandCapabilitiesBytes.toString("utf8"));
const nativeGrammarInventory = JSON.parse(nativeGrammarInventoryBytes.toString("utf8"));
const valueCapabilities = JSON.parse(valueCapabilitiesBytes.toString("utf8"));
const numberResultMathCapabilities = JSON.parse(
  numberResultMathCapabilitiesBytes.toString("utf8"),
);
const relativeColorCapabilities = JSON.parse(
  relativeColorCapabilitiesBytes.toString("utf8"),
);
const propertyValueObservations = JSON.parse(propertyValueObservationsBytes.toString("utf8"));
if (
  shorthandGrammarContracts.profiles.length !== 26 ||
  shorthandGrammarCases.length !== 140 ||
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

async function requiredEvidenceReport(argumentName) {
  const argument = process.argv.find(candidate => candidate.startsWith(`--${argumentName}=`));
  if (!argument) throw new Error(`Compatibility recording requires --${argumentName}=path`);
  const reportPath = path.resolve(argument.slice(`--${argumentName}=`.length));
  const bytes = await readFile(reportPath);
  return { bytes, report: JSON.parse(bytes.toString("utf8")) };
}

const nativeCorpusEvidence = await requiredEvidenceReport("native-corpus-report");
const nativeCorpusReport = nativeCorpusEvidence.report;
const grammarPositive = shorthandGrammarCases.filter(candidate => candidate.accepted).length;
const grammarNegative = shorthandGrammarCases.length - grammarPositive;
const propertyPositive = nativeGrammarInventory.propertyBranches
  .filter(candidate => candidate.accepted).length;
const propertyNegative = nativeGrammarInventory.propertyBranches.length - propertyPositive;
const valuePositive = valueCapabilities.cases.filter(candidate => candidate.accepted).length;
const valueNegative = valueCapabilities.cases.length - valuePositive;
const numberResultMathPositive = numberResultMathCapabilities.cases
  .filter(candidate => candidate.accepted).length;
const numberResultMathNegative = numberResultMathCapabilities.cases.length
  - numberResultMathPositive;
const relativeColorPositive = relativeColorCapabilities.cases
  .filter(candidate => candidate.chromiumAccepted).length;
const relativeColorNegative = relativeColorCapabilities.cases.length - relativeColorPositive;
const expectedNativeCorpus = {
  schemaVersion: 1,
  shorthandProperties: {
    passed: shorthandCapabilities.cases.length,
    total: shorthandCapabilities.cases.length,
  },
  grammarBranches: {
    passed: shorthandGrammarCases.length,
    total: shorthandGrammarCases.length,
    positive: grammarPositive,
    negative: grammarNegative,
  },
  propertyBranches: {
    passed: nativeGrammarInventory.propertyBranches.length,
    total: nativeGrammarInventory.propertyBranches.length,
    positive: propertyPositive,
    negative: propertyNegative,
  },
  valueCapabilities: {
    passed: valueCapabilities.cases.length,
    total: valueCapabilities.cases.length,
    positive: valuePositive,
    negative: valueNegative,
  },
  numberResultMath: {
    passed: numberResultMathCapabilities.cases.length,
    total: numberResultMathCapabilities.cases.length,
    positive: numberResultMathPositive,
    negative: numberResultMathNegative,
  },
  relativeColors: {
    passed: relativeColorCapabilities.cases.length,
    total: relativeColorCapabilities.cases.length,
    positive: relativeColorPositive,
    negative: relativeColorNegative,
  },
};
if (JSON.stringify(nativeCorpusReport) !== JSON.stringify(expectedNativeCorpus)) {
  throw new Error("Native Grammar Inventory execution evidence is incomplete");
}

const processSafetyEvidence = await requiredEvidenceReport("process-safety-report");
const processSafetyReport = processSafetyEvidence.report;
const processSafetyContractSha256 = createHash("sha256")
  .update(await readFile(path.join(repositoryRoot, "scripts/test-native-crash-safety.mjs")))
  .digest("hex");
for (const adapter of ["native", "public"]) {
  const evidence = processSafetyReport[adapter];
  if (
    processSafetyReport.schemaVersion !== 1
    || processSafetyReport.contractSha256 !== processSafetyContractSha256
    || !evidence
    || evidence.total < 1
    || evidence.passed !== evidence.total
  ) {
    throw new Error(`Process Safety evidence is incomplete for ${adapter}`);
  }
}

const numericPropertyEvidence = await requiredEvidenceReport("numeric-property-report");
const numericPropertyReport = numericPropertyEvidence.report;
const numericMismatchCount = Object.values(numericPropertyReport.mismatches ?? {})
  .reduce((count, mismatches) => count + (Array.isArray(mismatches) ? mismatches.length : 1), 0);
if (
  numericPropertyReport.schemaVersion !== 1
  || numericPropertyReport.contract !== "compatibility/numeric-property-contracts.json"
  || numericPropertyReport.properties < 1
  || numericPropertyReport.probes < 1
  || numericPropertyReport.expectedAccepted < 1
  || numericMismatchCount !== 0
) {
  throw new Error("Numeric Property contract execution evidence is incomplete");
}

const propertyValueEvidence = await requiredEvidenceReport("property-value-report");
const propertyValueReport = propertyValueEvidence.report;
const propertyValueMismatchCount = Object.values(propertyValueReport.mismatches ?? {})
  .reduce((count, mismatches) => count + (Array.isArray(mismatches) ? mismatches.length : 1), 0);
const propertyValueBaseline = propertyValueObservations.baseline;
if (
  propertyValueReport.schemaVersion !== 1
  || Object.hasOwn(propertyValueReport, "contract")
  || JSON.stringify(propertyValueReport.checks) !== JSON.stringify([
    "acceptance",
    "observable",
    "cssText",
    "items",
    "atomicity",
  ])
  || propertyValueReport.properties !== propertyValueBaseline.propertyCount
  || propertyValueReport.probes !== propertyValueBaseline.probeCount
  || propertyValueReport.expectedAccepted !== propertyValueBaseline.acceptedCount
  || propertyValueMismatchCount !== 0
) {
  throw new Error("Property Value Matrix execution evidence is incomplete");
}

const geometricEvidence = await requiredEvidenceReport("geometric-report");
const geometricReport = geometricEvidence.report;
if (
  geometricReport.schemaVersion !== 1
  || geometricReport.passed !== geometricReport.total
  || geometricReport.reviewed < 1
  || geometricReport.generated < 1
  || geometricReport.reviewed + geometricReport.generated !== geometricReport.total
  || geometricReport.contractsSha256 !== createHash("sha256")
    .update(geometricContractsBytes)
    .digest("hex")
  || geometricReport.generatorSha256 !== createHash("sha256")
    .update(geometricGeneratorBytes)
    .digest("hex")
  || typeof geometricReport.userAgent !== "string"
) {
  throw new Error("Geometric browser differential evidence is incomplete");
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
  schemaVersion: 6,
  packageVersion: packageManifest.version,
  baseline: {
    wptCommit: wptLock.commit,
    nativeEngine: await nativeEngineEvidence(repositoryRoot),
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
    nativeGrammar: {
      inventorySha256: createHash("sha256").update(nativeGrammarInventoryBytes).digest("hex"),
      executionSha256: createHash("sha256").update(nativeCorpusEvidence.bytes).digest("hex"),
      shorthandProperties: nativeCorpusReport.shorthandProperties,
      codecProfiles: shorthandGrammarContracts.profiles.length,
      grammarBranches: {
        ...nativeCorpusReport.grammarBranches,
        contractsSha256: createHash("sha256")
          .update(shorthandGrammarContractsBytes)
          .digest("hex"),
        observationsSha256: createHash("sha256")
          .update(shorthandGrammarObservationsBytes)
          .digest("hex"),
      },
      propertyBranches: nativeCorpusReport.propertyBranches,
      valueCapabilities: {
        ...nativeCorpusReport.valueCapabilities,
        sha256: createHash("sha256").update(valueCapabilitiesBytes).digest("hex"),
      },
      numberResultMath: {
        ...nativeCorpusReport.numberResultMath,
        sha256: createHash("sha256")
          .update(numberResultMathCapabilitiesBytes)
          .digest("hex"),
      },
      numericProperties: {
        passed: numericPropertyReport.properties * numericPropertyReport.probes,
        total: numericPropertyReport.properties * numericPropertyReport.probes,
        accepted: numericPropertyReport.expectedAccepted,
        rejected: numericPropertyReport.properties * numericPropertyReport.probes
          - numericPropertyReport.expectedAccepted,
        contractSha256: createHash("sha256")
          .update(numericPropertyContractsBytes)
          .digest("hex"),
        observationsSha256: createHash("sha256")
          .update(propertyValueObservationsBytes)
          .digest("hex"),
        executionSha256: createHash("sha256")
          .update(numericPropertyEvidence.bytes)
          .digest("hex"),
      },
      propertyValues: {
        passed: propertyValueReport.properties * propertyValueReport.probes,
        total: propertyValueReport.properties * propertyValueReport.probes,
        properties: propertyValueReport.properties,
        probes: propertyValueReport.probes,
        accepted: propertyValueReport.expectedAccepted,
        rejected: propertyValueReport.properties * propertyValueReport.probes
          - propertyValueReport.expectedAccepted,
        observationsSha256: createHash("sha256")
          .update(propertyValueObservationsBytes)
          .digest("hex"),
        probesSha256: createHash("sha256").update(propertyValueProbesBytes).digest("hex"),
        executionSha256: createHash("sha256")
          .update(propertyValueEvidence.bytes)
          .digest("hex"),
      },
      relativeColors: {
        ...nativeCorpusReport.relativeColors,
        sha256: createHash("sha256")
          .update(relativeColorCapabilitiesBytes)
          .digest("hex"),
      },
      geometricBranches: {
        passed: geometricReport.passed,
        total: geometricReport.total,
        reviewed: geometricReport.reviewed,
        generated: geometricReport.generated,
        userAgent: geometricReport.userAgent,
        contractsSha256: geometricReport.contractsSha256,
        generatorSha256: geometricReport.generatorSha256,
        executionSha256: createHash("sha256").update(geometricEvidence.bytes).digest("hex"),
      },
    },
    processSafety: {
      contractSha256: processSafetyReport.contractSha256,
      executionSha256: createHash("sha256").update(processSafetyEvidence.bytes).digest("hex"),
      native: processSafetyReport.native,
      public: processSafetyReport.public,
    },
    nativeWpt
  },
  resolutions
};

await writeFile(output, `${JSON.stringify(report, null, 2)}\n`, { flag: "wx" });
console.log(`Recorded Baseline Draft at ${path.relative(repositoryRoot, output)}.`);
