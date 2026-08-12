import { expect, test } from "vitest";

import { CSSStyleRule, CSSStyleSheet } from "../src/index.js";

function whiteSpaceRule(): { sheet: CSSStyleSheet; rule: CSSStyleRule } {
  const sheet = new CSSStyleSheet();
  sheet.insertRule(".copy { white-space: normal; }");
  const rule = sheet.cssRules[0];
  if (!(rule instanceof CSSStyleRule)) throw new TypeError("expected a style rule");
  return { sheet, rule };
}

test("white-space owns omitted and unordered level four components", () => {
  const { rule } = whiteSpaceRule();

  for (const [input, expected, collapse, mode] of [
    ["collapse", "normal", "collapse", "initial"],
    ["preserve", "pre-wrap", "preserve", "initial"],
    ["preserve-breaks", "pre-line", "preserve-breaks", "initial"],
    ["wrap", "normal", "initial", "wrap"],
    ["wrap preserve", "pre-wrap", "preserve", "wrap"],
    ["nowrap preserve", "pre", "preserve", "nowrap"],
    ["nowrap preserve-breaks", "preserve-breaks nowrap", "preserve-breaks", "nowrap"],
    ["break-spaces nowrap", "break-spaces nowrap", "break-spaces", "nowrap"],
  ] as const) {
    rule.style.setProperty("white-space", input);
    expect(rule.style.getPropertyValue("white-space"), input).toBe(expected);
    expect(rule.style.cssText, input).toBe(`white-space: ${expected};`);
    expect(rule.style.getPropertyValue("white-space-collapse"), input).toBe(collapse);
    expect(rule.style.getPropertyValue("text-wrap-mode"), input).toBe(mode);
  }
});

test("invalid white-space component combinations are atomic no-ops", () => {
  const { rule } = whiteSpaceRule();
  rule.style.setProperty("white-space", "preserve nowrap", "important");

  for (const input of [
    "wrap nowrap",
    "collapse preserve",
    "preserve wrap nowrap",
    "preserve unknown",
  ]) {
    const before = rule.style.cssText;
    rule.style.setProperty("white-space", input);
    expect(rule.style.cssText, input).toBe(before);
  }
});

test("white-space longhand removal breaks synthesis without restoring old shorthand state", () => {
  const { sheet, rule } = whiteSpaceRule();
  rule.style.setProperty("white-space", "preserve");
  expect(rule.style.removeProperty("text-wrap-mode")).toBe("initial");
  expect(rule.style.getPropertyValue("white-space")).toBe("");
  expect(rule.style.getPropertyValue("white-space-collapse")).toBe("preserve");

  const serialized = sheet.serialize();
  const reparsed = new CSSStyleSheet();
  reparsed.replaceSync(serialized);
  expect(reparsed.serialize()).toBe(serialized);
});
