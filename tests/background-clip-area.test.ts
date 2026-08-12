import { expect, test } from "vitest";

import { CSSStyleRule, CSSStyleSheet } from "../src/index.js";

function styleRule(): { sheet: CSSStyleSheet; rule: CSSStyleRule } {
  const sheet = new CSSStyleSheet();
  sheet.insertRule(".background { background: none; }");
  const rule = sheet.cssRules[0];
  if (!(rule instanceof CSSStyleRule)) throw new TypeError("expected a style rule");
  return { sheet, rule };
}

test("background expands border-area and text clipping across layers", () => {
  const { rule } = styleRule();

  rule.style.setProperty("background", "none, text border-area");

  expect(rule.style.getPropertyValue("background")).toBe(
    "none, border-box border-area text",
  );
  expect(rule.style.getPropertyValue("background-origin")).toBe("initial, border-box");
  expect(rule.style.getPropertyValue("background-clip")).toBe(
    "initial, border-area text",
  );
});

test("background canonicalizes level four clipping forms", () => {
  const { rule } = styleRule();

  for (const [input, shorthand, origin, clip] of [
    ["border-area", "border-box border-area", "border-box", "border-area"],
    ["text", "text", "initial", "text"],
    [
      "text border-area",
      "border-box border-area text",
      "border-box",
      "border-area text",
    ],
    [
      "content-box border-area text",
      "content-box border-area text",
      "content-box",
      "border-area text",
    ],
    ["border-area content-box", "content-box border-area", "content-box", "border-area"],
  ] as const) {
    rule.style.setProperty("background", input);
    expect(rule.style.getPropertyValue("background"), input).toBe(shorthand);
    expect(rule.style.getPropertyValue("background-origin"), input).toBe(origin);
    expect(rule.style.getPropertyValue("background-clip"), input).toBe(clip);
  }
});

test("background-clip owns level four lists and rejects invalid neighbors atomically", () => {
  const { rule } = styleRule();

  rule.style.setProperty("background-clip", "text border-area, text");
  expect(rule.style.getPropertyValue("background-clip")).toBe("border-area text, text");
  const before = rule.style.cssText;

  for (const input of [
    "border",
    "border-area border-area",
    "text text",
    "content-box text",
    "border-area content-box",
  ]) {
    rule.style.setProperty("background-clip", input);
    expect(rule.style.cssText, input).toBe(before);
  }
});

test("background synthesizes compatible level four clip mutations", () => {
  const { rule } = styleRule();
  rule.style.setProperty("background", "none, border-area text", "important");

  rule.style.setProperty("background-clip", "text, border-area text", "important");

  expect(rule.style.getPropertyValue("background")).toBe(
    "none text, border-box border-area text",
  );
  expect(rule.style.cssText).toBe(
    "background: none text, border-box border-area text !important;",
  );

  expect(rule.style.removeProperty("background-origin")).toBe("initial, border-box");
  expect(rule.style.getPropertyValue("background")).toBe("");
  expect(rule.style.cssText).toBe(
    "background-image: none, initial !important; background-position-x: initial, initial !important; background-position-y: initial, initial !important; background-size: initial, initial !important; background-repeat: initial, initial !important; background-attachment: initial, initial !important; background-clip: text, border-area text !important; background-color: initial !important;",
  );
});

test("background does not synthesize level four clips across mixed priorities", () => {
  const { rule } = styleRule();
  rule.style.setProperty("background", "none, border-area text", "important");
  rule.style.setProperty("background-clip", "text, border-area text");

  expect(rule.style.getPropertyValue("background")).toBe("");
  expect(rule.style.getPropertyPriority("background")).toBe("");
});

test("level four background clipping round-trips through reparsable serialization", () => {
  const { sheet, rule } = styleRule();
  rule.style.setProperty(
    "background",
    "none, none center / 1px repeat-x scroll text border-area content-box red",
  );

  expect(rule.style.getPropertyValue("background")).toBe(
    "none, none center center / 1px repeat-x scroll content-box border-area text red",
  );

  const serialized = sheet.serialize();
  const reparsed = new CSSStyleSheet();
  reparsed.replaceSync(serialized);
  expect(reparsed.serialize()).toBe(serialized);
});
