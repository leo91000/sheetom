import assert from "node:assert/strict";
import { test } from "vitest";

import contracts from "../compatibility/browser-geometric-contracts.json" with { type: "json" };
import { CSSStyleRule, CSSStyleSheet, parseStyleSheet } from "../src/index.js";
import { createStyleRule } from "./support/create-style-rule.js";

test("every reviewed geometric branch survives mutation and safe round trips", () => {
  for (const { property, branches } of contracts.properties) {
    for (const branch of branches) {
      const rule = createStyleRule(".geometry");
      rule.style.setProperty(property, branch.input, "important");

      assert.notEqual(
        rule.style.getPropertyValue(property),
        "",
        `${property}.${branch.id} must be accepted`,
      );
      assert.equal(rule.style.getPropertyPriority(property), "important");

      const beforeInvalid = rule.style.cssText;
      rule.style.setProperty(property, branch.invalidNeighbor, "important");
      assert.equal(
        rule.style.cssText,
        beforeInvalid,
        `${property}.${branch.id} invalid replacement must be atomic`,
      );

      const sheet = rule.parentStyleSheet;
      assert.ok(sheet);
      const serialized = sheet.serialize();
      const reparsed = parseStyleSheet(serialized);
      assert.equal(
        reparsed.serialize(),
        serialized,
        `${property}.${branch.id} must serialize idempotently`,
      );
      const reparsedRule = reparsed.cssRules[0];
      assert.ok(reparsedRule instanceof CSSStyleRule);
      assert.equal(
        reparsedRule.style.getPropertyValue(property),
        rule.style.getPropertyValue(property),
        `${property}.${branch.id} observable value must survive`,
      );
    }
  }
});

test("geometric properties retain malformed pending substitutions like Chromium", () => {
  const sheet = new CSSStyleSheet();
  sheet.insertRule(".geometry {}", 0);
  const rule = sheet.cssRules[0];
  assert.ok(rule instanceof CSSStyleRule);

  for (const [property, value] of [
    ["border-shape", "var(--border"],
    ["d", "var(--path"],
    ["object-view-box", "var(--view"],
    ["shape-outside", "var(--shape"],
  ] as const) {
    rule.style.cssText = "";
    rule.style.setProperty(property, value);
    assert.equal(rule.style.getPropertyValue(property), value);
    assert.equal(rule.style.cssText, `${property}: ${value};`);

    const serialized = sheet.serialize();
    const reparsed = parseStyleSheet(serialized);
    assert.equal(reparsed.serialize(), serialized);
  }
});

test("removing a geometric property returns its canonical value and clears state", () => {
  const style = createStyleRule(".geometry").style;
  style.setProperty("d", 'path("M0 0 10 10")');
  assert.equal(style.length, 1);
  assert.equal(style.item(0), "d");
  assert.equal(style.removeProperty("d"), 'path("M 0 0 L 10 10")');
  assert.equal(style.length, 0);
  assert.equal(style.getPropertyValue("d"), "");
});
