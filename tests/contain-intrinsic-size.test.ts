import { expect, test } from "vitest";

import { CSSStyleRule, CSSStyleSheet } from "../src/index.js";

function intrinsicRule(): { sheet: CSSStyleSheet; rule: CSSStyleRule } {
  const sheet = new CSSStyleSheet();
  sheet.insertRule(".virtual { contain-intrinsic-size: none; }");
  const rule = sheet.cssRules[0];
  if (!(rule instanceof CSSStyleRule)) throw new TypeError("expected a style rule");
  return { sheet, rule };
}

test("contain-intrinsic-size owns one or two compound axis values", () => {
  const { rule } = intrinsicRule();

  for (const [input, expected, width, height] of [
    ["auto none", "auto none", "auto none", "auto none"],
    ["auto 1px", "auto 1px", "auto 1px", "auto 1px"],
    ["auto none auto 1px", "auto none auto 1px", "auto none", "auto 1px"],
    ["10px auto none", "10px auto none", "10px", "auto none"],
    ["auto 1px 20px", "auto 1px 20px", "auto 1px", "20px"],
    ["auto calc(-1px) 20px", "auto calc(-1px) 20px", "auto calc(-1px)", "20px"],
  ] as const) {
    rule.style.setProperty("contain-intrinsic-size", input);
    expect(rule.style.getPropertyValue("contain-intrinsic-size"), input).toBe(expected);
    expect(rule.style.cssText, input).toBe(`contain-intrinsic-size: ${expected};`);
    expect(rule.style.getPropertyValue("contain-intrinsic-width"), input).toBe(width);
    expect(rule.style.getPropertyValue("contain-intrinsic-height"), input).toBe(height);
  }
});

test("invalid contain-intrinsic-size partitions are atomic no-ops", () => {
  const { rule } = intrinsicRule();
  rule.style.setProperty("contain-intrinsic-size", "auto none 10px", "important");

  for (const input of ["auto", "auto auto", "10px 20px 30px", "-1px", "auto none extra"]) {
    const before = rule.style.cssText;
    rule.style.setProperty("contain-intrinsic-size", input);
    expect(rule.style.cssText, input).toBe(before);
  }
});

test("contain-intrinsic-size mutation and serialization preserve expanded state", () => {
  const { sheet, rule } = intrinsicRule();
  rule.style.setProperty("contain-intrinsic-size", "auto none auto 2px");
  expect(rule.style.removeProperty("contain-intrinsic-height")).toBe("auto 2px");
  expect(rule.style.getPropertyValue("contain-intrinsic-size")).toBe("");
  expect(rule.style.getPropertyValue("contain-intrinsic-width")).toBe("auto none");

  rule.style.setProperty("contain-intrinsic-size", "var(--intrinsic, auto none)");
  const serialized = sheet.serialize();
  const reparsed = new CSSStyleSheet();
  reparsed.replaceSync(serialized);
  expect(reparsed.serialize()).toBe(serialized);
});
