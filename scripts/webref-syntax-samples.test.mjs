import assert from "node:assert/strict";
import test from "node:test";

import { generateWebrefSyntaxSamples } from "./lib/webref-syntax-samples.mjs";

const definitions = {
  properties: {
    color: { syntax: "red | blue" },
    inset: { syntax: "auto | <length>" },
  },
  types: {
    paint: { syntax: "<'color'> || <length>" },
    pair: { syntax: "[ <length> | auto ]{1,2}" },
  },
  functions: {},
};

test("generates every alternative and respects primitive ranges", () => {
  const result = generateWebrefSyntaxSamples({
    definitions,
    property: "size",
    syntax: "none | <length [0,∞]>",
  });
  assert.deepEqual(result.issues, []);
  assert.deepEqual(
    new Set(result.samples.map(sample => sample.value)),
    new Set(["none", "1px", "0px"]),
  );
});

test("covers unordered, repeated, and referenced productions without a Cartesian explosion", () => {
  const result = generateWebrefSyntaxSamples({
    definitions,
    property: "example",
    syntax: "<paint> | <pair>#",
  });
  assert.deepEqual(result.issues, []);
  const values = new Set(result.samples.map(sample => sample.value));
  assert.ok(values.has("red"));
  assert.ok(values.has("blue"));
  assert.ok(values.has("1px"));
  assert.ok(values.has("red 1px"));
  assert.ok(values.has("1px red"));
  assert.ok(values.has("1px, 1px"));
});

test("covers bounded multiplier minimum, adjacent, and maximum cardinalities", () => {
  const result = generateWebrefSyntaxSamples({
    definitions,
    property: "example",
    syntax: "<length>{2,4}",
  });
  assert.deepEqual(result.issues, []);
  assert.deepEqual(
    new Set(result.samples.map(sample => sample.value)),
    new Set([
      "1px 1px",
      "0px 0px",
      "-1px -1px",
      "1px 1px 1px",
      "1px 1px 1px 1px",
    ]),
  );
  assert.ok(!result.samples.some(sample => sample.value === "1px"));
});

test("covers pair subsets and both orders for double-bar groups", () => {
  const result = generateWebrefSyntaxSamples({
    definitions,
    property: "example",
    syntax: "alpha || beta || gamma",
  });
  assert.deepEqual(result.issues, []);
  const values = new Set(result.samples.map(sample => sample.value));
  assert.ok(values.has("alpha beta"));
  assert.ok(values.has("beta alpha"));
  assert.ok(values.has("alpha gamma"));
  assert.ok(values.has("gamma alpha"));
  assert.ok(values.has("alpha beta gamma"));
  assert.ok(values.has("gamma beta alpha"));
});

test("uses an explicit property fallback to terminate reference cycles", () => {
  const cyclicDefinitions = {
    properties: {
      first: { syntax: "<'second'>" },
      second: { syntax: "<'first'>" },
    },
    types: {},
    functions: {},
  };
  const result = generateWebrefSyntaxSamples({
    definitions: cyclicDefinitions,
    property: "first",
    syntax: cyclicDefinitions.properties.first.syntax,
    fallbackValue: property => property === "first" ? "initial" : null,
  });
  assert.deepEqual(result.issues, []);
  assert.equal(result.samples[0]?.value, "initial");
});

test("reports missing definitions and sampling budgets instead of silently dropping branches", () => {
  const missing = generateWebrefSyntaxSamples({
    definitions,
    property: "example",
    syntax: "<unknown>",
  });
  assert.equal(missing.samples.length, 0);
  assert.equal(missing.issues[0]?.kind, "missing-definition");

  const limited = generateWebrefSyntaxSamples({
    definitions,
    property: "example",
    syntax: "a | b | c",
    maximumSamplesPerNode: 2,
  });
  assert.equal(limited.samples.length, 2);
  assert.equal(limited.issues[0]?.kind, "sample-budget");
});

test("stops at reviewed semantic terminals", () => {
  const result = generateWebrefSyntaxSamples({
    definitions: {
      properties: {},
      types: { color: { syntax: "<color>" } },
      functions: {},
    },
    property: "example",
    syntax: "none | <color>",
    terminalValues: { color: ["red", "oklch(50% .2 120)"] },
  });
  assert.deepEqual(result.issues, []);
  assert.deepEqual(
    result.samples.map(sample => sample.value),
    ["none", "red", "oklch(50% .2 120)"],
  );
});
