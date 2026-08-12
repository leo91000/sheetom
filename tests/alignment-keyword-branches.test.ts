import { expect, test } from "vitest";

import { CSSStyleRule, CSSStyleSheet } from "../src/index.js";

function createStyle() {
  const sheet = new CSSStyleSheet();
  sheet.insertRule(".aligned {}");
  const rule = sheet.cssRules[0];
  if (!(rule instanceof CSSStyleRule)) throw new TypeError("Expected a style rule");
  return rule.style;
}

test("self alignment owns anchored overflow positions", () => {
  for (const property of ["align-self", "justify-self"]) {
    for (const value of ["anchor-center", "safe anchor-center", "unsafe anchor-center"]) {
      const style = createStyle();
      style.setProperty(property, value, "important");
      expect(style.getPropertyValue(property)).toBe(value);
      expect(style.getPropertyPriority(property)).toBe("important");

      const before = style.cssText;
      for (const invalid of ["anchor-center anchor-center", "safe safe anchor-center", "legacy"]) {
        style.setProperty(property, invalid);
        expect(style.cssText, `${property}: ${invalid}`).toBe(before);
      }
    }
  }
});

test("justify-items owns bare and paired legacy values", () => {
  for (const [input, expected] of [
    ["legacy", "legacy"],
    ["legacy left", "legacy left"],
    ["left legacy", "legacy left"],
    ["center legacy", "legacy center"],
  ] as const) {
    const style = createStyle();
    style.setProperty("justify-items", input);
    expect(style.getPropertyValue("justify-items")).toBe(expected);

    const before = style.cssText;
    for (const invalid of ["legacy auto", "legacy start", "anchor-center"]) {
      style.setProperty("justify-items", invalid);
      expect(style.cssText, invalid).toBe(before);
    }
  }
});

test("place shorthands expand, synthesize, mutate and reject adjacent invalid values", () => {
  const cases = [
    {
      shorthand: "place-self",
      input: "safe anchor-center unsafe anchor-center",
      longhands: ["align-self", "justify-self"],
      expected: ["safe anchor-center", "unsafe anchor-center"],
      invalid: "auto legacy",
    },
    {
      shorthand: "place-items",
      input: "normal legacy",
      longhands: ["align-items", "justify-items"],
      expected: ["normal", "legacy"],
      invalid: "normal anchor-center",
    },
  ] as const;

  for (const fixture of cases) {
    const style = createStyle();
    style.setProperty(fixture.shorthand, fixture.input, "important");
    expect(fixture.longhands.map(property => style.getPropertyValue(property))).toEqual(
      fixture.expected,
    );
    expect(style.getPropertyValue(fixture.shorthand)).toBe(fixture.input);

    const before = style.cssText;
    style.setProperty(fixture.shorthand, fixture.invalid);
    expect(style.cssText).toBe(before);

    const removed = fixture.longhands[1];
    expect(style.removeProperty(removed)).toBe(fixture.expected[1]);
    expect(style.getPropertyValue(fixture.shorthand)).toBe("");
  }
});
