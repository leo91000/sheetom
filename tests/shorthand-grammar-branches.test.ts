import assert from "node:assert/strict";
import { test } from "vitest";

import { CSSStyleRule, CSSStyleSheet } from "../src/index.js";

function createRule(): CSSStyleRule {
  const sheet = new CSSStyleSheet();
  sheet.insertRule(".x {}");
  const rule = sheet.cssRules[0];
  assert.ok(rule instanceof CSSStyleRule);
  return rule;
}

test("repeated logical and corner values match Chromium branches", () => {
  const cases = [
    {
      property: "border-block-color",
      input: "red blue",
      items: ["border-block-start-color", "border-block-end-color"],
      values: ["red", "blue"],
    },
    {
      property: "border-inline-style",
      input: "dotted double",
      items: ["border-inline-start-style", "border-inline-end-style"],
      values: ["dotted", "double"],
    },
    {
      property: "border-block-width",
      input: "1px 2px",
      items: ["border-block-start-width", "border-block-end-width"],
      values: ["1px", "2px"],
    },
    {
      property: "border-inline-color",
      input: "red blue",
      items: ["border-inline-start-color", "border-inline-end-color"],
      values: ["red", "blue"],
    },
    {
      property: "corner-shape",
      input: "round bevel scoop notch",
      items: [
        "corner-top-left-shape",
        "corner-top-right-shape",
        "corner-bottom-right-shape",
        "corner-bottom-left-shape",
      ],
      values: ["round", "bevel", "scoop", "notch"],
    },
    {
      property: "corner-block-start-shape",
      input: "round bevel",
      items: ["corner-start-start-shape", "corner-start-end-shape"],
      values: ["round", "bevel"],
    },
    {
      property: "corner-inline-end-shape",
      input: "round bevel",
      items: ["corner-start-end-shape", "corner-end-end-shape"],
      values: ["round", "bevel"],
    },
  ] as const;

  for (const fixture of cases) {
    const rule = createRule();
    rule.style.setProperty(fixture.property, fixture.input);
    assert.deepEqual(
      Array.from({ length: rule.style.length }, (_, index) => rule.style.item(index)),
      fixture.items,
      fixture.property,
    );
    assert.deepEqual(
      fixture.items.map(item => rule.style.getPropertyValue(item)),
      fixture.values,
      fixture.property,
    );
    assert.equal(rule.style.getPropertyValue(fixture.property), fixture.input, fixture.property);
    assert.equal(rule.style.cssText, `${fixture.property}: ${fixture.input};`, fixture.property);
  }

  const mutable = createRule();
  mutable.style.setProperty("corner-shape", "round bevel scoop notch");
  const accepted = mutable.style.cssText;
  mutable.style.setProperty("corner-shape", "round bevel scoop notch square");
  assert.equal(mutable.style.cssText, accepted);
  mutable.style.setProperty("corner-top-right-shape", "square");
  mutable.style.removeProperty("corner-top-right-shape");
  assert.equal(mutable.style.getPropertyValue("corner-shape"), "");
});

test("compound placement values expand and synthesize like Chromium", () => {
  const cases = [
    {
      property: "place-content",
      input: "safe center space-evenly",
      items: ["align-content", "justify-content"],
      values: ["safe center", "space-evenly"],
    },
    {
      property: "place-self",
      input: "safe end self-start",
      items: ["align-self", "justify-self"],
      values: ["safe end", "self-start"],
    },
    {
      property: "place-items",
      input: "safe center self-start",
      items: ["align-items", "justify-items"],
      values: ["safe center", "self-start"],
    },
  ] as const;

  for (const fixture of cases) {
    const rule = createRule();
    rule.style.setProperty(fixture.property, fixture.input);
    assert.deepEqual(
      Array.from({ length: rule.style.length }, (_, index) => rule.style.item(index)),
      fixture.items,
      fixture.property,
    );
    assert.deepEqual(
      fixture.items.map(item => rule.style.getPropertyValue(item)),
      fixture.values,
      fixture.property,
    );
    assert.equal(rule.style.getPropertyValue(fixture.property), fixture.input, fixture.property);
    assert.equal(rule.style.cssText, `${fixture.property}: ${fixture.input};`, fixture.property);
  }
});

