import { execFileSync } from "node:child_process";
import { mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

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
const fixtureTotal = resolutionDocument.resolutions.length;
const fixtureIds = new Set(
  resolutionDocument.resolutions.map(resolution => resolution.fixtureId),
);
const adapters = [];
const vitest = path.join(repositoryRoot, "node_modules/vitest/vitest.mjs");
const reportDirectory = await mkdtemp(path.join(os.tmpdir(), "sheetom-operation-evidence-"));

async function runFixtureSuite(adapter, project, environment = {}) {
  const reportPath = path.join(reportDirectory, `${adapter}.json`);
  execFileSync(
    process.execPath,
    [
      vitest,
      "run",
      "--project",
      project,
      project === "unit"
        ? "tests/conformance-fixtures.test.ts"
        : "tests/browser/operation-fixtures.test.ts",
      "--reporter=json",
      `--outputFile=${reportPath}`,
    ],
    {
      cwd: repositoryRoot,
      env: { ...process.env, ...environment },
      stdio: "inherit",
    },
  );
  const report = JSON.parse(await readFile(reportPath, "utf8"));
  if (
    report.numTotalTests !== fixtureTotal ||
    report.numPassedTests !== fixtureTotal ||
    report.numFailedTests !== 0 ||
    report.numPendingTests !== 0
  ) {
    throw new Error(
      `${adapter} executed ${report.numTotalTests ?? 0}/${fixtureTotal} Operation Fixtures with ${report.numPassedTests ?? 0} passes`,
    );
  }

  const titlePrefix = project === "unit"
    ? "SheetOM matches the Compatibility Resolution for "
    : "the native adapter executes ";
  const observedFixtureIds = new Set(
    report.testResults.flatMap(result => result.assertionResults)
      .filter(result => result.status === "passed" && result.title.startsWith(titlePrefix))
      .map(result => result.title.slice(titlePrefix.length)),
  );
  if (
    observedFixtureIds.size !== fixtureIds.size ||
    [...fixtureIds].some(fixtureId => !observedFixtureIds.has(fixtureId))
  ) {
    throw new Error(`${adapter} did not execute the complete Operation Fixture ID set`);
  }
  adapters.push({ adapter, passed: report.numPassedTests, total: report.numTotalTests });
}

try {
  await runFixtureSuite("sheetom", "unit");
  for (const browser of ["chromium", "firefox", "webkit"]) {
    await runFixtureSuite(browser, "browser", { SHEETOM_BROWSER: browser });
  }
} finally {
  await rm(reportDirectory, { recursive: true, force: true });
}

await writeFile(output, `${JSON.stringify({ schemaVersion: 1, adapters }, null, 2)}\n`, {
  flag: "wx",
});
console.log(`Recorded Operation Fixture evidence at ${path.relative(repositoryRoot, output)}.`);
