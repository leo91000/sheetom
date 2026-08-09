import assert from "node:assert/strict";
import { test } from "vitest";

import { CSSMediaRule, CSSRule, CSSStyleSheet } from "../src/index.js";

test("mutable grouping at-rules use specialized live rules", () => {
  const sheet = new CSSStyleSheet();

  assert.equal(sheet.insertRule("@media print { .card { color: red; } }"), 0);

  const rule = sheet.cssRules[0];
  assert.ok(rule instanceof CSSMediaRule);
  assert.ok(rule instanceof CSSRule);
  assert.equal(rule.type, CSSRule.MEDIA_RULE);
  assert.equal(rule.cssText, "@media print {\n  .card { color: red; }\n}");
  assert.equal(rule.parentStyleSheet, sheet);
  assert.equal(
    sheet.serialize(),
    "@media print {\n  .card {\n    color: red;\n  }\n}\n",
  );
});
