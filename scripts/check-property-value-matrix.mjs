import { writeFile } from "node:fs/promises";
import path from "node:path";

import { CSSStyleRule, CSSStyleSheet } from "../dist/index.js";
import observations from "../compatibility/property-value-observations.json" with { type: "json" };
import probes from "../compatibility/property-value-probes.json" with { type: "json" };
import { chromiumSupportedProperties } from "../src/chromium-properties.ts";

const expectedByKey = new Map(
  observations.accepted.map(candidate => [
    `${candidate.property}\0${candidate.probe}`,
    candidate,
  ]),
);
const sheet = new CSSStyleSheet();
sheet.insertRule(".probe {}");
const rule = sheet.cssRules[0];
if (!(rule instanceof CSSStyleRule)) throw new TypeError("Expected a style rule");

const acceptance = [];
const observable = [];
const cssText = [];
const items = [];
const atomicity = [];
for (const property of chromiumSupportedProperties) {
  for (const probe of probes.values) {
    const expected = expectedByKey.get(`${property}\0${probe.id}`);
    rule.style.cssText = "";
    rule.style.setProperty(property, probe.input);
    const actualAccepted = rule.style.length > 0;
    if (Boolean(expected) !== actualAccepted) {
      acceptance.push({
        property,
        probe: probe.id,
        input: probe.input,
        expected: Boolean(expected),
        actual: actualAccepted,
      });
      continue;
    }
    if (!expected) {
      rule.style.setProperty(property, "initial");
      const before = {
        cssText: rule.style.cssText,
        items: Array.from(
          { length: rule.style.length },
          (_, index) => {
            const name = rule.style.item(index);
            return {
              name,
              value: rule.style.getPropertyValue(name),
              priority: rule.style.getPropertyPriority(name),
            };
          },
        ),
      };
      rule.style.setProperty(property, probe.input);
      const after = {
        cssText: rule.style.cssText,
        items: Array.from(
          { length: rule.style.length },
          (_, index) => {
            const name = rule.style.item(index);
            return {
              name,
              value: rule.style.getPropertyValue(name),
              priority: rule.style.getPropertyPriority(name),
            };
          },
        ),
      };
      if (JSON.stringify(before) !== JSON.stringify(after)) {
        atomicity.push({ property, probe: probe.id, input: probe.input, before, after });
      }
      continue;
    }

    const actualObservable = rule.style.getPropertyValue(property);
    if (actualObservable !== expected.observable) {
      observable.push({
        property,
        probe: probe.id,
        input: probe.input,
        expected: expected.observable,
        actual: actualObservable,
      });
    }
    if (rule.style.cssText !== expected.cssText) {
      cssText.push({
        property,
        probe: probe.id,
        input: probe.input,
        expected: expected.cssText,
        actual: rule.style.cssText,
      });
    }
    const actualItems = Array.from(
      { length: rule.style.length },
      (_, index) => rule.style.item(index),
    );
    if (JSON.stringify(actualItems) !== JSON.stringify(expected.items)) {
      items.push({
        property,
        probe: probe.id,
        input: probe.input,
        expected: expected.items,
        actual: actualItems,
      });
    }
  }
}

const report = {
  schemaVersion: 1,
  properties: chromiumSupportedProperties.size,
  probes: probes.values.length,
  expectedAccepted: observations.accepted.length,
  mismatches: {
    acceptance,
    observable,
    cssText,
    items,
    atomicity,
  },
};
const reportArgument = process.argv.find(argument => argument.startsWith("--report="));
if (reportArgument) {
  await writeFile(
    path.resolve(reportArgument.slice("--report=".length)),
    `${JSON.stringify(report, null, 2)}\n`,
  );
}

const summary = Object.fromEntries(
  Object.entries(report.mismatches).map(([kind, candidates]) => [kind, candidates.length]),
);
console.log(JSON.stringify({ ...report, mismatches: summary }, null, 2));
if (
  !process.argv.includes("--allow-mismatches")
  && Object.values(summary).some(count => count > 0)
) {
  throw new Error("Property Value Matrix does not match the Chromium baseline");
}
