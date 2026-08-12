import { expect, test } from "vitest";

import { CSSStyleRule, CSSStyleSheet } from "../src/index.js";

function decorationRule(): { sheet: CSSStyleSheet; rule: CSSStyleRule } {
  const sheet = new CSSStyleSheet();
  sheet.insertRule(".misspelled { text-decoration: none; }");
  const rule = sheet.cssRules[0];
  if (!(rule instanceof CSSStyleRule)) throw new TypeError("expected a style rule");
  return { sheet, rule };
}

test("text-decoration owns spelling and grammar error branches", () => {
  const { rule } = decorationRule();

  for (const [input, expected, line, thickness, style, color] of [
    ["spelling-error", "spelling-error", "spelling-error", "initial", "initial", "initial"],
    ["grammar-error", "grammar-error", "grammar-error", "initial", "initial", "initial"],
    ["spelling-error auto solid red", "spelling-error red", "spelling-error", "auto", "solid", "red"],
    ["spelling-error wavy blue 2px", "spelling-error 2px wavy blue", "spelling-error", "2px", "wavy", "blue"],
    ["none red", "red", "none", "initial", "initial", "red"],
    ["solid red", "red", "initial", "initial", "solid", "red"],
  ] as const) {
    rule.style.setProperty("text-decoration", input);
    expect(rule.style.getPropertyValue("text-decoration"), input).toBe(expected);
    expect(rule.style.cssText, input).toBe(`text-decoration: ${expected};`);
    expect(rule.style.getPropertyValue("text-decoration-line"), input).toBe(line);
    expect(rule.style.getPropertyValue("text-decoration-thickness"), input).toBe(thickness);
    expect(rule.style.getPropertyValue("text-decoration-style"), input).toBe(style);
    expect(rule.style.getPropertyValue("text-decoration-color"), input).toBe(color);
  }
});

test("invalid text-decoration error combinations are atomic no-ops", () => {
  const { rule } = decorationRule();
  rule.style.setProperty("text-decoration", "spelling-error wavy blue 2px", "important");

  for (const input of [
    "grammar-error underline",
    "spelling-error grammar-error",
    "spelling-error none",
    "spelling-error double wavy",
    "spelling-error 1px 2px",
  ]) {
    const before = rule.style.cssText;
    rule.style.setProperty("text-decoration", input);
    expect(rule.style.cssText, input).toBe(before);
  }
});

test("text-decoration error state survives longhand mutation and serialization", () => {
  const { sheet, rule } = decorationRule();
  rule.style.setProperty("text-decoration", "spelling-error wavy blue 2px");
  expect(rule.style.removeProperty("text-decoration-color")).toBe("blue");
  expect(rule.style.getPropertyValue("text-decoration")).toBe("");
  expect(rule.style.getPropertyValue("text-decoration-line")).toBe("spelling-error");

  rule.style.setProperty("text-decoration", "var(--decoration, spelling-error red)");
  const serialized = sheet.serialize();
  const reparsed = new CSSStyleSheet();
  reparsed.replaceSync(serialized);
  expect(reparsed.serialize()).toBe(serialized);
});
