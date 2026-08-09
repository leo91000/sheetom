import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import path from "node:path";
import { test } from "vitest";

import { runOperationFixture } from "./support/operation-fixture.js";
import { createSheetOMFixtureAdapter } from "./support/sheetom-fixture-adapter.js";

async function expectedResolution(fixtureId: string): Promise<unknown[]> {
  const file = path.resolve("compatibility/resolutions/declarations.json");
  const document = JSON.parse(await readFile(file, "utf8"));
  const resolution = document.resolutions.find(
    (candidate: { fixtureId: string }) => candidate.fixtureId === fixtureId,
  );
  assert.ok(resolution, `Missing Compatibility Resolution for ${fixtureId}`);
  return resolution.expected;
}

test("an Operation Fixture observes malformed declaration behavior through the public interface", async () => {
  const fixturePath = path.resolve(
    "compatibility/fixtures/declarations/malformed-padding.json",
  );
  const fixture = JSON.parse(await readFile(fixturePath, "utf8"));

  const observations = await runOperationFixture(
    fixture,
    createSheetOMFixtureAdapter(),
  );

  assert.deepEqual(observations, await expectedResolution(fixture.id));
});

test("adapted WPT null and undefined operations preserve WebIDL argument boundaries", async () => {
  const fixturePath = path.resolve(
    "compatibility/fixtures/declarations/setproperty-null-undefined.json",
  );
  const fixture = JSON.parse(await readFile(fixturePath, "utf8"));

  const observations = await runOperationFixture(
    fixture,
    createSheetOMFixtureAdapter(),
  );

  assert.deepEqual(observations, await expectedResolution(fixture.id));
});

test("Operation Fixtures preserve required WebIDL argument arity", async () => {
  const fixturePath = path.resolve(
    "compatibility/fixtures/declarations/required-arguments.json",
  );
  const fixture = JSON.parse(await readFile(fixturePath, "utf8"));
  const observations = await runOperationFixture(
    fixture,
    createSheetOMFixtureAdapter(),
  );
  assert.deepEqual(observations, await expectedResolution(fixture.id));
});
