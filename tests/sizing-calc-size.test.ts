import { expect, test } from "vitest";

import { CSSStyleRule, CSSStyleSheet } from "../src/index.js";

function styleRule(): { sheet: CSSStyleSheet; rule: CSSStyleRule } {
  const sheet = new CSSStyleSheet();
  sheet.insertRule(".sizing { width: 10px; }");
  const rule = sheet.cssRules[0];
  if (!(rule instanceof CSSStyleRule)) throw new TypeError("expected a style rule");
  return { sheet, rule };
}

test("preferred and minimum sizing properties own calc-size()", () => {
  for (const [property, item] of [
    ["width", "width"],
    ["height", "height"],
    ["min-width", "min-width"],
    ["min-height", "min-height"],
    ["inline-size", "inline-size"],
    ["block-size", "block-size"],
    ["min-inline-size", "min-inline-size"],
    ["min-block-size", "min-block-size"],
    ["-webkit-logical-width", "inline-size"],
    ["-webkit-logical-height", "block-size"],
    ["-webkit-min-logical-width", "min-inline-size"],
    ["-webkit-min-logical-height", "min-block-size"],
  ] as const) {
    const { rule } = styleRule();
    rule.style.cssText = "";
    rule.style.setProperty(property, "calc-size(auto, size)");
    expect(rule.style.length, property).toBe(1);
    expect(rule.style.item(0), property).toBe(item);
    expect(rule.style.getPropertyValue(property), property).toBe("calc-size(auto, size)");
    expect(rule.style.cssText, property).toBe(`${item}: calc-size(auto, size);`);
  }
});

test("calc-size() canonicalizes every Chromium calculation branch", () => {
  const { rule } = styleRule();

  for (const [input, expected] of [
    ["calc-size(auto, size + 1px)", "calc-size(auto, 1px + size)"],
    ["calc-size(auto, size / 2)", "calc-size(auto, 0.5 * size)"],
    ["calc-size(auto, 50%)", "calc-size(auto, 50%)"],
    ["calc-size(auto, round(up, size, 20px))", "calc-size(auto, round(up, size, 20px))"],
    ["calc-size(auto, sign(size) * 1px)", "calc-size(auto, 1px * sign(size))"],
    ["calc-size(any, 1px)", "calc-size(any, 1px)"],
    ["calc-size(10%, size)", "calc-size(10%, size)"],
    ["calc-size(1px + 2px, size)", "calc-size(3px, size)"],
    ["calc-size(anchor-size(width), size)", "calc-size(anchor-size(width), size)"],
    ["calc-size(calc-size(auto, size), size)", "calc-size(calc-size(auto, size), size)"],
    ["calc-size(auto, size, ignored tokens)", "calc-size(auto, size)"],
    ["calc-size(auto, size; color: red)", "calc-size(auto, size)"],
  ] as const) {
    rule.style.setProperty("width", input);
    expect(rule.style.getPropertyValue("width"), input).toBe(expected);
    expect(rule.style.getPropertyValue("color"), input).toBe("");
  }
});

test("maximum sizing properties exclude preferred-only calc-size() bases", () => {
  const { rule } = styleRule();
  rule.style.setProperty("max-width", "calc-size(min-content, size)", "important");
  expect(rule.style.getPropertyValue("max-width")).toBe("calc-size(min-content, size)");

  for (const input of [
    "calc-size(auto, size)",
    "calc-size(calc-size(auto, size), size)",
    "calc-size(none, size)",
    "calc-size(contain, size)",
    "calc-size(fit-content(20px), size)",
  ]) {
    const before = rule.style.cssText;
    rule.style.setProperty("max-width", input);
    expect(rule.style.cssText, input).toBe(before);
  }
});

test("invalid calc-size() values are atomic and pending substitutions remain deferred", () => {
  const { sheet, rule } = styleRule();
  rule.style.setProperty("width", "calc-size(auto, size + 1px)", "important");

  for (const input of [
    "calc-size(any, size)",
    "calc-size(any, min(size, 10px))",
    "calc-size(auto, 0)",
    "calc-size(auto, 1)",
    "calc-size(auto, 1deg)",
    "calc-size(auto size)",
    "calc-size(auto, calc-size(auto, size))",
    "calc-size(auto, size + garbage)",
    "calc-size(contain, size)",
    "calc-size(fit-content(20px), size)",
  ]) {
    const before = rule.style.cssText;
    rule.style.setProperty("width", input);
    expect(rule.style.cssText, input).toBe(before);
  }

  rule.style.setProperty("width", "calc-size(auto, var(--size))", "important");
  expect(rule.style.getPropertyValue("width")).toBe("calc-size(auto, var(--size))");
  const serialized = sheet.serialize();
  const reparsed = new CSSStyleSheet();
  reparsed.replaceSync(serialized);
  expect(reparsed.serialize()).toBe(serialized);
});
