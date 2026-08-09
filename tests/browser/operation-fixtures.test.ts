import { expect, test } from "vitest";

import fixture from "../../compatibility/fixtures/declarations/basic-set-property.json" with { type: "json" };
import nullUndefinedFixture from "../../compatibility/fixtures/declarations/setproperty-null-undefined.json" with { type: "json" };
import requiredArgumentsFixture from "../../compatibility/fixtures/declarations/required-arguments.json" with { type: "json" };
import resolutionDocument from "../../compatibility/resolutions/declarations.json" with { type: "json" };
import { createNativeBrowserFixtureAdapter } from "../support/browser-fixture-adapter.js";
import {
  runOperationFixture,
  type OperationFixture,
} from "../support/operation-fixture.js";

function expectedResolution(fixtureId: string): unknown[] {
  const resolution = resolutionDocument.resolutions.find(
    candidate => candidate.fixtureId === fixtureId,
  );
  if (!resolution) throw new Error(`Missing Compatibility Resolution for ${fixtureId}`);
  return resolution.expected;
}

test("shared Operation Fixtures execute through the native browser adapter", async () => {
  const observations = await runOperationFixture(
    fixture as OperationFixture,
    createNativeBrowserFixtureAdapter(),
  );

  expect(observations).toEqual(expectedResolution(fixture.id));
});

test("adapted WPT fixtures preserve null and undefined boundaries", async () => {
  const observations = await runOperationFixture(
    nullUndefinedFixture as OperationFixture,
    createNativeBrowserFixtureAdapter(),
  );

  expect(observations).toEqual(expectedResolution(nullUndefinedFixture.id));
});

test("required argument fixtures execute through the native browser adapter", async () => {
  const observations = await runOperationFixture(
    requiredArgumentsFixture as OperationFixture,
    createNativeBrowserFixtureAdapter(),
  );
  expect(observations).toEqual(expectedResolution(requiredArgumentsFixture.id));
});
