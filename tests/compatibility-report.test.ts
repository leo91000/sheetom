import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
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
        ...argumentsList,
      ],
      { env: { ...process.env, SHEETOM_RECORD_BASELINE: "1" }, stdio: "ignore" },
    );
    const report = JSON.parse(await readFile(output, "utf8"));
    assert.equal(report.schemaVersion, 3);
    assert.deepEqual(report.baseline.syntaxEngineSet, {
      lightningcss: "1.33.0",
      cssTree: "3.2.1",
      cssstyle: "6.2.0",
      cssTokenizer: "4.0.0",
    });
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
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
});
