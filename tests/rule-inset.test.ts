import assert from "node:assert/strict";
import { test } from "vitest";

import { CSSStyleRule, parseStyleSheet } from "../src/index.js";
import { createStyleRule } from "./support/create-style-rule.js";

test("rule inset shorthands own canonical expanded longhand state", () => {
  const rule = createStyleRule(".rule");
  rule.style.setProperty(
    "rule-inset",
    "calc(10px + 5%) -2px / overlap-join 4%",
    "important",
  );

  assert.equal(rule.style.length, 8);
  assert.deepEqual(
    Array.from({ length: rule.style.length }, (_, index) => rule.style.item(index)),
    [
      "column-rule-inset-cap-start",
      "column-rule-inset-cap-end",
      "column-rule-inset-junction-start",
      "column-rule-inset-junction-end",
      "row-rule-inset-cap-start",
      "row-rule-inset-cap-end",
      "row-rule-inset-junction-start",
      "row-rule-inset-junction-end",
    ],
  );
  assert.equal(
    rule.style.getPropertyValue("rule-inset"),
    "calc(5% + 10px) -2px / overlap-join 4%",
  );
  assert.equal(
    rule.style.getPropertyValue("column-rule-inset"),
    "calc(5% + 10px) -2px / overlap-join 4%",
  );
  assert.equal(
    rule.style.getPropertyValue("row-rule-inset"),
    "calc(5% + 10px) -2px / overlap-join 4%",
  );
  assert.equal(rule.style.getPropertyPriority("rule-inset"), "important");
  assert.equal(
    rule.style.cssText,
    "rule-inset: calc(5% + 10px) -2px / overlap-join 4% !important;",
  );

  rule.style.setProperty("column-rule-inset-cap-start", "3px", "important");
  assert.equal(rule.style.getPropertyValue("rule-inset"), "");
  assert.equal(
    rule.style.getPropertyValue("column-rule-inset"),
    "3px -2px / overlap-join 4%",
  );
  assert.equal(rule.style.removeProperty("column-rule-inset-cap-start"), "3px");
  assert.equal(rule.style.getPropertyValue("column-rule-inset"), "");
  assert.equal(rule.style.length, 7);
  assert.equal(
    rule.style.cssText,
    "column-rule-inset-cap-end: -2px !important; rule-inset-junction: overlap-join 4% !important; row-rule-inset-cap: calc(5% + 10px) -2px !important;",
  );
});

test("rule inset component shorthands enforce Chromium cardinality", () => {
  const style = createStyleRule(".rule").style;
  style.setProperty("rule-inset-cap", "1px 2px");
  assert.equal(style.getPropertyValue("rule-inset-cap"), "1px 2px");
  assert.equal(style.length, 4);

  style.setProperty("rule-inset-junction", "min(1px, 2px)");
  assert.equal(style.getPropertyValue("rule-inset-junction"), "calc(1px)");

  style.setProperty("rule-inset-start", "overlap-join");
  assert.equal(style.getPropertyValue("rule-inset-start"), "overlap-join");

  const before = style.cssText;
  for (const invalid of [
    "auto",
    "1px 2px 3px",
    "1px / 2px",
    "1px, 2px",
    "overlap-join 1px 2px",
  ]) {
    style.setProperty("rule-inset-cap", invalid);
    assert.equal(style.cssText, before, invalid);
  }
});

test("rule inset substitutions stay pending and safe serialization is idempotent", () => {
  const rule = createStyleRule(".rule");
  rule.style.setProperty("rule-inset", "var(--inset)", "important");

  assert.equal(rule.style.length, 8);
  assert.equal(rule.style.getPropertyValue("rule-inset"), "var(--inset)");
  assert.equal(rule.style.getPropertyValue("row-rule-inset-cap-start"), "");
  assert.equal(rule.style.cssText, "rule-inset: var(--inset) !important;");

  const sheet = rule.parentStyleSheet;
  assert.ok(sheet);
  const serialized = sheet.serialize();
  const reparsed = parseStyleSheet(serialized);
  assert.equal(reparsed.serialize(), serialized);
  const reparsedRule = reparsed.cssRules[0];
  assert.ok(reparsedRule instanceof CSSStyleRule);
  assert.equal(reparsedRule.style.getPropertyValue("rule-inset"), "var(--inset)");
});
