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

test("typed EOF recovery matches Chromium CSSOM serialization", () => {
  const cases = [
    ["font-family", '"Gotham', "Gotham"],
    ["content", '"hello', '"hello"'],
    ["color", "red/*comment", "red"],
    ["width", "calc(1px", "calc(1px)"],
    ["width", "min(1px", "calc(1px)"],
    ["color", "rgb(1 2 3", "rgb(1, 2, 3)"],
    ["background-image", "url(foo", 'url("foo")'],
    ["transform", "translateX(1px", "translateX(1px)"],
    [
      "background-image",
      "linear-gradient(red, blue",
      "linear-gradient(red, blue)",
    ],
  ] as const;

  for (const [property, input, expected] of cases) {
    const rule = createStyleRule(".x");
    rule.style.setProperty(property, input);
    assert.equal(rule.style.getPropertyValue(property), expected, `${property}: ${input}`);
    assert.equal(rule.style.cssText, `${property}: ${expected};`);
  }
});

test("typed font-family serialization follows the Chromium fallback", () => {
  const rule = createStyleRule(".x");
  rule.style.setProperty("font-family", '"Gotham"');

  assert.equal(rule.style.getPropertyValue("font-family"), "Gotham");
  assert.equal(rule.style.cssText, "font-family: Gotham;");
});

test("typed values use browser-facing canonical serialization", () => {
  const rule = createStyleRule(".x");
  rule.style.setProperty("color", "rgb(1 2 3 / 50%)");

  assert.equal(rule.style.getPropertyValue("color"), "rgba(1, 2, 3, 0.5)");
  assert.equal(rule.style.cssText, "color: rgba(1, 2, 3, 0.5);");
});

test("retained EOF token text applies lexical recovery without closing blocks", () => {
  const cases = [
    ["--escaped", "foo\\", "foo�"],
    ["--string", '"hello', '"hello'],
    ["--comment", "red/*comment", "red"],
    ["--square", "[foo", "[foo"],
    ["--curly", "{foo", "{foo"],
    ["--url", "url(foo\\", "url(foo�)"],
    ["width", "calc(var(--x, 1px", "calc(var(--x, 1px"],
    ["color", "var(--x, red/*comment", "var(--x, red"],
  ] as const;

  for (const [property, input, expected] of cases) {
    const rule = createStyleRule(".x");
    rule.style.setProperty(property, input);
    assert.equal(rule.style.getPropertyValue(property), expected, `${property}: ${input}`);
  }
});
