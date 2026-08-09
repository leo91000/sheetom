import assert from "node:assert/strict";
import { test } from "vitest";

import {
  CSSStyleRule,
  CSSStyleSheet,
  parseStyleSheet,
} from "../src/index.js";
import { createStyleRule } from "./support/create-style-rule.js";

test("a recovered setProperty value remains observable and serializes safely", () => {
  const sheet = new CSSStyleSheet();
  assert.equal(sheet.insertRule(".x {}"), 0);

  const rule = sheet.cssRules[0];
  assert.ok(rule instanceof CSSStyleRule);
  assert.equal(rule.type, 1);

  const value = "72px var(--space, var(--space,";
  assert.equal(rule.style.setProperty("padding", value), undefined);

  assert.equal(rule.style.getPropertyValue("padding"), value);
  assert.equal(rule.style.getPropertyValue("padding-top"), "");
  assert.equal(rule.style.getPropertyValue("padding-right"), "");
  assert.equal(rule.style.cssText, `padding: ${value};`);
  assert.equal(rule.cssText, `.x { padding: ${value}; }`);
  const serialized = sheet.serialize();
  assert.equal(
    serialized,
    ".x {\n  padding: 72px var(--space, var(--space, ));\n}\n",
  );

  const reparsed = parseStyleSheet(serialized);
  const reparsedRule = reparsed.cssRules[0];
  assert.ok(reparsedRule instanceof CSSStyleRule);
  assert.equal(
    reparsedRule.style.getPropertyValue("padding"),
    "72px var(--space, var(--space, ))",
  );

  rule.style.setProperty("padding-left", "3px");
  assert.equal(rule.style.getPropertyValue("padding"), "");
  assert.equal(
    rule.style.cssText,
    "padding-top: ; padding-right: ; padding-bottom: ; padding-left: 3px;",
  );
});

test("declaration parsing retains pending shorthand priority and provenance", () => {
  const rule = createStyleRule(".x");
  rule.style.cssText = "padding: var(--p) !important;";

  assert.equal(rule.style.getPropertyValue("padding"), "var(--p)");
  assert.equal(rule.style.getPropertyPriority("padding"), "important");
  assert.equal(rule.style.getPropertyValue("padding-top"), "");
  assert.equal(rule.style.getPropertyPriority("padding-top"), "important");

  assert.equal(rule.style.removeProperty("padding-left"), "");
  assert.equal(
    rule.style.cssText,
    "padding-top:  !important; padding-right:  !important; padding-bottom:  !important;",
  );
});

test("generated shorthand metadata applies pending provenance beyond padding", () => {
  const rule = createStyleRule(".x");
  const value = "12px var(--gap, var(--gap,";
  rule.style.setProperty("margin", value);

  assert.deepEqual(
    Array.from({ length: rule.style.length }, (_, index) => rule.style[index]),
    ["margin-top", "margin-right", "margin-bottom", "margin-left"],
  );
  assert.equal(rule.style.getPropertyValue("margin"), value);
  assert.equal(rule.style.getPropertyValue("margin-top"), "");
  assert.equal(rule.style.cssText, `margin: ${value};`);

  rule.style.setProperty("margin-left", "3px");
  assert.equal(
    rule.style.cssText,
    "margin-top: ; margin-right: ; margin-bottom: ; margin-left: 3px;",
  );
});
