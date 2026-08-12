import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import { mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import { test } from "vitest";

const subtests = [
  "Verify that setting a CSS property to undefined has no effect.",
  "Verify that setting a CSS property priority to undefined is accepted.",
  "Verify that setting a CSS property to null is treated like empty string.",
  "Verify that setting a CSS property priority to null is treated like empty string.",
];

test("compatibility recording verifies and hashes every native WPT report", async () => {
  const directory = await mkdtemp(path.join(os.tmpdir(), "sheetom-report-test-"));
  try {
    const resolutions = JSON.parse(await readFile(
      "compatibility/resolutions/declarations.json",
      "utf8",
    ));
    const fixtureTotal = resolutions.resolutions.length;
    const operationReportPath = path.join(directory, "operation-fixtures.json");
    await writeFile(operationReportPath, JSON.stringify({
      schemaVersion: 2,
      adapters: ["sheetom", "chromium", "firefox", "webkit"].map(adapter => ({
        adapter,
        version: `${adapter}-test-version`,
        passed: fixtureTotal,
        total: fixtureTotal,
        observations: resolutions.resolutions.map((resolution: { fixtureId: string }) => ({
          fixtureId: resolution.fixtureId,
          operations: [{}],
        })),
      })),
    }));
    const nativeCorpusReportPath = path.join(directory, "native-corpus.json");
    await writeFile(nativeCorpusReportPath, JSON.stringify({
      schemaVersion: 1,
      shorthandProperties: { passed: 129, total: 129 },
      grammarBranches: { passed: 126, total: 126, positive: 92, negative: 34 },
      propertyBranches: { passed: 10, total: 10, positive: 5, negative: 5 },
      valueCapabilities: { passed: 325, total: 325, positive: 212, negative: 113 },
      numberResultMath: { passed: 860, total: 860, positive: 616, negative: 244 },
      relativeColors: { passed: 1306, total: 1306, positive: 1146, negative: 160 },
    }));
    const processSafetyReportPath = path.join(directory, "process-safety.json");
    const processSafetyContractSha256 = createHash("sha256")
      .update(await readFile("scripts/test-native-crash-safety.mjs"))
      .digest("hex");
    await writeFile(processSafetyReportPath, JSON.stringify({
      schemaVersion: 1,
      contractSha256: processSafetyContractSha256,
      native: { passed: 33, total: 33 },
      public: { passed: 7, total: 7 },
    }));
    const numericPropertyReportPath = path.join(directory, "numeric-properties.json");
    await writeFile(numericPropertyReportPath, JSON.stringify({
      schemaVersion: 1,
      contract: "compatibility/numeric-property-contracts.json",
      properties: 57,
      probes: 11,
      expectedAccepted: 309,
      mismatches: {
        acceptance: [],
        observable: [],
        cssText: [],
        items: [],
        atomicity: [],
      },
    }));
    const propertyValueReportPath = path.join(directory, "property-values.json");
    await writeFile(propertyValueReportPath, JSON.stringify({
      schemaVersion: 1,
      checks: ["acceptance", "observable", "cssText", "items", "atomicity"],
      properties: 711,
      probes: 93,
      expectedAccepted: 11_107,
      mismatches: {
        acceptance: [],
        observable: [],
        cssText: [],
        items: [],
        atomicity: [],
      },
    }));
    const geometricReportPath = path.join(directory, "geometric.json");
    await writeFile(geometricReportPath, JSON.stringify({
      schemaVersion: 1,
      userAgent: "HeadlessChrome/151.0.7922.34",
      passed: 199,
      total: 199,
      reviewed: 55,
      generated: 144,
      contractsSha256: createHash("sha256")
        .update(await readFile("compatibility/browser-geometric-contracts.json"))
        .digest("hex"),
      generatorSha256: createHash("sha256")
        .update(await readFile("scripts/browser-geometric-differential.mjs"))
        .digest("hex"),
    }));
    const argumentsList: string[] = [];
    for (const engine of ["chrome", "firefox", "safari"]) {
      const reportPath = path.join(directory, `${engine}.json`);
      await writeFile(reportPath, JSON.stringify({
        run_info: { browser_version: `${engine}-test-version` },
        results: [{
          test: "/css/cssom/setproperty-null-undefined.html",
          subtests: subtests.map(name => ({ name, status: "PASS" })),
        }],
      }));
      argumentsList.push(`--wpt-report=${engine}=${reportPath}`);
    }

    const output = path.join(directory, "compatibility.json");
    execFileSync(
      process.execPath,
      [
        "scripts/record-compatibility.mjs",
        "--output",
        output,
        `--operation-report=${operationReportPath}`,
        `--native-corpus-report=${nativeCorpusReportPath}`,
        `--process-safety-report=${processSafetyReportPath}`,
        `--numeric-property-report=${numericPropertyReportPath}`,
        `--property-value-report=${propertyValueReportPath}`,
        `--geometric-report=${geometricReportPath}`,
        ...argumentsList,
      ],
      { env: { ...process.env, SHEETOM_RECORD_BASELINE: "1" }, stdio: "ignore" },
    );
    const report = JSON.parse(await readFile(output, "utf8"));
    assert.equal(report.schemaVersion, 6);
    assert.deepEqual(report.baseline.nativeEngine.upstream, {
      repository: "https://github.com/parcel-bundler/lightningcss",
      version: "1.33.0",
      commit: "c6a0c3cebf3395635e61075d2c81a96a710d4910",
    });
    assert.deepEqual(report.baseline.nativeEngine.cssSyntax, {
      repository: "https://github.com/servo/rust-cssparser",
      version: "0.37.0",
      commit: "4c49486494fb24dc01390e3baca9698ef1744c71",
    });
    assert.equal(
      report.baseline.nativeEngine.revision,
      "lightningcss-1.33.0-c6a0c3ce-sheetom.57",
    );
    assert.match(report.baseline.nativeEngine.sourceManifestSha256, /^[0-9a-f]{64}$/);
    assert.ok(report.baseline.nativeEngine.sourceFileCount > 200);
    assert.deepEqual(
      {
        passed: report.evidence.nativeGrammar.numberResultMath.passed,
        total: report.evidence.nativeGrammar.numberResultMath.total,
        positive: report.evidence.nativeGrammar.numberResultMath.positive,
        negative: report.evidence.nativeGrammar.numberResultMath.negative,
      },
      { passed: 860, total: 860, positive: 616, negative: 244 },
    );
    assert.deepEqual(
      {
        passed: report.evidence.nativeGrammar.numericProperties.passed,
        total: report.evidence.nativeGrammar.numericProperties.total,
        accepted: report.evidence.nativeGrammar.numericProperties.accepted,
        rejected: report.evidence.nativeGrammar.numericProperties.rejected,
      },
      { passed: 627, total: 627, accepted: 309, rejected: 318 },
    );
    assert.deepEqual(
      {
        passed: report.evidence.nativeGrammar.propertyValues.passed,
        total: report.evidence.nativeGrammar.propertyValues.total,
        properties: report.evidence.nativeGrammar.propertyValues.properties,
        probes: report.evidence.nativeGrammar.propertyValues.probes,
        accepted: report.evidence.nativeGrammar.propertyValues.accepted,
        rejected: report.evidence.nativeGrammar.propertyValues.rejected,
      },
      {
        passed: 66_123,
        total: 66_123,
        properties: 711,
        probes: 93,
        accepted: 11_107,
        rejected: 55_016,
      },
    );
    assert.deepEqual(
      report.evidence.nativeWpt.map((evidence: { engine: string }) => evidence.engine),
      ["chrome", "firefox", "safari"],
    );
    for (const evidence of report.evidence.nativeWpt) {
      assert.equal(evidence.passed, 4);
      assert.equal(evidence.total, 4);
      assert.match(evidence.sha256, /^[0-9a-f]{64}$/);
    }
    assert.deepEqual(
      report.evidence.operationFixtures.adapters.map(
        (evidence: { adapter: string }) => evidence.adapter,
      ),
      ["sheetom", "chromium", "firefox", "webkit"],
    );
    assert.match(report.evidence.operationFixtures.sha256, /^[0-9a-f]{64}$/);
    assert.deepEqual(
      {
        profiles: report.evidence.nativeGrammar.codecProfiles,
        passed: report.evidence.nativeGrammar.grammarBranches.passed,
        total: report.evidence.nativeGrammar.grammarBranches.total,
      },
      { profiles: 25, passed: 126, total: 126 },
    );
    assert.deepEqual(
      report.evidence.nativeGrammar.shorthandProperties,
      { passed: 129, total: 129 },
    );
    assert.deepEqual(
      report.evidence.nativeGrammar.propertyBranches,
      { passed: 10, total: 10, positive: 5, negative: 5 },
    );
    assert.deepEqual(
      {
        passed: report.evidence.nativeGrammar.valueCapabilities.passed,
        total: report.evidence.nativeGrammar.valueCapabilities.total,
        positive: report.evidence.nativeGrammar.valueCapabilities.positive,
        negative: report.evidence.nativeGrammar.valueCapabilities.negative,
      },
      { passed: 325, total: 325, positive: 212, negative: 113 },
    );
    assert.deepEqual(
      {
        passed: report.evidence.nativeGrammar.relativeColors.passed,
        total: report.evidence.nativeGrammar.relativeColors.total,
        positive: report.evidence.nativeGrammar.relativeColors.positive,
        negative: report.evidence.nativeGrammar.relativeColors.negative,
      },
      { passed: 1306, total: 1306, positive: 1146, negative: 160 },
    );
    assert.deepEqual(
      {
        passed: report.evidence.nativeGrammar.geometricBranches.passed,
        total: report.evidence.nativeGrammar.geometricBranches.total,
        reviewed: report.evidence.nativeGrammar.geometricBranches.reviewed,
        generated: report.evidence.nativeGrammar.geometricBranches.generated,
      },
      { passed: 199, total: 199, reviewed: 55, generated: 144 },
    );
    assert.match(
      report.evidence.nativeGrammar.grammarBranches.contractsSha256,
      /^[0-9a-f]{64}$/,
    );
    assert.match(
      report.evidence.nativeGrammar.grammarBranches.observationsSha256,
      /^[0-9a-f]{64}$/,
    );
    assert.deepEqual(report.evidence.processSafety.native, { passed: 33, total: 33 });
    assert.deepEqual(report.evidence.processSafety.public, { passed: 7, total: 7 });
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
});
