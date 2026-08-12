import { expect, test } from "vitest";

import { CSSStyleRule, CSSStyleSheet } from "../src/index.js";

function createStyle() {
  const sheet = new CSSStyleSheet();
  sheet.insertRule(".card {}");
  const rule = sheet.cssRules[0];
  if (!(rule instanceof CSSStyleRule)) throw new TypeError("Expected a style rule");
  return { sheet, style: rule.style };
}

test("container types combine size and scroll-state in Chromium order", () => {
  for (const [input, expected] of [
    ["size scroll-state", "size scroll-state"],
    ["scroll-state size", "size scroll-state"],
    ["inline-size scroll-state", "inline-size scroll-state"],
    ["scroll-state inline-size", "inline-size scroll-state"],
  ] as const) {
    const { style } = createStyle();
    style.setProperty("container-type", input);
    expect(style.getPropertyValue("container-type"), input).toBe(expected);
    expect(style.cssText, input).toBe(`container-type: ${expected};`);
  }
});

test("container expands combined types and remains mutable", () => {
  const { sheet, style } = createStyle();
  style.setProperty("container", "card / scroll-state size", "important");

  expect(style.getPropertyValue("container")).toBe("card / size scroll-state");
  expect(style.getPropertyValue("container-name")).toBe("card");
  expect(style.getPropertyValue("container-type")).toBe("size scroll-state");
  expect(style.getPropertyPriority("container")).toBe("important");

  expect(style.removeProperty("container-type")).toBe("size scroll-state");
  expect(style.getPropertyValue("container")).toBe("");
  expect(style.getPropertyValue("container-name")).toBe("card");

  const serialized = sheet.serialize();
  const reparsed = new CSSStyleSheet();
  reparsed.replaceSync(serialized);
  expect(reparsed.serialize()).toBe(serialized);
});

test("invalid container type combinations are atomic no-ops", () => {
  for (const input of [
    "normal scroll-state",
    "size inline-size",
    "size size",
    "scroll-state scroll-state",
  ]) {
    const { style } = createStyle();
    style.setProperty("container", "card / size scroll-state", "important");
    const before = style.cssText;
    style.setProperty("container-type", input);
    expect(style.cssText, input).toBe(before);
  }
});
