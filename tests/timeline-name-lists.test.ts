import assert from "node:assert/strict";
import { test } from "vitest";

import { CSSStyleRule, CSSStyleSheet } from "../src/index.js";

function createRule(): CSSStyleRule {
  const sheet = new CSSStyleSheet();
  sheet.insertRule(".timeline {}");
  const rule = sheet.cssRules[0];
  assert.ok(rule instanceof CSSStyleRule);
  return rule;
}

test("timeline name longhands own mixed none and dashed-ident lists", () => {
  for (const property of ["scroll-timeline-name", "view-timeline-name"]) {
    const rule = createRule();
    rule.style.setProperty(property, "none,--x,none", "important");

    assert.equal(rule.style.getPropertyValue(property), "none, --x, none");
    assert.equal(rule.style.getPropertyPriority(property), "important");
    assert.equal(rule.style.cssText, `${property}: none, --x, none !important;`);

    const before = rule.style.cssText;
    for (const invalid of ["none none", "all", "none,", "none,,--x"]) {
      rule.style.setProperty(property, invalid);
      assert.equal(rule.style.cssText, before, `${property}: ${invalid}`);
    }
  }
});

test("timeline shorthands expand, synthesize, mutate and remove list entries", () => {
  const cases = [
    {
      property: "scroll-timeline",
      value: "none inline, --x x",
      longhands: ["scroll-timeline-name", "scroll-timeline-axis"],
      expected: ["none, --x", "inline, x"],
    },
    {
      property: "view-timeline",
      value: "none 10% 20%, --x inline auto",
      longhands: ["view-timeline-name", "view-timeline-axis", "view-timeline-inset"],
      expected: ["none, --x", "block, inline", "10% 20%, auto"],
    },
  ] as const;

  for (const fixture of cases) {
    const rule = createRule();
    rule.style.setProperty(fixture.property, fixture.value);
    assert.deepEqual(
      fixture.longhands.map(property => rule.style.getPropertyValue(property)),
      fixture.expected,
    );
    assert.equal(rule.style.getPropertyValue(fixture.property), fixture.value.replace(" auto", ""));

    const removed = fixture.longhands[0];
    assert.equal(rule.style.removeProperty(removed), fixture.expected[0]);
    assert.equal(rule.style.getPropertyValue(fixture.property), "");
    assert.equal(rule.style.getPropertyValue(removed), "");
  }
});

test("invalid timeline list mutations are atomic", () => {
  for (const property of ["scroll-timeline", "view-timeline"]) {
    const rule = createRule();
    rule.style.setProperty(property, "none, none block", "important");
    const before = rule.style.cssText;

    for (const invalid of ["none none", "block", "none,", "none,,--x"]) {
      rule.style.setProperty(property, invalid);
      assert.equal(rule.style.cssText, before, `${property}: ${invalid}`);
    }
  }
});
