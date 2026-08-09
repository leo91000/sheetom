import assert from "node:assert/strict";
import fc from "fast-check";
import { test } from "vitest";

import { CSSStyleRule, CSSStyleSheet, parseStyleSheet } from "../../src/index.js";

const propertyName = fc.constantFrom(
  "color",
  "width",
  "padding",
  "padding-left",
  "margin",
  "--token",
  "--Token",
);
const mutation = fc.oneof(
  fc.record({
    operation: fc.constant("set" as const),
    name: propertyName,
    value: fc.string({ maxLength: 80 }),
    priority: fc.constantFrom("", "important", "bogus"),
  }),
  fc.record({
    operation: fc.constant("remove" as const),
    name: propertyName,
  }),
);

test("arbitrary declaration sequences preserve reparsable stylesheet output", () => {
  const numRuns = Number.parseInt(process.env.SHEETOM_FUZZ_RUNS ?? "200", 10);

  fc.assert(
    fc.property(fc.array(mutation, { maxLength: 50 }), mutations => {
      const sheet = new CSSStyleSheet();
      sheet.insertRule(".fuzz {}");
      const rule = sheet.cssRules[0];
      assert.ok(rule instanceof CSSStyleRule);

      for (const candidate of mutations) {
        if (candidate.operation === "remove") {
          rule.style.removeProperty(candidate.name);
          continue;
        }
        rule.style.setProperty(candidate.name, candidate.value, candidate.priority);
      }

      const serialized = sheet.serialize();
      const reparsed = parseStyleSheet(serialized);
      assert.equal(reparsed.cssRules.length, 1, serialized);
    }),
    { seed: 0x5e37_0a, numRuns },
  );
});
