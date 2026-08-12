import assert from "node:assert/strict";
import { test } from "vitest";

import { CSSStyleRule, CSSStyleSheet } from "../src/index.js";

function createRule(): CSSStyleRule {
  const sheet = new CSSStyleSheet();
  sheet.insertRule(".keywords {}");
  const rule = sheet.cssRules[0];
  assert.ok(rule instanceof CSSStyleRule);
  return rule;
}

test("current keyword and ordered-set branches expose Chromium CSSOM state", () => {
  const cases = [
    ["all", "revert-rule", "all", "revert-rule", "all: revert-rule;"],
    ["word-break", "auto-phrase", "word-break", "auto-phrase", "word-break: auto-phrase;"],
    [
      "-webkit-transform-style",
      "preserve-3d",
      "transform-style",
      "preserve-3d",
      "transform-style: preserve-3d;",
    ],
    [
      "image-rendering",
      "crisp-edges",
      "image-rendering",
      "crisp-edges",
      "image-rendering: crisp-edges;",
    ],
    ["display", "inline math", "display", "math", "display: math;"],
    ["grid-auto-flow", "dense row", "grid-auto-flow", "dense", "grid-auto-flow: dense;"],
    [
      "scroll-marker-group",
      "before links",
      "scroll-marker-group",
      "before links",
      "scroll-marker-group: before links;",
    ],
    [
      "scrollbar-gutter",
      "both-edges stable",
      "scrollbar-gutter",
      "stable both-edges",
      "scrollbar-gutter: stable both-edges;",
    ],
    ["page-break-before", "always", "break-before", "always", "break-before: page;"],
    [
      "-webkit-column-break-before",
      "always",
      "break-before",
      "",
      "break-before: column;",
    ],
  ] as const;

  for (const [property, input, item, observable, cssText] of cases) {
    const rule = createRule();
    rule.style.setProperty(property, input);
    assert.equal(rule.style.length, 1, `${property}: ${input}`);
    assert.equal(rule.style.item(0), item, `${property}: ${input} item`);
    assert.equal(
      rule.style.getPropertyValue(property),
      observable,
      `${property}: ${input} getter`,
    );
    assert.equal(rule.style.cssText, cssText, `${property}: ${input} cssText`);
  }
});

test("neighboring invalid current grammar values remain atomic no-ops", () => {
  for (const [property, valid, invalid] of [
    ["all", "revert-rule", "revert-rule extra"],
    ["word-break", "auto-phrase", "normal auto-phrase"],
    ["transform-style", "preserve-3d", "preserve3d"],
    ["image-rendering", "pixelated", "pixelated crisp-edges"],
    ["display", "math", "math list-item"],
    ["grid-auto-flow", "dense", "dense dense"],
    ["scroll-marker-group", "before links", "links before"],
    ["scrollbar-gutter", "stable both-edges", "both-edges"],
    ["page-break-before", "always", "page"],
    ["-webkit-column-break-before", "always", "page"],
  ] as const) {
    const rule = createRule();
    rule.style.setProperty(property, valid, "important");
    const before = rule.style.cssText;
    rule.style.setProperty(property, invalid);
    assert.equal(rule.style.cssText, before, `${property}: ${invalid}`);
  }
});

test("current grammar branches survive whole-sheet reparsing idempotently", () => {
  const sheet = new CSSStyleSheet();
  sheet.replaceSync(`
    .keywords {
      all: revert-rule;
      word-break: auto-phrase;
      transform-style: preserve-3d;
      image-rendering: pixelated;
      display: block math;
      grid-auto-flow: dense;
      scroll-marker-group: after tabs;
      scrollbar-gutter: both-edges stable;
    }
  `);
  const serialized = sheet.serialize();
  const reparsed = new CSSStyleSheet();
  reparsed.replaceSync(serialized);
  assert.equal(reparsed.serialize(), serialized);
});
