import { expect, test } from "vitest";

import { CSSStyleRule, CSSStyleSheet } from "../src/index.js";

function styleRule(): { sheet: CSSStyleSheet; rule: CSSStyleRule } {
  const sheet = new CSSStyleSheet();
  sheet.insertRule(".columns { columns: auto; }");
  const rule = sheet.cssRules[0];
  if (!(rule instanceof CSSStyleRule))
    throw new TypeError("expected a style rule");
  return { sheet, rule };
}

test("columns expands an optional height into canonical longhand state", () => {
  const { rule } = styleRule();

  rule.style.setProperty(
    "columns",
    "2 100px / calc(100px + 200px)",
    "important"
  );

  expect(Array.from(rule.style)).toEqual([
    "column-width",
    "column-count",
    "column-height",
    "column-wrap",
  ]);
  expect(rule.style.getPropertyValue("column-width")).toBe("100px");
  expect(rule.style.getPropertyValue("column-count")).toBe("2");
  expect(rule.style.getPropertyValue("column-height")).toBe("calc(300px)");
  expect(rule.style.getPropertyValue("column-wrap")).toBe("auto");
  expect(rule.style.getPropertyValue("columns")).toBe("100px 2 / calc(300px)");
  expect(rule.style.getPropertyPriority("columns")).toBe("important");
});

test("columns canonicalizes omitted and explicit auto heights", () => {
  const { rule } = styleRule();

  for (const [input, expected] of [
    ["auto / auto", "auto"],
    ["1px / auto", "1px"],
    ["auto auto / auto", "auto"],
    ["auto / 1px", "auto / 1px"],
    ["auto / 0", "auto / 0px"],
  ] as const) {
    rule.style.setProperty("columns", input);
    expect(rule.style.getPropertyValue("columns"), input).toBe(expected);
  }
});

test("columns synthesizes from compatible longhand mutations", () => {
  const { rule } = styleRule();
  rule.style.setProperty("columns", "100px 2 / 300px", "important");

  rule.style.setProperty("column-height", "400px", "important");
  expect(rule.style.getPropertyValue("columns")).toBe("100px 2 / 400px");

  rule.style.setProperty("column-wrap", "wrap", "important");
  expect(rule.style.getPropertyValue("columns")).toBe("100px 2 / 400px");
  expect(rule.style.cssText).toBe("columns: 100px 2 / 400px !important;");

  expect(rule.style.removeProperty("column-height")).toBe("400px");
  expect(rule.style.getPropertyValue("columns")).toBe("");
  expect(rule.style.cssText).toBe(
    "column-width: 100px !important; column-count: 2 !important; column-wrap: wrap !important;"
  );
});

test("columns does not synthesize across mixed priorities", () => {
  const { rule } = styleRule();
  rule.style.setProperty("columns", "100px 2 / 300px", "important");
  rule.style.setProperty("column-wrap", "wrap");

  expect(rule.style.getPropertyValue("columns")).toBe("");
  expect(rule.style.getPropertyPriority("columns")).toBe("");
  expect(rule.style.cssText).toBe(
    "column-width: 100px !important; column-count: 2 !important; column-height: 300px !important; column-wrap: wrap;"
  );
});

test("invalid columns heights are atomic no-ops", () => {
  const { rule } = styleRule();
  rule.style.setProperty("columns", "5px 3", "important");
  const before = rule.style.cssText;

  for (const input of [
    "auto / 10%",
    "auto / -1px",
    "auto / max-content",
    "auto / 1px 2px",
    "auto / 1px / 2px",
    "/ 1px",
    "auto /",
  ]) {
    rule.style.setProperty("columns", input);
    expect(rule.style.cssText, input).toBe(before);
  }
});

test("the webkit columns alias shares canonical state and round-trips", () => {
  const { sheet, rule } = styleRule();
  rule.style.setProperty("-webkit-columns", "2 100px / 300px", "important");

  expect(Array.from(rule.style)).toEqual([
    "column-width",
    "column-count",
    "column-height",
    "column-wrap",
  ]);
  expect(rule.style.getPropertyValue("columns")).toBe("100px 2 / 300px");
  expect(rule.style.getPropertyValue("-webkit-columns")).toBe(
    "100px 2 / 300px"
  );

  const serialized = sheet.serialize();
  const reparsed = new CSSStyleSheet();
  reparsed.replaceSync(serialized);
  expect(reparsed.serialize()).toBe(serialized);
});
