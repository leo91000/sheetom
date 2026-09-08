import assert from "node:assert/strict";
import { test } from "vitest";
import { CSS, CSSStyleSheet, StyleSheet } from "../src/index.js";

test("CSSStyleSheet inherits the nonconstructible StyleSheet interface", () => {
  const sheet = new CSSStyleSheet({ media: "print", disabled: true });
  assert.ok(sheet instanceof StyleSheet);
  assert.equal(sheet.media.mediaText, "print");
  assert.equal(sheet.disabled, true);
  assert.equal(sheet.href, null);
  assert.throws(() => Reflect.construct(StyleSheet, []), TypeError);
});

test("CSS.escape implements identifier serialization and DOMString conversion", () => {
  for (const [input, expected] of [
    ["", ""], ["-", "\\-"], ["--x", "--x"], ["0a", "\\30 a"],
    ["-1a", "-\\31 a"], ["a b#c", "a\\ b\\#c"], ["\0", "\uFFFD"],
    ["\n", "\\a "], ["é🐕", "é🐕"], ["\uD800", "\uD800"],
  ]) assert.equal(CSS.escape(input!), expected);
  assert.equal(Reflect.apply(CSS.escape, null, [null]), "null");
  assert.throws(() => Reflect.apply(CSS.escape, null, []), TypeError);
  assert.throws(() => Reflect.apply(CSS.escape, null, [Symbol()]), TypeError);
  assert.equal(Object.prototype.toString.call(CSS), "[object CSS]");
});

test("CSS.supports validates both overloads without accepting invalid declarations", () => {
  for (const [property, value, expected] of [
    ["color", "red", true], ["color", "red!important", false],
    ["color", "red; width: 1px", false], ["width", "-1px", false],
    ["width", "var(--x", true], ["--x", "", false],
    ["display", "grid", true], ["not-a-property", "1px", false],
  ] as const) assert.equal(CSS.supports(property, value), expected, `${property}: ${value}`);
  assert.throws(() => Reflect.apply(CSS.supports, null, []), TypeError);
  assert.equal(CSS.supports("color", undefined), false);
  for (const [condition, expected] of [
    ["color: red", true], ["(color: red !important)", true],
    ["(display: grid) and (color: red)", true],
    ["(invalid: value) or (display: grid)", true],
    ["not (invalid: value)", true], ["not (color:red;)", true],
    ["(color: red) and (display: grid) or (width: 1px)", false],
    ["selector(:has(> .card))", true], ["selector(:unknown)", false],
    ["selector(:is(div, :unknown))", false], ["selector(div, p)", false],
    ["selector([missing|attr])", false], ["selector([svg|attr])", false],
    ["selector([*|attr])", true], ["selector([|attr])", true],
    ["selector(:is([missing|attr], .valid))", false],
    ["selector(:where(missing|a, .valid))", false],
    ["font-format(woff2)", true], ["font-format(svg)", false],
    ["font-tech(color-colrv1)", true], ["font-tech(color-svg)", false],
    ["unknown(feature)", false],
  ] as const) assert.equal(CSS.supports(condition), expected, condition);
});