test("parallel transition and timeline lists match Chromium", () => {
  const cases = [
    {
      property: "transition",
      input: "opacity 1s ease 0s, transform 2s linear 1s allow-discrete",
      items: [
        "transition-behavior",
        "transition-duration",
        "transition-timing-function",
        "transition-delay",
        "transition-property",
      ],
      values: [
        "normal, allow-discrete",
        "1s, 2s",
        "ease, linear",
        "0s, 1s",
        "opacity, transform",
      ],
      observable: "opacity 1s, transform 2s linear 1s allow-discrete",
    },
    {
      property: "scroll-timeline",
      input: "--x block, --y inline",
      items: ["scroll-timeline-name", "scroll-timeline-axis"],
      values: ["--x, --y", "block, inline"],
      observable: "--x, --y inline",
    },
    {
      property: "view-timeline",
      input: "--x block 10% 20%, --y inline auto",
      items: ["view-timeline-name", "view-timeline-axis", "view-timeline-inset"],
      values: ["--x, --y", "block, inline", "10% 20%, auto"],
      observable: "--x 10% 20%, --y inline",
    },
  ] as const;

  for (const fixture of cases) {
    const rule = createRule();
    rule.style.setProperty(fixture.property, fixture.input);
    assert.deepEqual(
      Array.from({ length: rule.style.length }, (_, index) => rule.style.item(index)),
      fixture.items,
      fixture.property,
    );
    assert.deepEqual(
      fixture.items.map(item => rule.style.getPropertyValue(item)),
      fixture.values,
      fixture.property,
    );
    assert.equal(
      rule.style.getPropertyValue(fixture.property),
      fixture.observable,
      fixture.property,
    );
    assert.equal(
      rule.style.cssText,
      `${fixture.property}: ${fixture.observable};`,
      fixture.property,
    );
  }
});

test("a negative transition time fills delay rather than duration", () => {
  const rule = createRule();
  rule.style.setProperty("transition", "none 1s linear -1s normal");

  assert.equal(rule.style.getPropertyValue("transition-duration"), "1s");
  assert.equal(rule.style.getPropertyValue("transition-delay"), "-1s");
  assert.equal(rule.style.getPropertyValue("transition"), "none 1s linear -1s");
  assert.equal(rule.style.cssText, "transition: none 1s linear -1s;");

  const delayOnly = createRule();
  delayOnly.style.setProperty("transition", "-1s");
  assert.equal(delayOnly.style.getPropertyValue("transition-duration"), "0s");
  assert.equal(delayOnly.style.getPropertyValue("transition-delay"), "-1s");
  assert.equal(delayOnly.style.getPropertyValue("transition"), "-1s");
  assert.equal(delayOnly.style.cssText, "transition: -1s;");
});

test("text-box accepts a two-keyword edge branch", () => {
  const rule = createRule();
  rule.style.setProperty("text-box", "trim-both cap alphabetic");
  assert.deepEqual(
    Array.from({ length: rule.style.length }, (_, index) => rule.style.item(index)),
    ["text-box-trim", "text-box-edge"],
  );
  assert.equal(rule.style.getPropertyValue("text-box-trim"), "trim-both");
  assert.equal(rule.style.getPropertyValue("text-box-edge"), "cap alphabetic");
  assert.equal(rule.style.getPropertyValue("text-box"), "cap alphabetic");
  assert.equal(rule.style.cssText, "text-box: cap alphabetic;");
});

test("observable shorthand synthesis preserves Chromium list compression", () => {
  const cases = [
    {
      property: "animation",
      input: "1s ease foo, 2s linear bar",
      observable:
        "1s ease 0s 1 normal none running foo, 2s linear 0s 1 normal none running bar",
    },
    {
      property: "overscroll-behavior",
      input: "contain none",
      observable: "contain none",
    },
    {
      property: "background",
      input: "linear-gradient(red, blue)",
      observable: "linear-gradient(red, blue)",
    },
  ] as const;

  for (const fixture of cases) {
    const rule = createRule();
    rule.style.setProperty(fixture.property, fixture.input);
    assert.equal(
      rule.style.getPropertyValue(fixture.property),
      fixture.observable,
      fixture.property,
    );
    assert.equal(
      rule.style.cssText,
      `${fixture.property}: ${fixture.observable};`,
      fixture.property,
    );
  }
});
