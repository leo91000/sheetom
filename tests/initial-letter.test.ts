import { expect, test } from "vitest";

import { CSSStyleRule, CSSStyleSheet } from "../src/index.js";

function initialLetterRule(): { sheet: CSSStyleSheet; rule: CSSStyleRule } {
  const sheet = new CSSStyleSheet();
  sheet.insertRule(".drop-cap { initial-letter: normal; }");
  const rule = sheet.cssRules[0];
  if (!(rule instanceof CSSStyleRule)) throw new TypeError("expected a style rule");
  return { sheet, rule };
}

test("initial-letter owns size, sink and canonical component order", () => {
  const { rule } = initialLetterRule();

  for (const [input, expected] of [
    ["normal", "normal"],
    ["1.5", "1.5"],
    ["1.5 1", "1.5 1"],
    ["drop 1", "1 drop"],
    ["raise calc(1 + 1)", "calc(2) raise"],
    ["calc(-1) drop", "calc(-1) drop"],
    ["1 calc(1.5)", "1 calc(1.5)"],
    ["1 sign(1em)", "1 sign(1em)"],
  ] as const) {
    rule.style.setProperty("initial-letter", input);
    expect(rule.style.getPropertyValue("initial-letter"), input).toBe(expected);
    expect(rule.style.cssText, input).toBe(`initial-letter: ${expected};`);
  }
});

test("invalid initial-letter neighbors are atomic no-ops", () => {
  const { rule } = initialLetterRule();
  rule.style.setProperty("initial-letter", "1.5 drop", "important");

  for (const input of [
    "0",
    "-1",
    "drop",
    "1 0",
    "1 -1",
    "1 1.5",
    "drop 1 2",
    "1 drop 2",
    "normal 1",
  ]) {
    const before = rule.style.cssText;
    rule.style.setProperty("initial-letter", input);
    expect(rule.style.cssText, input).toBe(before);
  }
});

test("initial-letter pending substitutions and whole-sheet serialization remain stable", () => {
  const { sheet, rule } = initialLetterRule();
  rule.style.setProperty("initial-letter", "var(--drop-cap, 1.5 drop)");
  expect(rule.style.getPropertyValue("initial-letter")).toBe("var(--drop-cap, 1.5 drop)");

  const serialized = sheet.serialize();
  const reparsed = new CSSStyleSheet();
  reparsed.replaceSync(serialized);
  expect(reparsed.serialize()).toBe(serialized);
});
