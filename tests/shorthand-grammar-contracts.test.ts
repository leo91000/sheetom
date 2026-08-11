import assert from "node:assert/strict";
import { test } from "vitest";

import contracts from "../compatibility/shorthand-grammar-contracts.json" with { type: "json" };
import inventory from "../compatibility/native-grammar-inventory.json" with { type: "json" };
import observations from "../compatibility/shorthand-grammar-observations.json" with { type: "json" };
import { CSSStyleRule, CSSStyleSheet } from "../src/index.js";

function createRule(): CSSStyleRule {
  const sheet = new CSSStyleSheet();
  sheet.insertRule(".x {}");
  const rule = sheet.cssRules[0];
  assert.ok(rule instanceof CSSStyleRule);
  return rule;
}

test("every native coverage profile has distinct positive and negative grammar branches", () => {
  const coverageProfiles = new Set(inventory.properties.map(property => property.codec));
  assert.equal(coverageProfiles.size, 24);
  assert.deepEqual(
    new Set(contracts.profiles.map(profile => profile.codec)),
    coverageProfiles,
  );
  for (const profile of contracts.profiles) {
    assert.ok(profile.cases.length >= 4, profile.codec);
    assert.equal(
      new Set(profile.cases.map(grammarCase => grammarCase.branch)).size,
      profile.cases.length,
      profile.codec,
    );
    assert.ok(profile.cases.some(grammarCase => grammarCase.accepted), profile.codec);
    assert.ok(profile.cases.some(grammarCase => !grammarCase.accepted), profile.codec);
  }
});

test("every reviewed positive branch matches the pinned Chromium observation", () => {
  const byId = new Map(observations.cases.map(observation => [observation.id, observation]));
  for (const grammarCase of contracts.profiles.flatMap(profile => profile.cases)) {
    if (!grammarCase.accepted) continue;
    const expected = byId.get(grammarCase.id);
    assert.ok(expected, grammarCase.id);
    const rule = createRule();
    rule.style.setProperty(grammarCase.property, grammarCase.input);
    assert.deepEqual(
      Array.from({ length: rule.style.length }, (_, index) => rule.style.item(index)),
      expected.items,
      grammarCase.id,
    );
    assert.deepEqual(
      expected.items.map(name => ({
        name,
        value: rule.style.getPropertyValue(name),
        priority: rule.style.getPropertyPriority(name),
      })),
      expected.longhands,
      grammarCase.id,
    );
    assert.equal(
      rule.style.getPropertyValue(grammarCase.property),
      expected.shorthandValue,
      grammarCase.id,
    );
    assert.equal(rule.style.getPropertyPriority(grammarCase.property), expected.priority);
    assert.equal(rule.style.cssText, expected.cssText, grammarCase.id);

    const serialized = rule.parentStyleSheet?.serialize();
    assert.ok(serialized, grammarCase.id);
    const reparsed = new CSSStyleSheet();
    reparsed.replaceSync(serialized);
    assert.equal(reparsed.serialize(), serialized, grammarCase.id);
  }
});

test("every reviewed negative branch is an atomic no-op", () => {
  const cases = contracts.profiles.flatMap(profile => profile.cases);
  const byId = new Map(cases.map(grammarCase => [grammarCase.id, grammarCase]));
  for (const grammarCase of cases) {
    if (grammarCase.accepted) continue;
    assert.ok("preserves" in grammarCase, grammarCase.id);
    const accepted = byId.get(grammarCase.preserves);
    assert.ok(accepted?.accepted, grammarCase.id);
    assert.equal(accepted.property, grammarCase.property, grammarCase.id);
    const rule = createRule();
    rule.style.setProperty(accepted.property, accepted.input);
    const before = rule.style.cssText;
    rule.style.setProperty(grammarCase.property, grammarCase.input);
    assert.equal(rule.style.cssText, before, grammarCase.id);
  }
});
