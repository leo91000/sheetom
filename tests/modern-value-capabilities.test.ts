import assert from "node:assert/strict";
import { test } from "vitest";

import { CSSStyleRule, CSSStyleSheet } from "../src/index.js";
import { chromiumShorthandLonghands } from "../src/chromium-properties.js";
import { createStyleRule } from "./support/create-style-rule.js";
import valueCapabilities from "../compatibility/value-capabilities.json" with { type: "json" };

test("measured modern value families are not dropped by parser fallbacks", () => {
  for (const candidate of valueCapabilities.cases) {
    if (!candidate.accepted) continue;
    const rule = createStyleRule(".x");
    const style = rule.style;
    style.setProperty(candidate.property, candidate.input);
    assert.equal(
      style.getPropertyValue(candidate.property),
      candidate.observable,
      candidate.id,
    );
    const sheet = rule.parentStyleSheet;
    assert.ok(sheet);
    const serialized = sheet.serialize();
    const reparsed = new CSSStyleSheet();
    reparsed.replaceSync(serialized);
    const reparsedRule = reparsed.cssRules[0];
    assert.ok(reparsedRule instanceof CSSStyleRule);
    const reparsedItems = new Set(Array.from(
      { length: reparsedRule.style.length },
      (_, index) => reparsedRule.style.item(index),
    ));
    const expectedItems = chromiumShorthandLonghands[candidate.property]
      ?? [candidate.property];
    assert.ok(
      expectedItems.every(property => reparsedItems.has(property)),
      `${candidate.id} survives reparsing`,
    );
    assert.equal(reparsed.serialize(), serialized, `${candidate.id} is idempotent`);
  }
});

test("neighboring invalid capability cases are atomic no-ops", () => {
  const style = createStyleRule(".x").style;
  for (const candidate of valueCapabilities.cases) {
    if (candidate.accepted) continue;
    style.setProperty(candidate.property, "initial");
    style.setProperty(candidate.property, candidate.input);
    assert.equal(style.getPropertyValue(candidate.property), "initial", candidate.id);
  }
});

test("content recovery and feature support follow the Chromium baseline", () => {
  const style = createStyleRule(".x").style;

  style.setProperty("content", "var(--x");
  assert.equal(style.getPropertyValue("content"), "var(--x");

  style.setProperty("content", '"safe"');
  style.setProperty("content", "leader(.)");
  assert.equal(style.getPropertyValue("content"), '"safe"');

  style.setProperty("content", "target-text(url(#x))");
  assert.equal(style.getPropertyValue("content"), '"safe"');

  style.setProperty("content", "target-text(attr(href url))");
  assert.equal(style.getPropertyValue("content"), "target-text(attr(href url))");
});
