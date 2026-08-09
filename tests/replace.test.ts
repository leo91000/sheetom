import assert from "node:assert/strict";
import { test } from "vitest";

import { CSSStyleRule, CSSStyleSheet } from "../src/index.js";

test("replaceSync reparses a constructed sheet while preserving its live rule list", async () => {
  const sheet = new CSSStyleSheet();
  const ruleList = sheet.cssRules;
  sheet.insertRule(".old {}");
  const oldRule = sheet.cssRules[0];
  assert.ok(oldRule instanceof CSSStyleRule);

  assert.equal(
    sheet.replaceSync('@import "theme.css"; .next { width: 1px; color: red; }'),
    undefined,
  );

  assert.equal(sheet.cssRules, ruleList);
  assert.equal(oldRule.parentStyleSheet, null);
  assert.equal(ruleList.length, 1);

  const nextRule = ruleList[0];
  assert.ok(nextRule instanceof CSSStyleRule);
  assert.equal(nextRule.selectorText, ".next");
  assert.deepEqual(
    Array.from({ length: nextRule.style.length }, (_, index) => nextRule.style[index]),
    ["width", "color"],
  );
  assert.equal(nextRule.style.cssText, "width: 1px; color: red;");

  assert.equal(await sheet.replace(".final { padding: 1px 2px; }"), sheet);
  assert.equal(ruleList.length, 1);
  assert.equal(ruleList[0]?.cssText, ".final { padding: 1px 2px; }");
});
