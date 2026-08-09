import assert from "node:assert/strict";
import { test } from "vitest";

import { CSSStyleRule, parseStyleSheet } from "../src/index.js";

test("cssText replaces a declaration block using Chromium winner ordering", () => {
  const rule = new CSSStyleRule(".card");

  rule.style.cssText =
    "width: 1px !important; color: red; width: 2px; height: 3px !important;";

  assert.deepEqual(
    Array.from({ length: rule.style.length }, (_, index) => rule.style[index]),
    ["color", "width", "height"],
  );
  assert.equal(rule.style.getPropertyValue("width"), "1px");
  assert.equal(rule.style.getPropertyPriority("width"), "important");
  assert.equal(
    rule.style.cssText,
    "color: red; width: 1px !important; height: 3px !important;",
  );

  rule.style.cssText = "";
  assert.equal(rule.style.length, 0);
  assert.equal(rule.style.cssText, "");
});

test("stylesheet parsing shares declaration-block winner semantics", () => {
  const sheet = parseStyleSheet(
    ".card { width: 1px !important; color: red; width: 2px; }",
  );
  const rule = sheet.cssRules[0];
  assert.ok(rule instanceof CSSStyleRule);

  assert.deepEqual(
    Array.from({ length: rule.style.length }, (_, index) => rule.style[index]),
    ["color", "width"],
  );
  assert.equal(rule.style.cssText, "color: red; width: 1px !important;");
});
