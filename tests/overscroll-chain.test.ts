import { expect, test } from "vitest";

import { CSSStyleRule, CSSStyleSheet } from "../src/index.js";

function createStyle() {
  const sheet = new CSSStyleSheet();
  sheet.insertRule(".scroller {}");
  const rule = sheet.cssRules[0];
  if (!(rule instanceof CSSStyleRule)) throw new TypeError("Expected a style rule");
  return { sheet, style: rule.style };
}

test("overscroll chaining expands, mutates, removes, and round trips", () => {
  const { sheet, style } = createStyle();
  style.setProperty("overscroll-behavior", "CHAIN contain", "important");
  expect(style.getPropertyValue("overscroll-behavior")).toBe("chain contain");
  expect(style.getPropertyValue("overscroll-behavior-x")).toBe("chain");
  expect(style.getPropertyValue("overscroll-behavior-y")).toBe("contain");

  style.setProperty("overscroll-behavior-y", "chain", "important");
  expect(style.getPropertyValue("overscroll-behavior")).toBe("chain");
  expect(style.removeProperty("overscroll-behavior-y")).toBe("chain");
  expect(style.getPropertyValue("overscroll-behavior")).toBe("");
  expect(Array.from(style)).toEqual(["overscroll-behavior-x"]);

  const serialized = sheet.serialize();
  const reparsed = new CSSStyleSheet();
  reparsed.replaceSync(serialized);
  expect(reparsed.serialize()).toBe(serialized);
});

test("invalid overscroll replacements are atomic", () => {
  const { style } = createStyle();
  style.setProperty("overscroll-behavior", "chain none", "important");
  const before = style.cssText;
  for (const invalid of ["chain auto none", "normal", "chain, none"]) {
    style.setProperty("overscroll-behavior", invalid);
    expect(style.cssText, invalid).toBe(before);
  }
});
