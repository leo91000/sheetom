/// <reference types="vite/client" />

import { expect, test } from "vitest";

import resolutionDocument from "../../compatibility/resolutions/declarations.json" with { type: "json" };
import { createNativeBrowserFixtureAdapter } from "../support/browser-fixture-adapter.js";
import {
  runOperationFixture,
  type OperationFixture,
} from "../support/operation-fixture.js";

interface FixtureModule {
  default: OperationFixture;
}

const fixtureModules = import.meta.glob<FixtureModule>(
  "../../compatibility/fixtures/**/*.json",
  { eager: true },
);
const fixtures = Object.values(fixtureModules)
  .map(module => module.default)
  .sort((left, right) => left.id.localeCompare(right.id));
const chromium = navigator.userAgent.includes("Chrome");

for (const fixture of fixtures) {
  test(`the native adapter executes ${fixture.id}`, async () => {
    const resolution = resolutionDocument.resolutions.find(
      candidate => candidate.fixtureId === fixture.id,
    );
    if (!resolution) throw new Error(`Missing Compatibility Resolution for ${fixture.id}`);

    const observations = await runOperationFixture(
      fixture,
      createNativeBrowserFixtureAdapter(),
    );

    expect(observations).toHaveLength(fixture.operations.length);
    if (resolution.decision !== "chromium-fallback" || chromium) {
      expect(observations).toEqual(resolution.expected);
    }
  });
}
