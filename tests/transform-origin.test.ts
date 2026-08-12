import { expect, test } from "vitest";

import { CSSStyleRule, CSSStyleSheet } from "../src/index.js";

function styleRule(): { sheet: CSSStyleSheet; rule: CSSStyleRule } {
  const sheet = new CSSStyleSheet();
  sheet.insertRule(".origin { transform-origin: 9px 8px; }");
  const rule = sheet.cssRules[0];
  if (!(rule instanceof CSSStyleRule)) throw new TypeError("expected a style rule");
  return { sheet, rule };
}

test("transform origins own the optional depth axis", () => {
  const { sheet, rule } = styleRule();

  for (const [input, expected] of [
    ["left", "left center"],
    ["top left", "left top"],
    ["1px top 1px", "1px top 1px"],
    ["left 10% 1px", "left 10% 1px"],
    ["center center -1px", "center center -1px"],
    ["center center 0", "center center 0px"],
    ["center center calc(1px + 2px)", "center center calc(3px)"],
  ] as const) {
    rule.style.setProperty("transform-origin", input);
    expect(rule.style.getPropertyValue("transform-origin"), input).toBe(expected);
  }

  const serialized = sheet.serialize();
  const reparsed = new CSSStyleSheet();
  reparsed.replaceSync(serialized);
  expect(reparsed.serialize()).toBe(serialized);
});

test("invalid transform-origin positions are atomic no-ops", () => {
  const { rule } = styleRule();
  const before = rule.style.cssText;

  for (const input of [
    "left 10px top",
    "left 10px top 20px",
    "left top 10%",
    "center center calc(10%)",
    "left top 1px 2px",
    "top 10px",
    "10px left",
  ]) {
    rule.style.setProperty("transform-origin", input);
    expect(rule.style.cssText, input).toBe(before);
  }
});

test("the webkit transform-origin alias shares canonical state", () => {
  const { rule } = styleRule();

  rule.style.setProperty("-webkit-transform-origin", "top center 1px", "important");

  expect(Array.from(rule.style)).toEqual(["transform-origin"]);
  expect(rule.style.getPropertyValue("transform-origin")).toBe("center top 1px");
  expect(rule.style.getPropertyValue("-webkit-transform-origin")).toBe("center top 1px");
  expect(rule.style.getPropertyPriority("transform-origin")).toBe("important");
});
