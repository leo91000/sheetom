import { expect, test } from "vitest";

import { CSSStyleRule, CSSStyleSheet } from "../src/index.js";

function textBoxRule(): { sheet: CSSStyleSheet; rule: CSSStyleRule } {
  const sheet = new CSSStyleSheet();
  sheet.insertRule(".label { text-box: normal; }");
  const rule = sheet.cssRules[0];
  if (!(rule instanceof CSSStyleRule)) throw new TypeError("expected a style rule");
  return { sheet, rule };
}

test("text-box owns omitted and unordered component branches", () => {
  const { rule } = textBoxRule();

  for (const [input, expected, trim, edge] of [
    ["trim-start", "trim-start", "trim-start", "auto"],
    ["trim-end", "trim-end", "trim-end", "auto"],
    ["auto", "trim-both", "trim-both", "auto"],
    ["auto none", "normal", "none", "auto"],
    ["none auto", "normal", "none", "auto"],
    ["cap alphabetic trim-end", "trim-end cap alphabetic", "trim-end", "cap alphabetic"],
    ["text trim-start", "trim-start text", "trim-start", "text"],
  ] as const) {
    rule.style.setProperty("text-box", input);
    expect(rule.style.getPropertyValue("text-box"), input).toBe(expected);
    expect(rule.style.cssText, input).toBe(`text-box: ${expected};`);
    expect(rule.style.getPropertyValue("text-box-trim"), input).toBe(trim);
    expect(rule.style.getPropertyValue("text-box-edge"), input).toBe(edge);
  }
});

test("invalid text-box partitions are atomic no-ops", () => {
  const { rule } = textBoxRule();
  rule.style.setProperty("text-box", "cap alphabetic trim-end", "important");

  for (const input of [
    "cap trim-end alphabetic",
    "trim-start trim-end",
    "auto text",
    "trim-start auto text",
  ]) {
    const before = rule.style.cssText;
    rule.style.setProperty("text-box", input);
    expect(rule.style.cssText, input).toBe(before);
  }
});

test("text-box longhand mutation and whole-sheet serialization preserve state", () => {
  const { sheet, rule } = textBoxRule();
  rule.style.setProperty("text-box", "cap alphabetic trim-end");
  expect(rule.style.removeProperty("text-box-edge")).toBe("cap alphabetic");
  expect(rule.style.getPropertyValue("text-box")).toBe("");
  expect(rule.style.getPropertyValue("text-box-trim")).toBe("trim-end");

  rule.style.setProperty("text-box", "var(--box, trim-start auto)");
  const serialized = sheet.serialize();
  const reparsed = new CSSStyleSheet();
  reparsed.replaceSync(serialized);
  expect(reparsed.serialize()).toBe(serialized);
});
