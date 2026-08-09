import assert from "node:assert/strict";
import { test } from "vitest";

import shorthandCapabilities from "../compatibility/shorthand-capabilities.json" with { type: "json" };
import { chromiumShorthandLonghands } from "../src/chromium-properties.js";
import { getStaticShorthandDefinitions } from "../src/internal/shorthand-registry.js";

const cssWideKeywords = new Set([
  "initial",
  "inherit",
  "unset",
  "revert",
  "revert-layer",
]);

test("the shorthand capability corpus covers every manifested multi-longhand property", () => {
  const manifested = Object.entries(chromiumShorthandLonghands)
    .filter(([, longhands]) => longhands.length > 1)
    .map(([property]) => property)
    .sort();
  const corpusProperties = shorthandCapabilities.cases
    .map(capability => capability.property)
    .sort();
  const registered = getStaticShorthandDefinitions()
    .map(definition => definition.name)
    .sort();

  assert.equal(manifested.length, 129);
  assert.deepEqual(corpusProperties, manifested);
  assert.deepEqual(registered, manifested);
  assert.equal(new Set(corpusProperties).size, corpusProperties.length);
});

test("every shorthand seed is concrete and retains ordered Chromium observations", () => {
  for (const capability of shorthandCapabilities.cases) {
    const expectedLonghands = chromiumShorthandLonghands[capability.property];
    assert.ok(expectedLonghands, capability.property);
    assert.equal(cssWideKeywords.has(capability.input), false, capability.property);
    assert.equal(capability.chromium.accepted, true, capability.property);
    assert.deepEqual(
      [...capability.chromium.items].sort(),
      [...expectedLonghands].sort(),
      capability.property,
    );
    assert.deepEqual(
      capability.chromium.longhands.map(longhand => longhand.name),
      capability.chromium.items,
      capability.property,
    );
    assert.equal(
      expectedLonghands.includes(capability.mutationProbe.longhand),
      true,
      capability.property,
    );
  }

  assert.deepEqual(
    shorthandCapabilities.cases
      .filter(capability => capability.source === "manual")
      .map(capability => capability.property),
    ["-webkit-mask-box-image"],
  );
});
