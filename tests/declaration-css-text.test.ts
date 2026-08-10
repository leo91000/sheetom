import assert from "node:assert/strict";
import { test } from "vitest";

import { CSSStyleRule, parseStyleSheet } from "../src/index.js";
import { createStyleRule } from "./support/create-style-rule.js";

test("cssText replaces a declaration block using Chromium winner ordering", () => {
  const rule = createStyleRule(".card");

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

test("repeated serialization is cached without hiding later mutations", () => {
  const sheet = parseStyleSheet(".card { color: red; padding: 1px 2px; }");
  const rule = sheet.cssRules[0];
  assert.ok(rule instanceof CSSStyleRule);

  const initial = sheet.serialize();
  assert.equal(sheet.serialize(), initial);

  rule.style.setProperty("padding-left", "3px");
  const mutated = sheet.serialize();
  assert.notEqual(mutated, initial);
  assert.match(mutated, /padding: 1px 2px 1px 3px/u);
  assert.equal(sheet.serialize(), mutated);

  rule.style.removeProperty("color");
  const removed = sheet.serialize();
  assert.doesNotMatch(removed, /color:/u);
  assert.equal(sheet.serialize(), removed);
});
