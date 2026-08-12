import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import { readFile } from "node:fs/promises";

import { nativeEngineEvidence } from "./native-engine-evidence.mjs";
import {
  replaceCargoLockVersions,
  replaceCargoPackageVersion,
} from "./sync-cargo-version.mjs";

const manifest = JSON.parse(await readFile("package.json", "utf8"));
if (manifest.version === "0.0.0") {
  throw new Error("Changesets must assign a release version before release verification");
}
for (const filename of [
  "crates/sheetom-core/Cargo.toml",
  "crates/sheetom-native/Cargo.toml",
]) {
  const source = await readFile(filename, "utf8");
  if (replaceCargoPackageVersion(source, manifest.version) !== source) {
    throw new Error(`${filename} does not match package version ${manifest.version}`);
  }
}
const cargoLock = await readFile("Cargo.lock", "utf8");
if (replaceCargoLockVersions(cargoLock, manifest.version) !== cargoLock) {
  throw new Error(`Cargo.lock does not match package version ${manifest.version}`);
}

const reportPath = `compatibility/baselines/${manifest.version}.json`;
const report = JSON.parse(await readFile(reportPath, "utf8"));
const shorthandGrammarContracts = JSON.parse(
  await readFile("compatibility/shorthand-grammar-contracts.json", "utf8"),
);
const nativeGrammarInventory = JSON.parse(
  await readFile("compatibility/native-grammar-inventory.json", "utf8"),
);
const valueCapabilities = JSON.parse(
  await readFile("compatibility/value-capabilities.json", "utf8"),
);
const webrefPropertyBranchesBytes = await readFile(
  "compatibility/webref-property-branches.json",
);
const webrefBranchRatchetBytes = await readFile(
  "compatibility/webref-branch-ratchet.json",
);
const webrefPropertyBranches = JSON.parse(webrefPropertyBranchesBytes.toString("utf8"));
const webrefBranchRatchet = JSON.parse(webrefBranchRatchetBytes.toString("utf8"));
const webrefCorpusSha256 = createHash("sha256")
  .update(webrefPropertyBranchesBytes)
  .digest("hex");
