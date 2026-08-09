import assert from "node:assert/strict";
import { test } from "vitest";

import { CSSRule, CSSStyleSheet } from "../src/index.js";

test("valid at-rules are inserted as live generic rules", () => {
  const sheet = new CSSStyleSheet();

  assert.equal(sheet.insertRule("@media print { .card { color: red; } }"), 0);

  const rule = sheet.cssRules[0];
  assert.ok(rule instanceof CSSRule);
  assert.equal(rule.type, CSSRule.MEDIA_RULE);
  assert.equal(rule.cssText, "@media print{.card{color:red}}");
  assert.equal(rule.parentStyleSheet, sheet);
  assert.equal(sheet.serialize(), "@media print{.card{color:red}}\n");
});
