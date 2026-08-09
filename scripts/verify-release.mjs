import { execFileSync } from "node:child_process";
import { readFile } from "node:fs/promises";

const manifest = JSON.parse(await readFile("package.json", "utf8"));
if (manifest.version === "0.0.0") {
  throw new Error("Changesets must assign a release version before release verification");
}

const reportPath = `compatibility/baselines/${manifest.version}.json`;
const report = JSON.parse(await readFile(reportPath, "utf8"));
if (report.packageVersion !== manifest.version) {
  throw new Error(`${reportPath} does not describe package version ${manifest.version}`);
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
