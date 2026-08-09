import assert from "node:assert/strict";
import { test } from "vitest";

import { CSSStyleRule, parseStyleSheet } from "../src/index.js";

test("selectorText normalizes valid lists and ignores invalid replacements", () => {
  const sheet = parseStyleSheet(".initial { color: red; }");
  const rule = sheet.cssRules[0];
  assert.ok(rule instanceof CSSStyleRule);

  rule.selectorText = ".a,.b";
  assert.equal(rule.selectorText, ".a, .b");
  assert.equal(rule.cssText, ".a, .b { color: red; }");

  rule.selectorText = "::not-a-pseudo(";
  assert.equal(rule.selectorText, ".a, .b");
});
