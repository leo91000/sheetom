import assert from "node:assert/strict";
import { test } from "vitest";

import { CSSStyleRule, CSSStyleSheet } from "../src/index.js";

test("setProperty applies browser string conversion before validation", () => {
  const sheet = new CSSStyleSheet();
  sheet.insertRule(".x {}");

  const rule = sheet.cssRules[0];
  assert.ok(rule instanceof CSSStyleRule);

  Reflect.apply(rule.style.setProperty, rule.style, ["width", 0]);
  assert.equal(rule.style.getPropertyValue("width"), "0px");

  Reflect.apply(rule.style.setProperty, rule.style, ["--flag", false]);
  assert.equal(rule.style.getPropertyValue("--flag"), "false");

  Reflect.apply(rule.style.setProperty, rule.style, ["width", undefined]);
  assert.equal(rule.style.getPropertyValue("width"), "0px");

  Reflect.apply(rule.style.setProperty, rule.style, ["width", null]);
  assert.equal(rule.style.getPropertyValue("width"), "");
});

test("CSSOM methods apply WebIDL conversions to names and indices", () => {
  const sheet = new CSSStyleSheet();
  sheet.insertRule(".first { width: 1px; color: red; }");
  sheet.insertRule(".second {}", 1);

  const first = sheet.cssRules[0];
  assert.ok(first instanceof CSSStyleRule);
  assert.equal(Reflect.apply(first.style.item, first.style, [1.9]), "color");
  assert.equal(
    Reflect.apply(first.style.getPropertyValue, first.style, [null]),
    "",
  );

  Reflect.apply(sheet.deleteRule, sheet, [Number.NaN]);
  assert.equal(sheet.cssRules.length, 1);
  assert.equal(sheet.cssRules[0]?.cssText, ".second { }");
});
