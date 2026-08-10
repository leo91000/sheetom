import assert from "node:assert/strict";
import { readFile, writeFile } from "node:fs/promises";
import path from "node:path";

import { CSSStyleRule, CSSStyleSheet } from "../dist/index.js";

const capabilities = JSON.parse(
  await readFile(new URL("../compatibility/shorthand-capabilities.json", import.meta.url)),
);
const contracts = JSON.parse(
  await readFile(new URL("../compatibility/shorthand-grammar-contracts.json", import.meta.url)),
);
const observations = JSON.parse(
  await readFile(new URL("../compatibility/shorthand-grammar-observations.json", import.meta.url)),
);
const nativeInventory = JSON.parse(
  await readFile(new URL("../compatibility/native-grammar-inventory.json", import.meta.url)),
);
const valueCapabilities = JSON.parse(
  await readFile(new URL("../compatibility/value-capabilities.json", import.meta.url)),
);
const relativeColorCapabilities = JSON.parse(
  await readFile(new URL("../compatibility/relative-color-capabilities.json", import.meta.url)),
);
const observationsById = new Map(observations.cases.map(candidate => [candidate.id, candidate]));
const contractCasesById = new Map(
  [
    ...contracts.profiles.flatMap(profile => profile.cases),
    ...nativeInventory.propertyBranches,
  ].map(candidate => [candidate.id, candidate]),
);
const failures = [];

function capture(id, operation) {
  try {
    operation();
  } catch (error) {
    failures.push(`${id}: ${error instanceof Error ? error.message : error}`);
  }
}

function createRule() {
  const sheet = new CSSStyleSheet();
  sheet.insertRule(".x {}");
  const rule = sheet.cssRules[0];
  assert.ok(rule instanceof CSSStyleRule);
  return { sheet, rule };
}

function state(rule) {
  const items = Array.from({ length: rule.style.length }, (_, index) => rule.style.item(index));
  return {
    items,
    longhands: items.map(name => ({
      name,
      value: rule.style.getPropertyValue(name),
      priority: rule.style.getPropertyPriority(name),
    })),
  };
}

for (const candidate of capabilities.cases) {
  capture(candidate.id, () => {
    const { sheet, rule } = createRule();
    rule.style.setProperty(candidate.property, candidate.input);
    const actual = state(rule);
    assert.deepEqual(actual.items, candidate.chromium.items);
    assert.deepEqual(actual.longhands, candidate.chromium.longhands);
    assert.equal(rule.style.getPropertyValue(candidate.property), candidate.chromium.shorthandValue);
    assert.equal(rule.style.getPropertyPriority(candidate.property), candidate.chromium.shorthandPriority);
    assert.equal(rule.style.cssText, candidate.chromium.cssText);
    const serialized = sheet.serialize();
    const reparsed = new CSSStyleSheet();
    reparsed.replaceSync(serialized);
    assert.equal(reparsed.serialize(), serialized);
  });
}

for (const profile of contracts.profiles) {
  for (const candidate of profile.cases) {
    capture(candidate.id, () => {
      const expected = observationsById.get(candidate.id);
      assert.equal(expected?.accepted, candidate.accepted);
      const { sheet, rule } = createRule();

      if (!candidate.accepted) {
        const preserved = contractCasesById.get(candidate.preserves);
        assert.ok(preserved?.accepted, `${candidate.preserves} must name an accepted case`);
        rule.style.setProperty(preserved.property, preserved.input);
        const before = {
          state: state(rule),
          shorthandValue: rule.style.getPropertyValue(preserved.property),
          shorthandPriority: rule.style.getPropertyPriority(preserved.property),
          cssText: rule.style.cssText,
          serialized: sheet.serialize(),
        };
        rule.style.setProperty(candidate.property, candidate.input);
        assert.deepEqual(state(rule), before.state);
        assert.equal(rule.style.getPropertyValue(preserved.property), before.shorthandValue);
        assert.equal(rule.style.getPropertyPriority(preserved.property), before.shorthandPriority);
        assert.equal(rule.style.cssText, before.cssText);
        assert.equal(sheet.serialize(), before.serialized);
        return;
      }

      rule.style.setProperty(candidate.property, candidate.input);
      const actual = state(rule);
      assert.deepEqual(actual.items, expected.items);
      assert.deepEqual(actual.longhands, expected.longhands);
      assert.equal(rule.style.getPropertyValue(candidate.property), expected.shorthandValue);
      assert.equal(rule.style.getPropertyPriority(candidate.property), expected.priority);
      assert.equal(rule.style.cssText, expected.cssText);
      const serialized = sheet.serialize();
      const reparsed = new CSSStyleSheet();
      reparsed.replaceSync(serialized);
      assert.equal(reparsed.serialize(), serialized);
    });
  }
}

