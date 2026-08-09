import assert from "node:assert/strict";
import { test } from "vitest";

import { CSSStyleRule, CSSStyleSheet } from "../src/index.js";

test("insertRule strictly parses one populated style rule", () => {
  const sheet = new CSSStyleSheet();

  assert.equal(sheet.insertRule(".card { width: 1px; color: red !important; }"), 0);

  const rule = sheet.cssRules[0];
  assert.ok(rule instanceof CSSStyleRule);
  assert.equal(rule.parentStyleSheet, sheet);
  assert.equal(rule.selectorText, ".card");
  assert.equal(rule.style.cssText, "width: 1px; color: red !important;");

  assert.throws(
    () => sheet.insertRule(".one {} .two {}"),
    error => error instanceof DOMException && error.name === "SyntaxError",
  );
  assert.equal(sheet.cssRules.length, 1);
});
