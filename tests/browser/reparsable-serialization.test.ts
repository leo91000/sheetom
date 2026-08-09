import { expect, test } from "vitest";

import {
  CSSStyleRule as SheetOMStyleRule,
  CSSStyleSheet as SheetOMStyleSheet,
} from "../../src/index.js";

test("reparsable serialization remains confined in a native browser stylesheet", () => {
  const sheet = new SheetOMStyleSheet();
  sheet.insertRule(".recovered { color: green; }");
  sheet.insertRule(".following { color: blue; }");
  const recovered = sheet.cssRules[0];
  expect(recovered).toBeInstanceOf(SheetOMStyleRule);
  if (!(recovered instanceof SheetOMStyleRule)) return;
  recovered.style.setProperty("padding", "72px var(--space, var(--space,");

  const nativeSheet = new globalThis.CSSStyleSheet();
  nativeSheet.replaceSync(sheet.serialize());

  expect(nativeSheet.cssRules).toHaveLength(2);
  const nativeRecovered = nativeSheet.cssRules[0] as globalThis.CSSStyleRule;
  const nativeFollowing = nativeSheet.cssRules[1] as globalThis.CSSStyleRule;
  expect(nativeRecovered.style.getPropertyValue("color")).toBe("green");
  expect(nativeRecovered.style.getPropertyValue("padding")).not.toBe("");
  expect(nativeFollowing.style.getPropertyValue("color")).toBe("blue");
});

test("repaired custom properties preserve measured substitution behavior", () => {
  const cases = [
    { property: "content", malformed: "\"hello" },
    { property: "width", malformed: "calc(10px" },
    { property: "background-image", malformed: "linear-gradient(red, blue" },
  ] as const;
  const sheet = new SheetOMStyleSheet();

  for (const [index, candidate] of cases.entries()) {
    sheet.insertRule(`.sheetom-substitution-${index} {}`);
    const rule = sheet.cssRules[index];
    expect(rule).toBeInstanceOf(SheetOMStyleRule);
    if (!(rule instanceof SheetOMStyleRule)) continue;
    rule.style.setProperty("--x", candidate.malformed);
    rule.style.setProperty(candidate.property, "var(--x)");
  }

  const style = document.createElement("style");
  style.textContent = sheet.serialize();
  document.head.append(style);
  const elements: HTMLElement[] = [];

  try {
    for (const [index, candidate] of cases.entries()) {
      const reference = document.createElement("div");
      reference.style.setProperty("--x", candidate.malformed);
      reference.style.setProperty(candidate.property, "var(--x)");
      const serialized = document.createElement("div");
      serialized.className = `sheetom-substitution-${index}`;
      document.body.append(reference, serialized);
      elements.push(reference, serialized);

      const referenceValue = getComputedStyle(reference)
        .getPropertyValue(candidate.property);
      const serializedValue = getComputedStyle(serialized)
        .getPropertyValue(candidate.property);
      expect(serializedValue).toBe(referenceValue);
    }
  } finally {
    style.remove();
    for (const element of elements) element.remove();
  }
});