for (const candidate of nativeInventory.propertyBranches) {
  capture(candidate.id, () => {
    const { sheet, rule } = createRule();
    if (!candidate.accepted) {
      const preserved = contractCasesById.get(candidate.preserves);
      assert.ok(preserved?.accepted, `${candidate.preserves} must name an accepted case`);
      rule.style.setProperty(preserved.property, preserved.input);
      const before = {
        state: state(rule),
        shorthandValue: rule.style.getPropertyValue(preserved.property),
        shorthandPriority: rule.style.getPropertyPriority(preserved.property),
        cssText: rule.style.cssText,
        serialized: sheet.serialize(),
      };
      rule.style.setProperty(candidate.property, candidate.input);
      assert.deepEqual(state(rule), before.state);
      assert.equal(rule.style.getPropertyValue(preserved.property), before.shorthandValue);
      assert.equal(rule.style.getPropertyPriority(preserved.property), before.shorthandPriority);
      assert.equal(rule.style.cssText, before.cssText);
      assert.equal(sheet.serialize(), before.serialized);
      return;
    }

    rule.style.setProperty(candidate.property, candidate.input);
    const actual = state(rule);
    assert.deepEqual(actual.items, candidate.chromium.items);
    assert.deepEqual(actual.longhands, candidate.chromium.longhands);
    assert.equal(rule.style.getPropertyValue(candidate.property), candidate.chromium.shorthandValue);
    assert.equal(rule.style.getPropertyPriority(candidate.property), candidate.chromium.priority);
    assert.equal(rule.style.cssText, candidate.chromium.cssText);
    const serialized = sheet.serialize();
    const reparsed = new CSSStyleSheet();
    reparsed.replaceSync(serialized);
    assert.equal(reparsed.serialize(), serialized);
  });
}

for (const candidate of valueCapabilities.cases) {
  capture(candidate.id, () => {
    const { sheet, rule } = createRule();
    if (!candidate.accepted) {
      rule.style.setProperty(candidate.property, "initial");
      const before = {
        state: state(rule),
        value: rule.style.getPropertyValue(candidate.property),
        cssText: rule.style.cssText,
        serialized: sheet.serialize(),
      };
      rule.style.setProperty(candidate.property, candidate.input);
      assert.deepEqual(state(rule), before.state);
      assert.equal(rule.style.getPropertyValue(candidate.property), before.value);
      assert.equal(rule.style.cssText, before.cssText);
      assert.equal(sheet.serialize(), before.serialized);
      return;
    }

    rule.style.setProperty(candidate.property, candidate.input);
    assert.equal(rule.style.getPropertyValue(candidate.property), candidate.observable);
    const serialized = sheet.serialize();
    const reparsed = new CSSStyleSheet();
    reparsed.replaceSync(serialized);
    assert.equal(reparsed.serialize(), serialized);
  });
}

for (const candidate of relativeColorCapabilities.cases) {
  capture(candidate.id, () => {
    const { sheet, rule } = createRule();
    if (!candidate.chromiumAccepted) {
      rule.style.setProperty(candidate.property, "red");
      const before = {
        state: state(rule),
        value: rule.style.getPropertyValue(candidate.property),
        cssText: rule.style.cssText,
        serialized: sheet.serialize(),
      };
      rule.style.setProperty(candidate.property, candidate.input);
      assert.deepEqual(state(rule), before.state);
      assert.equal(rule.style.getPropertyValue(candidate.property), before.value);
      assert.equal(rule.style.cssText, before.cssText);
      assert.equal(sheet.serialize(), before.serialized);
      return;
    }

    rule.style.setProperty(candidate.property, candidate.input);
    assert.equal(
      rule.style.getPropertyValue(candidate.property),
      candidate.chromiumObservable,
    );
    const serialized = sheet.serialize();
    const reparsed = new CSSStyleSheet();
    reparsed.replaceSync(serialized);
    assert.equal(reparsed.serialize(), serialized);
  });
}

if (failures.length > 0) {
  throw new Error(`Native public corpus failures (${failures.length}):\n${failures.join("\n")}`);
}

const grammarCases = contracts.profiles.flatMap(profile => profile.cases);
const positiveGrammarCases = grammarCases.filter(candidate => candidate.accepted).length;
const negativeGrammarCases = grammarCases.length - positiveGrammarCases;
const positivePropertyBranches = nativeInventory.propertyBranches
  .filter(candidate => candidate.accepted).length;
const negativePropertyBranches = nativeInventory.propertyBranches.length
  - positivePropertyBranches;
const positiveValueCapabilities = valueCapabilities.cases
  .filter(candidate => candidate.accepted).length;
const negativeValueCapabilities = valueCapabilities.cases.length
  - positiveValueCapabilities;
const reportArgument = process.argv.find(argument => argument.startsWith("--report="));
if (reportArgument) {
  const reportPath = path.resolve(reportArgument.slice("--report=".length));
  await writeFile(reportPath, `${JSON.stringify({
    schemaVersion: 1,
    shorthandProperties: {
      passed: capabilities.cases.length,
      total: capabilities.cases.length,
    },
    grammarBranches: {
      passed: grammarCases.length,
      total: grammarCases.length,
      positive: positiveGrammarCases,
      negative: negativeGrammarCases,
    },
    propertyBranches: {
      passed: nativeInventory.propertyBranches.length,
      total: nativeInventory.propertyBranches.length,
      positive: positivePropertyBranches,
      negative: negativePropertyBranches,
    },
    valueCapabilities: {
      passed: valueCapabilities.cases.length,
      total: valueCapabilities.cases.length,
      positive: positiveValueCapabilities,
      negative: negativeValueCapabilities,
    },
    relativeColors: {
      passed: relativeColorCapabilities.cases.length,
      total: relativeColorCapabilities.cases.length,
      positive: relativeColorCapabilities.cases
        .filter(candidate => candidate.chromiumAccepted).length,
      negative: relativeColorCapabilities.cases
        .filter(candidate => !candidate.chromiumAccepted).length,
    },
  }, null, 2)}\n`);
}

console.log(
  `Verified ${capabilities.cases.length} shorthand capabilities and ` +
    `${positiveGrammarCases} positive plus ` +
    `${negativeGrammarCases} atomic rejection branches and ` +
    `${nativeInventory.propertyBranches.length} property plus ` +
    `${valueCapabilities.cases.length} value and ` +
    `${relativeColorCapabilities.cases.length} relative-color branches.`,
);
