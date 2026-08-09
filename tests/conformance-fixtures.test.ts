import assert from "node:assert/strict";
import { readFileSync, readdirSync, writeFileSync } from "node:fs";
import path from "node:path";
import { afterAll, test } from "vitest";

import {
  runOperationFixture,
  type OperationFixture,
} from "./support/operation-fixture.js";
import { createSheetOMFixtureAdapter } from "./support/sheetom-fixture-adapter.js";

interface Resolution {
  fixtureId: string;
  expected: unknown[];
}

function fixtureFiles(directory: string): string[] {
  const files: string[] = [];
  for (const entry of readdirSync(directory, { withFileTypes: true })) {
    const entryPath = path.join(directory, entry.name);
    if (entry.isDirectory()) {
      files.push(...fixtureFiles(entryPath));
      continue;
    }
    if (entry.name.endsWith(".json")) files.push(entryPath);
  }
  return files;
}

const fixtureDirectory = path.resolve("compatibility/fixtures");
const fixtures = fixtureFiles(fixtureDirectory)
  .sort()
  .map(file => JSON.parse(
    readFileSync(file, "utf8"),
  ) as OperationFixture);
const resolutionDocument = JSON.parse(readFileSync(
  path.resolve("compatibility/resolutions/declarations.json"),
  "utf8",
)) as { resolutions: Resolution[] };
const recordedObservations: Array<{ fixtureId: string; operations: unknown[] }> = [];

afterAll(() => {
  const output = process.env.SHEETOM_OPERATION_OBSERVATIONS_PATH;
  if (!output) return;
  writeFileSync(output, `${JSON.stringify(recordedObservations, null, 2)}\n`);
});

for (const fixture of fixtures) {
  test(`SheetOM matches the Compatibility Resolution for ${fixture.id}`, async () => {
    const resolution = resolutionDocument.resolutions.find(
      candidate => candidate.fixtureId === fixture.id,
    );
    assert.ok(resolution, `Missing Compatibility Resolution for ${fixture.id}`);

    const observations = await runOperationFixture(
      fixture,
      createSheetOMFixtureAdapter(),
    );
    recordedObservations.push({ fixtureId: fixture.id, operations: observations });

    assert.deepEqual(observations, resolution.expected);
  });
}
