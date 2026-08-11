import { createHash } from "node:crypto";
import { readFile, writeFile } from "node:fs/promises";
import path from "node:path";

import corpus from "../compatibility/webref-property-branches.json" with { type: "json" };
import { CSSStyleRule, CSSStyleSheet } from "../dist/index.js";

const corpusUrl = new URL("../compatibility/webref-property-branches.json", import.meta.url);
const ratchetUrl = new URL("../compatibility/webref-branch-ratchet.json", import.meta.url);
const corpusBytes = await readFile(corpusUrl);

const expectedByKey = new Map(corpus.accepted.map(observation => [
  `${observation[1]}\0${observation[2]}`,
  observation,
]));
const seedByProperty = new Map(corpus.seeds.map(seed => [seed[0], seed]));
const mismatches = {
  acceptance: [],
  observable: [],
  cssText: [],
  items: [],
  atomicity: [],
  reparse: [],
};

function createRule() {
  const sheet = new CSSStyleSheet();
  sheet.insertRule(".webref-probe {}");
  const rule = sheet.cssRules[0];
  if (!(rule instanceof CSSStyleRule)) throw new TypeError("Expected a style rule");
  return rule;
}

function state(style) {
  return {
    cssText: style.cssText,
    items: Array.from({ length: style.length }, (_, index) => {
      const name = style.item(index);
      return [
        name,
        style.getPropertyValue(name),
        style.getPropertyPriority(name),
      ];
    }),
  };
}

let checkCount = 0;
for (const profile of corpus.profiles) {
  for (const property of profile.properties) {
    for (const sample of profile.samples) {
      checkCount += 1;
      const key = `${property}\0${sample.id}`;
      const expected = expectedByKey.get(key);
      const rule = createRule();
      rule.style.setProperty(property, sample.input);
      const actualAccepted = rule.style.length > 0;
      if (actualAccepted !== Boolean(expected)) {
        mismatches.acceptance.push({
          profile: profile.id,
          property,
          sample: sample.id,
          input: sample.input,
          expected: Boolean(expected),
          actual: actualAccepted,
        });
        if (expected) continue;
      }
      if (!expected) {
        const atomicRule = createRule();
        const seed = seedByProperty.get(property);
        if (!seed) throw new Error(`Missing Webref atomicity seed for ${property}`);
        atomicRule.style.setProperty(property, seed[1]);
        const before = state(atomicRule.style);
        if (before.items.length === 0) {
          throw new Error(`SheetOM rejected the Webref atomicity seed for ${property}`);
        }
        atomicRule.style.setProperty(property, sample.input);
        const after = state(atomicRule.style);
        if (JSON.stringify(before) !== JSON.stringify(after)) {
          mismatches.atomicity.push({
            property,
            sample: sample.id,
            input: sample.input,
            before,
            after,
          });
        }
        continue;
      }

      const [, , , expectedObservable, expectedCssText, expectedItems, invalidNeighbor] = expected;
      const actualObservable = rule.style.getPropertyValue(property);
      if (actualObservable !== expectedObservable) {
        mismatches.observable.push({
          property,
          sample: sample.id,
          input: sample.input,
          expected: expectedObservable,
          actual: actualObservable,
        });
      }
      if (rule.style.cssText !== expectedCssText) {
        mismatches.cssText.push({
          property,
          sample: sample.id,
          input: sample.input,
          expected: expectedCssText,
          actual: rule.style.cssText,
        });
      }
      const actualItems = state(rule.style).items;
      if (JSON.stringify(actualItems) !== JSON.stringify(expectedItems)) {
        mismatches.items.push({
          property,
          sample: sample.id,
          input: sample.input,
          expected: expectedItems,
          actual: actualItems,
        });
      }

      const before = state(rule.style);
      rule.style.setProperty(property, invalidNeighbor);
      const after = state(rule.style);
      if (JSON.stringify(before) !== JSON.stringify(after)) {
        mismatches.atomicity.push({
          property,
          sample: sample.id,
          input: invalidNeighbor,
          before,
          after,
        });
      }

      const serialized = rule.parentStyleSheet?.serialize();
      if (!serialized) {
        mismatches.reparse.push({ property, sample: sample.id, error: "empty serialization" });
        continue;
      }
      try {
        const reparsed = new CSSStyleSheet();
        reparsed.replaceSync(serialized);
        if (reparsed.serialize() !== serialized) {
          mismatches.reparse.push({
            property,
            sample: sample.id,
            error: "serialization is not idempotent",
          });
        }
      } catch (error) {
        mismatches.reparse.push({
          property,
          sample: sample.id,
          error: error instanceof Error ? error.message : "unknown reparse error",
        });
      }
    }
  }
}

const report = {
  schemaVersion: 1,
  checks: ["acceptance", "observable", "cssText", "items", "atomicity", "reparse"],
  properties: corpus.coverage.webrefProperties,
  profiles: corpus.coverage.profiles,
  branches: corpus.coverage.branches,
  checksRun: checkCount,
  expectedAccepted: corpus.coverage.accepted,
  mismatches,
};
const reportArgument = process.argv.find(argument => argument.startsWith("--report="));
if (reportArgument) {
  await writeFile(
    path.resolve(reportArgument.slice("--report=".length)),
    `${JSON.stringify(report, null, 2)}\n`,
  );
}

const summary = Object.fromEntries(
  Object.entries(mismatches).map(([kind, values]) => [kind, values.length]),
);
console.log(JSON.stringify({ ...report, mismatches: summary }, null, 2));

const casesByKey = new Map();
for (const [kind, values] of Object.entries(mismatches)) {
  for (const mismatch of values) {
    const key = `${mismatch.property}\0${mismatch.sample}`;
    const existing = casesByKey.get(key) ?? {
      property: mismatch.property,
      sample: mismatch.sample,
      kinds: [],
    };
    existing.kinds.push(kind);
    casesByKey.set(key, existing);
  }
}
const ratchet = {
  "$schema": "./schemas/webref-branch-ratchet.schema.json",
  schemaVersion: 1,
  corpusSha256: createHash("sha256").update(corpusBytes).digest("hex"),
  mismatchCases: casesByKey.size,
  summary,
  unresolved: [...casesByKey.values()]
    .map(entry => ({ ...entry, kinds: entry.kinds.sort() }))
    .sort((left, right) =>
      left.property.localeCompare(right.property)
      || left.sample.localeCompare(right.sample)),
};
const serializedRatchet = `${JSON.stringify(ratchet, null, 2)}\n`;
if (process.argv.includes("--record-ratchet")) {
  await writeFile(ratchetUrl, serializedRatchet);
} else if (!process.argv.includes("--allow-mismatches")) {
  const expectedRatchet = await readFile(ratchetUrl, "utf8");
  if (expectedRatchet !== serializedRatchet) {
    throw new Error(
      "Webref mismatch ratchet changed; regressions are forbidden and fixes must be "
      + "reviewed with --record-ratchet",
    );
  }
}
if (process.argv.includes("--strict") && casesByKey.size > 0) {
  throw new Error("SheetOM does not yet close every Webref Chromium branch");
}
