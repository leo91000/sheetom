import assert from "node:assert/strict";
import { test } from "vitest";

import { CSSStyleRule, CSSStyleSheet } from "../src/index.js";

test("custom properties preserve case and expose empty-but-present entries", () => {
  const sheet = new CSSStyleSheet();
  sheet.insertRule(".x {}");

  const rule = sheet.cssRules[0];
  assert.ok(rule instanceof CSSStyleRule);

  rule.style.setProperty("--X", "false");
  rule.style.setProperty("--x", " ");

  assert.equal(rule.style.length, 2);
  assert.equal(rule.style[0], "--X");
  assert.equal(rule.style[1], "--x");
  assert.equal(rule.style.getPropertyValue("--X"), "false");
  assert.equal(rule.style.getPropertyValue("--x"), "");
  assert.equal(rule.style.cssText, "--X: false; --x: ;");
});

test("declaration-block parsing preserves empty custom-property records", () => {
  const rule = new CSSStyleRule(".x");
  rule.style.cssText = "--empty: ; --flag: false;";

  assert.equal(rule.style.length, 2);
  assert.equal(rule.style[0], "--empty");
  assert.equal(rule.style.getPropertyValue("--empty"), "");
  assert.equal(rule.style.cssText, "--empty: ; --flag: false;");
});

test("CSSOM custom-property names retain logical text and serialize as identifiers", () => {
  const rule = new CSSStyleRule(".x");
  rule.style.setProperty("-- x", "red");
  rule.style.setProperty("--x!", "blue");
  rule.style.setProperty("--", "green");

  assert.equal(rule.style.length, 2);
  assert.equal(rule.style[0], "-- x");
  assert.equal(rule.style[1], "--x!");
  assert.equal(rule.style.getPropertyValue("-- x"), "red");
  assert.equal(rule.style.cssText, "--\\ x: red; --x\\!: blue;");
});
