import assert from "node:assert/strict";

import { chromium } from "playwright";

import { parseStyleSheet } from "../dist/index.js";

const cases = [
  {
    id: "malformed-pending-substitution",
    source: ".x { padding: 72px var(--space, var(--space,; }",
  },
  {
    id: "layer-and-legacy-media",
    source: "@layer app { @media (max-width: 767px) { .x:hover { color: red; } } }",
  },
  {
    id: "legacy-container-query",
    source: "@container card (max-width: 767px) { .x { width: 1px; } }",
  },
  {
    id: "page-margin-rule",
    source: '@page :first { size: A4; @top-left { content: "x"; } }',
  },
  {
    id: "keyframes",
    source: "@keyframes fade { from { opacity: 0; } to { opacity: 1; } }",
  },
  {
    id: "nested-declarations",
    source: ".a { color: red; & .b { width: 1px; } height: 2px; }",
  },
  {
    id: "selector-serialization",
    source: ":is(.a,.b) > .child+.sibling { color: red; }",
  },
  {
    id: "media-list-serialization",
    source: "@media screen/**/and (max-width:767px),print { .x { color: red; } }",
  },
  {
    id: "supports-serialization",
    source: "@supports (display:grid) and (not (color:contrast-color(red))) { .x { color: red; } }",
  },
  {
    id: "container-style-query",
    source: "@container style(--theme:dark) { .x { color: red; } }",
  },
  {
    id: "scope-selector-list",
    source: "@scope (.a,.b) to (:is(.c,.d)) { .x { color: red; } }",
  },
];

function styleSnapshot(style) {
  if (!style) return null;
  const items = Array.from({ length: style.length }, (_, index) => style.item(index));
  return {
    cssText: style.cssText,
    items,
    declarations: items.map(name => ({
      name,
      value: style.getPropertyValue(name),
      priority: style.getPropertyPriority(name),
    })),
  };
}

function ruleSnapshot(rule) {
  const snapshot = {
    type: rule.constructor.name,
    style: styleSnapshot(rule.style),
    children: "cssRules" in rule ? Array.from(rule.cssRules, ruleSnapshot) : [],
  };
  for (const field of ["selectorText", "conditionText", "name", "keyText"]) {
    if (typeof rule[field] === "string") snapshot[field] = rule[field];
  }
  if (rule.media && typeof rule.media.mediaText === "string") {
    snapshot.mediaText = rule.media.mediaText;
  }
  return snapshot;
}

const actual = cases.map(testCase => ({
  id: testCase.id,
  rules: Array.from(parseStyleSheet(testCase.source).cssRules, ruleSnapshot),
}));

const browser = await chromium.launch({ headless: true });
try {
  const page = await browser.newPage();
  const expected = await page.evaluate(testCases => {
    const styleSnapshot = style => {
      if (!style) return null;
      const items = Array.from({ length: style.length }, (_, index) => style.item(index));
      return {
        cssText: style.cssText,
        items,
        declarations: items.map(name => ({
          name,
          value: style.getPropertyValue(name),
          priority: style.getPropertyPriority(name),
        })),
      };
    };
    const ruleSnapshot = rule => {
      const snapshot = {
        type: rule.constructor.name,
        style: styleSnapshot(rule.style),
        children: "cssRules" in rule ? Array.from(rule.cssRules, ruleSnapshot) : [],
      };
      for (const field of ["selectorText", "conditionText", "name", "keyText"]) {
        if (typeof rule[field] === "string") snapshot[field] = rule[field];
      }
      if (rule.media && typeof rule.media.mediaText === "string") {
        snapshot.mediaText = rule.media.mediaText;
      }
      return snapshot;
    };
    return testCases.map(testCase => {
      const sheet = new CSSStyleSheet();
      sheet.replaceSync(testCase.source);
      return {
        id: testCase.id,
        rules: Array.from(sheet.cssRules, ruleSnapshot),
      };
    });
  }, cases);

  for (let index = 0; index < cases.length; index += 1) {
    assert.deepEqual(actual[index], expected[index], cases[index].id);
  }
} finally {
  await browser.close();
}

console.log(`${cases.length} native-backed public rule trees match Chromium.`);
