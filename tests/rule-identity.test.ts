import assert from "node:assert/strict";
import { test } from "vitest";

import { CSSStyleRule, CSSStyleSheet } from "../src/index.js";

test("deleteRule detaches retained live objects without replacing the rule list", () => {
  const sheet = new CSSStyleSheet();
  const ruleList = sheet.cssRules;
  sheet.insertRule(".first {}");
  sheet.insertRule(".second {}", 1);

  const retainedRule = sheet.cssRules[0];
  assert.ok(retainedRule instanceof CSSStyleRule);
  const retainedStyle = retainedRule.style;

  assert.equal(sheet.cssRules, ruleList);
  assert.equal(sheet.cssRules.item(0), retainedRule);
  assert.equal(retainedRule.style, retainedStyle);

  assert.equal(sheet.deleteRule(0), undefined);

  assert.equal(ruleList.length, 1);
  assert.equal(ruleList[0]?.cssText, ".second { }");
  assert.equal(retainedRule.parentStyleSheet, null);
  assert.equal(retainedStyle.parentRule, retainedRule);

  retainedStyle.setProperty("color", "red");
  assert.equal(retainedRule.cssText, ".first { color: red; }");
  assert.equal(sheet.serialize(), ".second {\n}\n");
});
