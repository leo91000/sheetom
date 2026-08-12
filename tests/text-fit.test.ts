import { expect, test } from "vitest";

import { CSSStyleRule, CSSStyleSheet } from "../src/index.js";

function textRule(): { sheet: CSSStyleSheet; rule: CSSStyleRule } {
  const sheet = new CSSStyleSheet();
  sheet.insertRule(".fit { text-fit: none; }");
  const rule = sheet.cssRules[0];
  if (!(rule instanceof CSSStyleRule)) throw new TypeError("expected a style rule");
  return { sheet, rule };
}

test("text-fit owns mode, line strategy and limit branches", () => {
  const { rule } = textRule();

  for (const [input, expected] of [
    ["none", "none"],
    ["grow consistent", "grow consistent"],
    ["shrink per-line 10%", "shrink per-line 10%"],
    ["none per-line-all 0%", "none per-line-all 0%"],
    ["none 25%", "none 25%"],
    ["none consistent calc(5% + 5%)", "none consistent calc(10%)"],
    ["none consistent min(10%, 20%)", "none consistent min(10%, 20%)"],
    ["none consistent calc(-1%)", "none consistent calc(-1%)"],
  ] as const) {
    rule.style.setProperty("text-fit", input);
    expect(rule.style.getPropertyValue("text-fit"), input).toBe(expected);
    expect(rule.style.cssText, input).toBe(`text-fit: ${expected};`);
  }
});

test("invalid text-fit neighbors are atomic no-ops", () => {
  const { rule } = textRule();
  rule.style.setProperty("text-fit", "grow consistent 10%", "important");

  for (const input of [
    "consistent",
    "10%",
    "consistent none 10%",
    "none 10% consistent",
    "none consistent -1%",
    "none consistent per-line",
    "none 10% 20%",
  ]) {
    const before = rule.style.cssText;
    rule.style.setProperty("text-fit", input);
    expect(rule.style.cssText, input).toBe(before);
  }
});

test("text-fit pending substitutions and whole-sheet serialization remain stable", () => {
  const { sheet, rule } = textRule();
  rule.style.setProperty("text-fit", "var(--fit, grow consistent 10%)");
  expect(rule.style.getPropertyValue("text-fit")).toBe("var(--fit, grow consistent 10%)");

  const serialized = sheet.serialize();
  const reparsed = new CSSStyleSheet();
  reparsed.replaceSync(serialized);
  expect(reparsed.serialize()).toBe(serialized);
});
