import assert from "node:assert/strict";
import { test } from "vitest";

import { CSSStyleRule, CSSStyleSheet } from "../src/index.js";

test("named CSS properties use the same live declaration state", () => {
  const sheet = new CSSStyleSheet();
  sheet.insertRule(".x {}");

  const rule = sheet.cssRules[0];
  assert.ok(rule instanceof CSSStyleRule);

  Reflect.set(rule.style, "backgroundColor", "red");
  Reflect.set(rule.style, "paddingLeft", "2px");

  assert.equal(Reflect.get(rule.style, "backgroundColor"), "red");
  assert.equal(rule.style.getPropertyValue("background-color"), "red");
  assert.equal(rule.style.getPropertyValue("padding-left"), "2px");
  assert.deepEqual(
    Array.from({ length: rule.style.length }, (_, index) => rule.style[index]),
    ["background-color", "padding-left"],
  );
});
