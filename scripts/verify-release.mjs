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
if (report.packageVersion !== manifest.version) {
  throw new Error(`${reportPath} does not describe package version ${manifest.version}`);
}
if (report.schemaVersion !== 5) {
  throw new Error("RC6 releases require Compatibility Report schema version 5");
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
if (
  nativeGrammar?.codecProfiles !== 24 ||
  nativeGrammar?.shorthandProperties?.passed !== 129 ||
  nativeGrammar?.shorthandProperties?.total !== 129 ||
  shorthandGrammar?.passed !== 96 ||
  shorthandGrammar?.total !== 96 ||
  shorthandGrammar?.positive !== 72 ||
  shorthandGrammar?.negative !== 24 ||
  nativeGrammar?.propertyBranches?.passed !== 10 ||
  nativeGrammar?.propertyBranches?.total !== 10 ||
  nativeGrammar?.propertyBranches?.positive !== 5 ||
  nativeGrammar?.propertyBranches?.negative !== 5 ||
  nativeGrammar?.valueCapabilities?.passed !== 36 ||
  nativeGrammar?.valueCapabilities?.total !== 36 ||
  nativeGrammar?.valueCapabilities?.positive !== 27 ||
  nativeGrammar?.valueCapabilities?.negative !== 9 ||
  !/^[0-9a-f]{64}$/.test(nativeGrammar?.inventorySha256 ?? "") ||
  !/^[0-9a-f]{64}$/.test(nativeGrammar?.executionSha256 ?? "") ||
  !/^[0-9a-f]{64}$/.test(shorthandGrammar?.contractsSha256 ?? "") ||
  !/^[0-9a-f]{64}$/.test(shorthandGrammar?.observationsSha256 ?? "") ||
  !/^[0-9a-f]{64}$/.test(nativeGrammar?.valueCapabilities?.sha256 ?? "")
) {
  throw new Error("Native Grammar Inventory evidence is incomplete");
}
for (const [filename, recordedHash] of [
  ["compatibility/native-grammar-inventory.json", nativeGrammar.inventorySha256],
  ["compatibility/shorthand-grammar-contracts.json", shorthandGrammar.contractsSha256],
  ["compatibility/shorthand-grammar-observations.json", shorthandGrammar.observationsSha256],
  ["compatibility/value-capabilities.json", nativeGrammar.valueCapabilities.sha256],
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