const grammarCases = shorthandGrammarContracts.profiles.flatMap(profile => profile.cases);
const grammarPositive = grammarCases.filter(grammarCase => grammarCase.accepted).length;
const valuePositive = valueCapabilities.cases.filter(capability => capability.accepted).length;
if (report.packageVersion !== manifest.version) {
  throw new Error(`${reportPath} does not describe package version ${manifest.version}`);
}
if (report.schemaVersion !== 6) {
  throw new Error("RC6 releases require Compatibility Report schema version 6");
}
const expectedNativeEngine = await nativeEngineEvidence(process.cwd());
if (JSON.stringify(report.baseline.nativeEngine) !== JSON.stringify(expectedNativeEngine)) {
  throw new Error("The native engine source does not match the Compatibility Report");
}
if (report.summary.unexplained !== 0) {
  throw new Error("A release cannot contain unexplained compatibility outcomes");
}
const nativeEngines = new Set(report.evidence.nativeWpt.map(evidence => evidence.engine));
for (const engine of ["chrome", "firefox"]) {
  if (!nativeEngines.has(engine)) {
    throw new Error(`The release Compatibility Report lacks stable ${engine} WPT evidence`);
  }
}
for (const evidence of report.evidence.nativeWpt) {
  if (evidence.passed !== evidence.total) {
    throw new Error(`${evidence.engine} native WPT evidence is not fully passing`);
  }
}
if (report.evidence.operationFixtures.passed !== report.evidence.operationFixtures.total) {
  throw new Error("Operation Fixture evidence is not fully passing");
}
if (!/^[0-9a-f]{64}$/.test(report.evidence.operationFixtures.sha256 ?? "")) {
  throw new Error("Operation Fixture evidence does not identify its executed report");
}
const nativeGrammar = report.evidence.nativeGrammar;
const shorthandGrammar = nativeGrammar?.grammarBranches;
const geometricGrammar = nativeGrammar?.geometricBranches;
const numericProperties = nativeGrammar?.numericProperties;
const propertyValues = nativeGrammar?.propertyValues;
const webrefBranches = nativeGrammar?.webrefBranches;
if (
  nativeGrammar?.codecProfiles !== shorthandGrammarContracts.profiles.length ||
  nativeGrammar?.shorthandProperties?.passed !== nativeGrammarInventory.properties.length ||
  nativeGrammar?.shorthandProperties?.total !== nativeGrammarInventory.properties.length ||
  shorthandGrammar?.passed !== grammarCases.length ||
  shorthandGrammar?.total !== grammarCases.length ||
  shorthandGrammar?.positive !== grammarPositive ||
  shorthandGrammar?.negative !== grammarCases.length - grammarPositive ||
  nativeGrammar?.propertyBranches?.passed !== nativeGrammarInventory.propertyBranches.length ||
  nativeGrammar?.propertyBranches?.total !== nativeGrammarInventory.propertyBranches.length ||
  nativeGrammar?.propertyBranches?.positive !== nativeGrammarInventory.propertyBranches.filter(branch => branch.accepted).length ||
  nativeGrammar?.propertyBranches?.negative !== nativeGrammarInventory.propertyBranches.filter(branch => !branch.accepted).length ||
  nativeGrammar?.valueCapabilities?.passed !== valueCapabilities.cases.length ||
  nativeGrammar?.valueCapabilities?.total !== valueCapabilities.cases.length ||
  nativeGrammar?.valueCapabilities?.positive !== valuePositive ||
  nativeGrammar?.valueCapabilities?.negative !== valueCapabilities.cases.length - valuePositive ||
  nativeGrammar?.numberResultMath?.passed !== 860 ||
  nativeGrammar?.numberResultMath?.total !== 860 ||
  nativeGrammar?.numberResultMath?.positive !== 616 ||
  nativeGrammar?.numberResultMath?.negative !== 244 ||
  numericProperties?.passed !== 627 ||
  numericProperties?.total !== 627 ||
  numericProperties?.accepted !== 309 ||
  numericProperties?.rejected !== 318 ||
  propertyValues?.passed !== 66_123 ||
  propertyValues?.total !== 66_123 ||
  propertyValues?.properties !== 711 ||
  propertyValues?.probes !== 93 ||
  propertyValues?.accepted !== 11_107 ||
  propertyValues?.rejected !== 55_016 ||
  webrefBranches?.passed !== webrefPropertyBranches.coverage.checks ||
  webrefBranches?.total !== webrefPropertyBranches.coverage.checks ||
  webrefBranches?.properties !== webrefPropertyBranches.coverage.webrefProperties ||
  webrefBranches?.profiles !== webrefPropertyBranches.coverage.profiles ||
  webrefBranches?.branches !== webrefPropertyBranches.coverage.branches ||
  webrefBranches?.accepted !== webrefPropertyBranches.coverage.accepted ||
  webrefBranches?.rejected !== webrefPropertyBranches.coverage.rejected ||
  webrefBranchRatchet.mismatchCases !== 0 ||
  webrefBranchRatchet.corpusSha256 !== webrefCorpusSha256 ||
  nativeGrammar?.relativeColors?.passed !== 1_306 ||
  nativeGrammar?.relativeColors?.total !== 1_306 ||
  nativeGrammar?.relativeColors?.positive !== 1_146 ||
  nativeGrammar?.relativeColors?.negative !== 160 ||
  geometricGrammar?.passed !== 317 ||
  geometricGrammar?.total !== 317 ||
  geometricGrammar?.reviewed !== 61 ||
  geometricGrammar?.generated !== 256 ||
  typeof geometricGrammar?.userAgent !== "string" ||
  !/^[0-9a-f]{64}$/.test(nativeGrammar?.inventorySha256 ?? "") ||
  !/^[0-9a-f]{64}$/.test(nativeGrammar?.executionSha256 ?? "") ||
  !/^[0-9a-f]{64}$/.test(shorthandGrammar?.contractsSha256 ?? "") ||
  !/^[0-9a-f]{64}$/.test(shorthandGrammar?.observationsSha256 ?? "") ||
  !/^[0-9a-f]{64}$/.test(nativeGrammar?.valueCapabilities?.sha256 ?? "") ||
  !/^[0-9a-f]{64}$/.test(nativeGrammar?.numberResultMath?.sha256 ?? "") ||
  !/^[0-9a-f]{64}$/.test(numericProperties?.contractSha256 ?? "") ||
  !/^[0-9a-f]{64}$/.test(numericProperties?.observationsSha256 ?? "") ||
  !/^[0-9a-f]{64}$/.test(numericProperties?.executionSha256 ?? "") ||
  !/^[0-9a-f]{64}$/.test(propertyValues?.observationsSha256 ?? "") ||
  !/^[0-9a-f]{64}$/.test(propertyValues?.probesSha256 ?? "") ||
  !/^[0-9a-f]{64}$/.test(propertyValues?.executionSha256 ?? "") ||
  !/^[0-9a-f]{64}$/.test(webrefBranches?.corpusSha256 ?? "") ||
  !/^[0-9a-f]{64}$/.test(webrefBranches?.ratchetSha256 ?? "") ||
  !/^[0-9a-f]{64}$/.test(webrefBranches?.executionSha256 ?? "") ||
  !/^[0-9a-f]{64}$/.test(nativeGrammar?.relativeColors?.sha256 ?? "") ||
  !/^[0-9a-f]{64}$/.test(geometricGrammar?.contractsSha256 ?? "") ||
  !/^[0-9a-f]{64}$/.test(geometricGrammar?.generatorSha256 ?? "") ||
  !/^[0-9a-f]{64}$/.test(geometricGrammar?.executionSha256 ?? "")
) {
  throw new Error("Native Grammar Inventory evidence is incomplete");
}
for (const [filename, recordedHash] of [
  ["compatibility/native-grammar-inventory.json", nativeGrammar.inventorySha256],
  ["compatibility/shorthand-grammar-contracts.json", shorthandGrammar.contractsSha256],
  ["compatibility/shorthand-grammar-observations.json", shorthandGrammar.observationsSha256],
  ["compatibility/value-capabilities.json", nativeGrammar.valueCapabilities.sha256],
  [
    "compatibility/number-result-math-capabilities.json",
    nativeGrammar.numberResultMath.sha256,
  ],
  ["compatibility/numeric-property-contracts.json", numericProperties.contractSha256],
  ["compatibility/property-value-observations.json", numericProperties.observationsSha256],
  ["compatibility/property-value-observations.json", propertyValues.observationsSha256],
  ["compatibility/property-value-probes.json", propertyValues.probesSha256],
  ["compatibility/webref-property-branches.json", webrefBranches.corpusSha256],
  ["compatibility/webref-branch-ratchet.json", webrefBranches.ratchetSha256],
  ["compatibility/relative-color-capabilities.json", nativeGrammar.relativeColors.sha256],
  ["compatibility/browser-geometric-contracts.json", geometricGrammar.contractsSha256],
  ["scripts/browser-geometric-differential.mjs", geometricGrammar.generatorSha256],
]) {
  const actualHash = createHash("sha256").update(await readFile(filename)).digest("hex");
  if (actualHash !== recordedHash) {
    throw new Error(`${filename} does not match the release Compatibility Report`);
  }
}
const processSafety = report.evidence.processSafety;
const processSafetyContractSha256 = createHash("sha256")
  .update(await readFile("scripts/test-native-crash-safety.mjs"))
  .digest("hex");
