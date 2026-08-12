import { expect, test } from "vitest";

import { CSSStyleRule, CSSStyleSheet } from "../src/index.js";

function styleRule(): { sheet: CSSStyleSheet; rule: CSSStyleRule } {
  const sheet = new CSSStyleSheet();
  sheet.insertRule(".border-image { border-image-slice: 5; }");
  const rule = sheet.cssRules[0];
  if (!(rule instanceof CSSStyleRule)) throw new TypeError("expected a style rule");
  return { sheet, rule };
}

test("border-image-slice accepts fill in either order and compresses four sides", () => {
  const { rule } = styleRule();

  for (const [input, expected] of [
    ["fill 1", "1 fill"],
    ["1 fill", "1 fill"],
    ["1 1 fill", "1 fill"],
    ["1 2 1 fill", "1 2 fill"],
    ["1 2 1 2 fill", "1 2 fill"],
    ["10% fill", "10% fill"],
  ] as const) {
    rule.style.setProperty("border-image-slice", input);
    expect(rule.style.getPropertyValue("border-image-slice"), input).toBe(expected);
    expect(rule.style.cssText, input).toBe(`border-image-slice: ${expected};`);
  }
});

test("border-image-slice preserves math provenance beside fill", () => {
  const { rule } = styleRule();

  for (const [input, expected] of [
    ["calc(-1) fill", "calc(-1) fill"],
    ["calc(1 + 1) fill", "calc(2) fill"],
    ["min(1, 2) fill", "calc(1) fill"],
    ["sign(1em) fill", "sign(1em) fill"],
    ["calc(2 * sign(1em)) fill", "calc(2 * sign(1em)) fill"],
    ["min(1%, 2%) fill", "min(1%, 2%) fill"],
  ] as const) {
    rule.style.setProperty("border-image-slice", input);
    expect(rule.style.getPropertyValue("border-image-slice"), input).toBe(expected);
  }
});

test("invalid border-image-slice fill neighbors are atomic no-ops", () => {
  const { rule } = styleRule();
  rule.style.setProperty("border-image-slice", "10% fill", "important");
  const before = rule.style.cssText;

  for (const input of [
    "fill",
    "1 fill fill",
    "-1 fill",
    "1 2 3 4 5 fill",
    "1px fill",
    "calc(1 + 1%) fill",
  ]) {
    rule.style.setProperty("border-image-slice", input);
    expect(rule.style.cssText, input).toBe(before);
  }
});

test("border-image synthesis survives a fill slice mutation", () => {
  const { rule } = styleRule();
  rule.style.setProperty(
    "border-image",
    'url("x.png") fill 1 2 / 3 / 4 round',
    "important",
  );

  expect(rule.style.getPropertyValue("border-image")).toBe(
    'url("x.png") 1 2 fill / 3 / 4 round',
  );
  rule.style.setProperty("border-image-slice", "10% fill", "important");
  expect(rule.style.getPropertyValue("border-image")).toBe(
    'url("x.png") 10% fill / 3 / 4 round',
  );
  expect(rule.style.removeProperty("border-image-slice")).toBe("10% fill");
  expect(rule.style.getPropertyValue("border-image")).toBe("");
});

test("border-image fill round-trips through reparsable serialization", () => {
  const { sheet, rule } = styleRule();
  rule.style.setProperty(
    "border-image",
    'image-set(url("a.png") 1x, url("b.png") 2x) sign(1em) fill / 2 / 3 round',
  );

  const serialized = sheet.serialize();
  const reparsed = new CSSStyleSheet();
  reparsed.replaceSync(serialized);
  expect(reparsed.serialize()).toBe(serialized);
});
