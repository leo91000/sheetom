import assert from "node:assert/strict";
import { test } from "vitest";

import { CSSStyleRule, parseStyleSheet } from "../src/index.js";
import { createStyleRule } from "./support/create-style-rule.js";

test("hyphenate-limit-chars owns one to three auto, integer, and calculation components", () => {
  const style = createStyleRule(".hyphenate").style;
  for (const [input, expected] of [
    ["auto", "auto"],
    ["auto auto", "auto auto"],
    ["auto auto auto", "auto auto auto"],
    ["1 2 3", "1 2 3"],
    ["auto 2 3", "auto 2 3"],
    ["calc(1 + 1) auto calc(.5)", "calc(2) auto calc(0.5)"],
  ] as const) {
    style.setProperty("hyphenate-limit-chars", input);
    assert.equal(style.getPropertyValue("hyphenate-limit-chars"), expected, input);
  }

  for (const invalid of ["0", "1.5", "auto auto auto auto", "calc(1px)"]) {
    const beforeInvalid: string = style.cssText;
    style.setProperty("hyphenate-limit-chars", invalid);
    assert.equal(style.cssText, beforeInvalid, invalid);
  }
});

test("font math size expands, mutates, and round-trips like Chromium", () => {
  const rule = createStyleRule(".math");
  rule.style.setProperty("font", 'normal math / normal "sheetom"', "important");

  assert.equal(rule.style.length, 19);
  assert.equal(rule.style.getPropertyValue("font"), "math sheetom");
  assert.equal(rule.style.cssText, "font: math sheetom !important;");
  assert.equal(rule.style.getPropertyValue("font-size"), "math");
  assert.equal(rule.style.getPropertyValue("font-family"), "sheetom");

  const beforeInvalid = rule.style.cssText;
  rule.style.setProperty("font", "math math serif", "important");
  assert.equal(rule.style.cssText, beforeInvalid);

  const serialized = rule.parentStyleSheet?.serialize();
  assert.ok(serialized);
  const reparsed = parseStyleSheet(serialized);
  assert.equal(reparsed.serialize(), serialized);
  const reparsedRule = reparsed.cssRules[0];
  assert.ok(reparsedRule instanceof CSSStyleRule);
  assert.equal(reparsedRule.style.getPropertyValue("font"), "math sheetom");
});