if (
  processSafety?.native?.passed !== processSafety?.native?.total ||
  processSafety?.public?.passed !== processSafety?.public?.total ||
  processSafety?.native?.total < 1 ||
  processSafety?.public?.total < 1 ||
  processSafety?.contractSha256 !== processSafetyContractSha256 ||
  !/^[0-9a-f]{64}$/.test(processSafety?.executionSha256 ?? "")
) {
  throw new Error("Process Safety evidence is incomplete");
}
const operationAdapters = new Map(
  report.evidence.operationFixtures.adapters?.map(evidence => [evidence.adapter, evidence]) ?? [],
);
for (const adapter of ["sheetom", "chromium", "firefox", "webkit"]) {
  const evidence = operationAdapters.get(adapter);
  if (!evidence) {
    throw new Error(`The release Compatibility Report lacks ${adapter} Operation Fixture evidence`);
  }
  if (evidence.passed !== evidence.total || evidence.total !== report.evidence.operationFixtures.total) {
    throw new Error(`${adapter} Operation Fixture evidence is incomplete`);
  }
  if (
    !Array.isArray(evidence.observations) ||
    evidence.observations.length !== evidence.total
  ) {
    throw new Error(`${adapter} Operation Fixture observations are incomplete`);
  }
}

const status = execFileSync("git", ["status", "--porcelain"], { encoding: "utf8" });
if (status !== "") {
  throw new Error("Release verification requires a clean reviewed commit");
}

const pack = JSON.parse(execFileSync("npm", ["pack", "--dry-run", "--json"], {
  encoding: "utf8",
}));
const files = new Set(pack[0]?.files?.map(file => file.path) ?? []);
if (!files.has(reportPath)) {
  throw new Error(`The npm tarball does not contain ${reportPath}`);
}

console.log(`Release inputs for sheetom@${manifest.version} are internally consistent.`);
