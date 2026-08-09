import assert from "node:assert/strict";
import { test } from "vitest";

import shorthandCapabilities from "../compatibility/shorthand-capabilities.json" with { type: "json" };
import { chromiumShorthandLonghands } from "../src/chromium-properties.js";
import { CSSStyleRule, CSSStyleSheet } from "../src/index.js";
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

test("every concrete shorthand capability passes expansion and synthesis", () => {
  for (const capability of shorthandCapabilities.cases) {
    const sheet = new CSSStyleSheet();
    sheet.insertRule(".x {}");
    const rule = sheet.cssRules[0];
    assert.ok(rule instanceof CSSStyleRule);
    rule.style.setProperty(capability.property, capability.input);

    assert.deepEqual(
      Array.from({ length: rule.style.length }, (_, index) => rule.style.item(index)),
      capability.chromium.items,
      capability.property,
    );
    assert.deepEqual(
      capability.chromium.items.map(name => ({
        name,
        value: rule.style.getPropertyValue(name),
        priority: rule.style.getPropertyPriority(name),
      })),
      capability.chromium.longhands,
      capability.property,
    );
    assert.equal(
      rule.style.getPropertyValue(capability.property),
      capability.chromium.shorthandValue,
      capability.property,
    );
    assert.equal(
      rule.style.getPropertyPriority(capability.property),
      capability.chromium.shorthandPriority,
      capability.property,
    );
    assert.equal(rule.style.cssText, capability.chromium.cssText, capability.property);
  }
});

test("every concrete shorthand capability round-trips through safe serialization", () => {
  for (const capability of shorthandCapabilities.cases) {
    const sheet = new CSSStyleSheet();
    sheet.insertRule(".x {}");
    const rule = sheet.cssRules[0];
    assert.ok(rule instanceof CSSStyleRule);
    rule.style.setProperty(capability.property, capability.input);

    const serialized = sheet.serialize();
    const reparsed = new CSSStyleSheet();
    reparsed.replaceSync(serialized);
    const reparsedRule = reparsed.cssRules[0];
    assert.ok(reparsedRule instanceof CSSStyleRule, capability.property);
    assert.equal(reparsed.serialize(), serialized, capability.property);
    assert.deepEqual(
      Array.from(
        { length: reparsedRule.style.length },
        (_, index) => reparsedRule.style.item(index),
      ),
      capability.chromium.items,
      capability.property,
    );
  }
});

test("every concrete shorthand capability is atomic and cannot resurrect", () => {
  for (const capability of shorthandCapabilities.cases) {
    const sheet = new CSSStyleSheet();
    sheet.insertRule(".x {}");
    const rule = sheet.cssRules[0];
    assert.ok(rule instanceof CSSStyleRule);
    rule.style.cssText = `${capability.property}: ${capability.input};`;
    const accepted = rule.style.cssText;

    rule.style.setProperty(
      capability.property,
      `${capability.input}; color: sheetom-invalid`,
    );
    assert.equal(rule.style.cssText, accepted, capability.property);

    rule.style.setProperty(
      capability.mutationProbe.longhand,
      capability.mutationProbe.override,
    );
    assert.equal(rule.style.getPropertyValue(capability.property), "", capability.property);
    rule.style.removeProperty(capability.mutationProbe.longhand);
    assert.equal(rule.style.getPropertyValue(capability.property), "", capability.property);
    assert.equal(
      Array.from({ length: rule.style.length }, (_, index) => rule.style.item(index))
        .includes(capability.property),
      false,
      capability.property,
    );
  }
});
