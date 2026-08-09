import assert from "node:assert/strict";
import { test } from "vitest";

import { CSSStyleRule, CSSStyleSheet } from "../src/index.js";

test("padding expands into indexed longhands and serializes opportunistically", () => {
  const sheet = new CSSStyleSheet();
  sheet.insertRule(".x {}");

  const rule = sheet.cssRules[0];
  assert.ok(rule instanceof CSSStyleRule);

  rule.style.setProperty("padding", "1px 2px");

  assert.equal(rule.style.length, 4);
  assert.deepEqual(
    Array.from({ length: rule.style.length }, (_, index) => rule.style.item(index)),
    ["padding-top", "padding-right", "padding-bottom", "padding-left"],
  );
  assert.equal(rule.style.getPropertyValue("padding"), "1px 2px");
  assert.equal(rule.style.cssText, "padding: 1px 2px;");

  rule.style.setProperty("padding-left", "3px");
  assert.equal(rule.style.getPropertyValue("padding"), "1px 2px 1px 3px");
  assert.equal(rule.style.cssText, "padding: 1px 2px 1px 3px;");

  assert.equal(rule.style.removeProperty("padding-left"), "3px");
  assert.equal(rule.style.getPropertyValue("padding"), "");
  assert.equal(
    rule.style.cssText,
    "padding-top: 1px; padding-right: 2px; padding-bottom: 1px;",
  );
});

test("common four-side shorthands share expanded record behavior", () => {
  const rule = new CSSStyleRule(".x");
  rule.style.setProperty("margin", "1px 2px");

  assert.deepEqual(
    Array.from({ length: rule.style.length }, (_, index) => rule.style[index]),
    ["margin-top", "margin-right", "margin-bottom", "margin-left"],
  );
  assert.equal(rule.style.getPropertyValue("margin"), "1px 2px");
  assert.equal(rule.style.cssText, "margin: 1px 2px;");

  rule.style.setProperty("margin-left", "3px");
  assert.equal(rule.style.cssText, "margin: 1px 2px 1px 3px;");
});
