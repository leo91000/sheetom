import assert from "node:assert/strict";
import { test } from "vitest";

import { CSSStyleRule, CSSStyleSheet, parseStyleSheet } from "../src/index.js";

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

test("legacy property aliases share the browser-canonical declaration", () => {
  const sheet = new CSSStyleSheet();
  sheet.insertRule(".x {}");

  const rule = sheet.cssRules[0];
  assert.ok(rule instanceof CSSStyleRule);
  rule.style.setProperty("-webkit-transform", "none");

  assert.equal(rule.style.length, 1);
  assert.equal(rule.style.item(0), "transform");
  assert.equal(rule.style.getPropertyValue("-webkit-transform"), "none");
  assert.equal(rule.style.getPropertyValue("transform"), "none");
  assert.equal(rule.style.cssText, "transform: none;");
  assert.match(sheet.serialize(), /transform: none/);
});

test("vendor property values use the Chromium compatibility baseline", () => {
  const sheet = new CSSStyleSheet();
  sheet.insertRule(".x {}");

  const rule = sheet.cssRules[0];
  assert.ok(rule instanceof CSSStyleRule);
  rule.style.setProperty("-webkit-box-reflect", "below");

  assert.equal(rule.style.getPropertyValue("-webkit-box-reflect"), "below 0px");
  assert.equal(rule.style.cssText, "-webkit-box-reflect: below 0px;");
  assert.match(sheet.serialize(), /-webkit-box-reflect: below 0px/);
});

test("stylesheet parsing decodes ordinary escaped property names", () => {
  const sheet = parseStyleSheet(".x { w\\69 dth: 1px; }");
  const rule = sheet.cssRules[0];

  assert.ok(rule instanceof CSSStyleRule);
  assert.equal(rule.style.length, 1);
  assert.equal(rule.style.item(0), "width");
  assert.equal(rule.style.getPropertyValue("width"), "1px");
  assert.equal(rule.style.cssText, "width: 1px;");
});
