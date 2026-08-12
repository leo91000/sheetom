import { expect, test } from "vitest";

import { CSSStyleRule, CSSStyleSheet } from "../src/index.js";

function mathRule(): { sheet: CSSStyleSheet; rule: CSSStyleRule } {
  const sheet = new CSSStyleSheet();
  sheet.insertRule("math { math-depth: 0; }");
  const rule = sheet.cssRules[0];
  if (!(rule instanceof CSSStyleRule)) throw new TypeError("expected a style rule");
  return { sheet, rule };
}

test("math layout keywords canonicalize like Chromium", () => {
  const { rule } = mathRule();

  for (const [property, input, expected] of [
    ["font-size", "MATH", "math"],
    ["baseline-shift", "SUB", "sub"],
    ["baseline-shift", "SUPER", "super"],
    ["text-transform", "MATH-AUTO", "math-auto"],
    ["math-depth", "AUTO-ADD", "auto-add"],
  ] as const) {
    rule.style.setProperty(property, input);
    expect(rule.style.getPropertyValue(property), `${property}: ${input}`).toBe(expected);
    expect(rule.style.getPropertyPriority(property), `${property}: ${input}`).toBe("");
  }
});

test("math-depth owns integer functions and canonicalizes their calculations", () => {
  const { rule } = mathRule();

  for (const [input, expected] of [
    ["add(+1)", "add(1)"],
    ["add(calc(1 + 1))", "add(calc(2))"],
    ["add(min(1, 2))", "add(calc(1))"],
    ["add(calc(1.5))", "add(calc(1.5))"],
    ["add(calc(infinity))", "add(calc(infinity))"],
    ["add(calc(-infinity))", "add(calc(-infinity))"],
    ["add(calc(NaN))", "add(calc(NaN))"],
    ["add(calc(1 / 0))", "add(calc(infinity))"],
  ] as const) {
    rule.style.setProperty("math-depth", input, "important");
    expect(rule.style.getPropertyValue("math-depth"), input).toBe(expected);
    expect(rule.style.getPropertyPriority("math-depth"), input).toBe("important");
  }
});

test("invalid math layout neighbors are atomic no-ops", () => {
  const { rule } = mathRule();
  rule.style.setProperty("math-depth", "add(calc(1 + 1))", "important");
  rule.style.setProperty("text-transform", "uppercase", "important");

  for (const [property, input] of [
    ["font-size", "math extra"],
    ["baseline-shift", "sub super"],
    ["text-transform", "math-auto uppercase"],
    ["math-depth", "add(1.0)"],
    ["math-depth", "add()"],
    ["math-depth", "add(foo(1))"],
    ["math-depth", "add(infinity)"],
    ["math-depth", "add(1) extra"],
  ] as const) {
    const before = rule.style.cssText;
    rule.style.setProperty(property, input);
    expect(rule.style.cssText, `${property}: ${input}`).toBe(before);
  }
});

test("math layout values survive whole-sheet serialization and substitutions", () => {
  const { sheet, rule } = mathRule();
  rule.style.setProperty("font-size", "math");
  rule.style.setProperty("baseline-shift", "super");
  rule.style.setProperty("text-transform", "math-auto");
  rule.style.setProperty("math-depth", "add(var(--increment, 1))");

  const serialized = sheet.serialize();
  const reparsed = new CSSStyleSheet();
  reparsed.replaceSync(serialized);
  expect(reparsed.serialize()).toBe(serialized);
});
