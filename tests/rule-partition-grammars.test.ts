import { expect, test } from "vitest";

import { CSSStyleRule, CSSStyleSheet } from "../src/index.js";

function createStyle() {
  const sheet = new CSSStyleSheet();
  sheet.insertRule(".partition {}");
  const rule = sheet.cssRules[0];
  if (!(rule instanceof CSSStyleRule)) throw new TypeError("Expected a style rule");
  return { sheet, style: rule.style };
}

test.each([
  {
    shorthand: "rule-break",
    longhands: ["column-rule-break", "row-rule-break"],
    value: "intersection",
    replacement: "none",
    invalid: ["none intersection", "spanning-item"],
  },
  {
    shorthand: "rule-visibility-items",
    longhands: [
      "column-rule-visibility-items",
      "row-rule-visibility-items",
    ],
    value: "around",
    replacement: "between",
    invalid: ["between around", "none"],
  },
])("$shorthand owns its complete one-keyword grammar", ({
  shorthand,
  longhands: [first, second],
  value,
  replacement,
  invalid,
}) => {
  if (!first || !second) throw new TypeError("Expected two observed longhands");
  const { sheet, style } = createStyle();
  style.setProperty(shorthand, value, "important");
  expect(style.getPropertyValue(shorthand)).toBe(value);
  expect(Array.from(style)).toEqual([first, second]);

  const before = style.cssText;
  for (const rejected of invalid) {
    style.setProperty(shorthand, rejected);
    expect(style.cssText, rejected).toBe(before);
  }

  style.setProperty(second, replacement, "important");
  expect(style.getPropertyValue(shorthand)).toBe("");
  expect(style.getPropertyValue(first)).toBe(value);
  expect(style.getPropertyValue(second)).toBe(replacement);

  expect(style.removeProperty(second)).toBe(replacement);
  expect(Array.from(style)).toEqual([first]);
  const serialized = sheet.serialize();
  const reparsed = new CSSStyleSheet();
  reparsed.replaceSync(serialized);
  expect(reparsed.serialize()).toBe(serialized);
});
