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
  {
    id: "counter-style-descriptors",
    source: '@counter-style digits { system: fixed; symbols: "a" "b"; additive-symbols: +010 "x", 001 "i"; negative: "(" ")"; prefix: "["; suffix: "]"; range: infinite -02, +004 infinite; pad: +03 "0"; speak-as: SPELL-OUT; fallback: DECIMAL; }',
  },
  {
    id: "counter-style-grammar-branches",
    source: '@counter-style \\62 ranches { system: extends none; symbols: foo/**/bar; additive-symbols: "x" 10, "i" 1; pad: "0" 2; fallback: none; speak-as: default; }',
  },
  {
    id: "font-feature-values-maps",
    source: '@font-feature-values "A B", Test { @styleset { a: 1; } @styleset { b: 2; a: 3; } @annotation { mark: 0; } @swash { good: 1; bad: -1; } }',
  },
  {
    id: "property-rule-descriptors",
    source: '@property --dynamic-width { syntax: "<length>"; inherits: false; initial-value: 0px; }',
  },
];

const counterStyleFields = [
  "system",
  "symbols",
  "additiveSymbols",
  "negative",
  "prefix",
  "suffix",
  "pad",
  "range",
  "fallback",
  "speakAs",
];
const fontFeatureMapFields = [
  "annotation",
  "ornaments",
  "stylistic",
  "swash",
  "characterVariant",
  "styleset",
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
  if (rule.constructor.name === "CSSCounterStyleRule") {
    snapshot.cssText = rule.cssText;
    for (const field of counterStyleFields) snapshot[field] = rule[field];
  }
  if (rule.constructor.name === "CSSFontFeatureValuesRule") {
    snapshot.cssText = rule.cssText;
    snapshot.fontFamily = rule.fontFamily;
    snapshot.featureMaps = Object.fromEntries(fontFeatureMapFields.map(field => [
      field,
      Array.from(rule[field], ([name, values]) => [name, [...values]]),
    ]));
  }
  if (rule.constructor.name === "CSSPropertyRule") {
    snapshot.cssText = rule.cssText;
    for (const field of ["name", "syntax", "inherits", "initialValue"]) {
      snapshot[field] = rule[field];
    }
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
      if (rule.constructor.name === "CSSCounterStyleRule") {
        snapshot.cssText = rule.cssText;
        for (const field of [
          "system",
          "symbols",
          "additiveSymbols",
          "negative",
          "prefix",
          "suffix",
          "pad",
          "range",
          "fallback",
          "speakAs",
        ]) snapshot[field] = rule[field];
      }
      if (rule.constructor.name === "CSSFontFeatureValuesRule") {
        const fields = [
          "annotation", "ornaments", "stylistic", "swash", "characterVariant", "styleset",
        ];
        snapshot.cssText = rule.cssText;
        snapshot.fontFamily = rule.fontFamily;
        snapshot.featureMaps = Object.fromEntries(fields.map(field => [
          field,
          Array.from(rule[field], ([name, values]) => [name, [...values]]),
        ]));
      }
      if (rule.constructor.name === "CSSPropertyRule") {
        snapshot.cssText = rule.cssText;
        for (const field of ["name", "syntax", "inherits", "initialValue"]) {
          snapshot[field] = rule[field];
        }
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

  const mutationSource = '@counter-style marks { system: cyclic; symbols: "a"; range: 1 10; }';
  const mutationOperations = [
    ["name", "bad name"],
    ["name", "--marks"],
    ["system", "numeric"],
    ["symbols", '"z"'],
    ["symbols", ""],
    ["symbols", '"x"; suffix: "evil"'],
    ["range", "auto"],
    ["range", "10 1"],
    ["fallback", "disc"],
  ];
  const sheet = parseStyleSheet(mutationSource);
  const rule = sheet.cssRules[0];
  const actualMutations = [];
  for (const [field, value] of mutationOperations) {
    rule[field] = value;
    actualMutations.push(ruleSnapshot(rule));
  }
  const expectedMutations = await page.evaluate(({ source, operations }) => {
    const sheet = new CSSStyleSheet();
    sheet.replaceSync(source);
    const rule = sheet.cssRules[0];
    const fields = [
      "system", "symbols", "additiveSymbols", "negative", "prefix", "suffix",
      "pad", "range", "fallback", "speakAs",
    ];
    const snapshot = () => ({
      type: rule.constructor.name,
      style: null,
      children: [],
      name: rule.name,
      cssText: rule.cssText,
      ...Object.fromEntries(fields.map(field => [field, rule[field]])),
    });
    return operations.map(([field, value]) => {
      rule[field] = value;
      return snapshot();
    });
  }, { source: mutationSource, operations: mutationOperations });
  assert.deepEqual(actualMutations, expectedMutations, "counter-style descriptor mutations");

  const featureSource = "@font-feature-values Test { @styleset { base: 1; } }";
  const featureSheet = parseStyleSheet(featureSource);
  const featureRule = featureSheet.cssRules[0];
  featureRule.styleset.set("bad name", [-1, 1.5, Number.NaN]);
  featureRule.annotation.set("", []);
  featureRule.styleset.delete("base");
  featureRule.fontFamily = '"A B", Test';
  const actualFeatureMutation = ruleSnapshot(featureRule);
  const expectedFeatureMutation = await page.evaluate(source => {
    const sheet = new CSSStyleSheet();
    sheet.replaceSync(source);
    const rule = sheet.cssRules[0];
    rule.styleset.set("bad name", [-1, 1.5, Number.NaN]);
    rule.annotation.set("", []);
    rule.styleset.delete("base");
    rule.fontFamily = '"A B", Test';
    const fields = [
      "annotation", "ornaments", "stylistic", "swash", "characterVariant", "styleset",
    ];
    return {
      type: rule.constructor.name,
      style: null,
      children: [],
      cssText: rule.cssText,
      fontFamily: rule.fontFamily,
      featureMaps: Object.fromEntries(fields.map(field => [
        field,
        Array.from(rule[field], ([name, values]) => [name, [...values]]),
      ])),
    };
  }, featureSource);
  assert.deepEqual(
    actualFeatureMutation,
    expectedFeatureMutation,
    "font-feature-values map mutations",
  );
} finally {
  await browser.close();
}

console.log(`${cases.length} native-backed public rule trees match Chromium.`);
