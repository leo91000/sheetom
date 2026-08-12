import { readFile, writeFile } from "node:fs/promises";
import path from "node:path";

import { CSSStyleRule, CSSStyleSheet } from "../dist/index.js";
import observations from "../compatibility/property-value-observations.json" with { type: "json" };
import grammarExtensions from "../compatibility/property-grammar-extensions.json" with { type: "json" };
import probes from "../compatibility/property-value-probes.json" with { type: "json" };
import { chromiumSupportedProperties } from "../src/chromium-properties.ts";

const expectedByKey = new Map(
  observations.accepted.map(([property, probe, observable, cssText, items]) => [
    `${property}\0${probe}`,
    { property, probe, observable, cssText, items },
  ]),
);
const contractArgument = process.argv.find(argument => argument.startsWith("--contract="));
const knownChecks = ["acceptance", "observable", "cssText", "items", "atomicity"];
const checksArgument = process.argv.find(argument => argument.startsWith("--checks="));
const checkedMismatchKinds = checksArgument
  ? checksArgument.slice("--checks=".length).split(",").filter(Boolean)
  : knownChecks;
const unknownChecks = checkedMismatchKinds.filter(check => !knownChecks.includes(check));
if (checkedMismatchKinds.length === 0 || unknownChecks.length > 0) {
  throw new Error(`Unknown Property Value Matrix checks: ${unknownChecks.join(", ")}`);
}
let propertiesToCheck = [...chromiumSupportedProperties];
let probesToCheck = probes.values;
let contractName = null;
if (contractArgument) {
  const contractPath = path.resolve(contractArgument.slice("--contract=".length));
  const contract = JSON.parse(await readFile(contractPath, "utf8"));
  contractName = path.relative(process.cwd(), contractPath);
  const families = new Map(grammarExtensions.families.map(family => [family.id, family]));
  const selectedProperties = new Set(contract.additionalProperties);
  for (const familyId of contract.extensionFamilies) {
    const family = families.get(familyId);
    if (!family) throw new Error(`Unknown Property Grammar extension family: ${familyId}`);
    for (const property of family.properties) selectedProperties.add(property);
  }
  propertiesToCheck = [...selectedProperties].sort();
  for (const property of propertiesToCheck) {
    if (!chromiumSupportedProperties.has(property)) {
      throw new Error(`Numeric contract references an unsupported property: ${property}`);
    }
  }
  const selectedProbeIds = new Set(contract.probes);
  probesToCheck = probes.values.filter(probe => selectedProbeIds.has(probe.id));
  if (probesToCheck.length !== selectedProbeIds.size) {
    const known = new Set(probesToCheck.map(probe => probe.id));
    const missing = [...selectedProbeIds].filter(probe => !known.has(probe));
    throw new Error(`Numeric contract references unknown probes: ${missing.join(", ")}`);
  }
}
const sheet = new CSSStyleSheet();
sheet.insertRule(".probe {}");
const rule = sheet.cssRules[0];
if (!(rule instanceof CSSStyleRule)) throw new TypeError("Expected a style rule");

const acceptance = [];
const observable = [];
const cssText = [];
const items = [];
const atomicity = [];
for (const property of propertiesToCheck) {
  for (const probe of probesToCheck) {
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
  ...(contractName ? { contract: contractName } : {}),
  checks: checkedMismatchKinds,
  properties: propertiesToCheck.length,
  probes: probesToCheck.length,
  expectedAccepted: propertiesToCheck.reduce(
    (count, property) => count + probesToCheck.filter(probe =>
      expectedByKey.has(`${property}\0${probe.id}`)).length,
    0,
  ),
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
  && checkedMismatchKinds.some(kind => summary[kind] > 0)
) {
  throw new Error(
    `${contractName ?? "Property Value Matrix"} does not match the Chromium baseline`,
  );
}
