import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";

import { chromium } from "playwright";

import { CSSStyleSheet, parseStyleSheet } from "../dist/index.js";

const functionRuleCorpus = JSON.parse(await readFile(
  new URL("../compatibility/function-rule-cases.json", import.meta.url),
  "utf8",
));

const functionPreludeTypes = [
  "",
  " <length>",
  " type(<length> | auto)",
  " <color>#",
  " foo+",
  " type(*)",
  " <dino>",
];
const functionPreludeDefaults = [
  "",
  ":",
  ": 1px",
  ": auto",
  ": red",
  ": calc(1px + 2px)",
  ": var(--x)",
  ": var(foo)",
  ": env()",
  ": --fallback()",
  ": --fallback(,)",
  ": foo(a,b)",
  ": {a,b}",
  ": !",
  ": 1px !important",
];
const functionPreludeReturns = [
  "",
  " returns <length>",
  " returns type(<length> | auto)",
  " returns type(*)",
  " returns auto",
  " returns *",
  " returns <transform-list>#",
  " return <length>",
];
const functionPreludeBoundaries = [["", ""], ["/**/", "/**/"], [" ", " "]];
const functionPreludeMatrix = [];
for (const type of functionPreludeTypes) {
  for (const defaultValue of functionPreludeDefaults) {
    for (const returnType of functionPreludeReturns) {
      for (const [before, after] of functionPreludeBoundaries) {
        functionPreludeMatrix.push(
          `@function --f(${before}--x${type}${defaultValue}${after})${returnType} {}`,
        );
      }
    }
  }
}

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
  {
    id: "layer-statement-interface",
    source: "@layer reset, theme.components;",
  },
  {
    id: "namespace-interface",
    source: '@namespace \\73 vg "urn:svg";',
  },
  {
    id: "font-palette-values-interface",
    source: '@font-palette-values --brand { font-family: "A B", Test; base-palette: invalid; base-palette: 2; override-colors: 0 red, 3 #00ff00; unknown: x; }',
  },
  {
    id: "view-transition-interface",
    source: "@view-transition { navigation: bad; navigation: auto; types: old; types: foo\\ bar \\62 az; unknown: x; }",
  },
  {
    id: "custom-function-interface",
    source: "@function --mix(--x <number>: 1, --color type(<color> | none): red, --rest type(*)) returns <number> { --local: if(style(--x: 1): red; else: blue); result: calc(var(--x) * 2); @supports (width: 100px) { result: 100px; } --tail: 2; }",
  },
  {
    id: "custom-function-escapes-and-component-blocks",
    source: "@function --escaped-\\9 -tab(--param-\\9 -tab) { --fn: foo(a;b); --square: [a;b]; --curly: {a;b} tail; result: ok; }",
  },
  {
    id: "custom-function-invalid-substitution-default",
    source: "@function --default(--x <length>: var(foo)) { result: 1px; }",
  },
  {
    id: "custom-function-ignored-at-rules",
    source: "@function --ignored() { --before: 1; @layer ignored {} --after-layer: 2; @unknown ignored {} --after-unknown: 3; }",
  },
  {
    id: "custom-function-invalid-style-recovery",
    source: "@function --invalid-style() { --before: 1; @media (width: 1px) { .invalid {} result: 2; } --after: 3; }",
  },
  {
    id: "invalid-known-custom-function",
    source: "@function --invalid(--x <length>: 10deg) { result: 1; }",
    constructed: true,
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
    type: style.constructor.name,
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
  if (rule.constructor.name === "CSSLayerStatementRule") {
    snapshot.cssText = rule.cssText;
    snapshot.nameList = [...rule.nameList];
  }
  if (rule.constructor.name === "CSSNamespaceRule") {
    snapshot.cssText = rule.cssText;
    snapshot.namespaceURI = rule.namespaceURI;
    snapshot.prefix = rule.prefix;
  }
  if (rule.constructor.name === "CSSFontPaletteValuesRule") {
    snapshot.cssText = rule.cssText;
    for (const field of ["name", "fontFamily", "basePalette", "overrideColors"]) {
      snapshot[field] = rule[field];
    }
  }
  if (rule.constructor.name === "CSSViewTransitionRule") {
    snapshot.cssText = rule.cssText;
    snapshot.navigation = rule.navigation;
    snapshot.types = [...rule.types];
  }
  if (rule.constructor.name === "CSSFunctionRule") {
    snapshot.cssText = rule.cssText;
    snapshot.returnType = rule.returnType;
    snapshot.parameters = rule.getParameters();
  }
  return snapshot;
}

function sheetOMRuleList(testCase) {
  if (!testCase.constructed) return parseStyleSheet(testCase.source).cssRules;
  const sheet = new CSSStyleSheet();
  sheet.replaceSync(testCase.source);
  return sheet.cssRules;
}

function functionPreludeSnapshot(rule) {
  if (!rule) return null;
  return {
    cssText: rule.cssText,
    name: rule.name,
    returnType: rule.returnType,
    parameters: rule.getParameters().map(parameter => [
      Object.keys(parameter),
      parameter.name,
      parameter.type,
      Object.hasOwn(parameter, "defaultValue"),
      parameter.defaultValue ?? null,
    ]),
  };
}

const actual = cases.map(testCase => ({
  id: testCase.id,
  rules: Array.from(sheetOMRuleList(testCase), ruleSnapshot),
}));

const browser = await chromium.launch({ headless: true });
try {
  const page = await browser.newPage();
  assert.equal(
    browser.version(),
    functionRuleCorpus.baseline.version,
    "Function Rule corpus must run against its pinned Chromium version",
  );
  const sheetOMPreludeAcceptance = functionRuleCorpus.cases.map(testCase => {
    const sheet = new CSSStyleSheet();
    sheet.replaceSync(`${testCase.prelude} {}`);
    return sheet.cssRules.length === 1;
  });
  const chromiumPreludeAcceptance = await page.evaluate(testCases => testCases.map(testCase => {
    const sheet = new CSSStyleSheet();
    sheet.replaceSync(`${testCase.prelude} {}`);
    return sheet.cssRules.length === 1;
  }), functionRuleCorpus.cases);
  for (let index = 0; index < functionRuleCorpus.cases.length; index += 1) {
    const testCase = functionRuleCorpus.cases[index];
    assert.equal(chromiumPreludeAcceptance[index], testCase.accepted, testCase.id);
    assert.equal(sheetOMPreludeAcceptance[index], testCase.accepted, testCase.id);
  }
  const sheetOMPreludeMatrix = functionPreludeMatrix.map(source => {
    const sheet = new CSSStyleSheet();
    sheet.replaceSync(source);
    return functionPreludeSnapshot(sheet.cssRules[0]);
  });
  const chromiumPreludeMatrix = await page.evaluate(sources => sources.map(source => {
    const sheet = new CSSStyleSheet();
    sheet.replaceSync(source);
    const rule = sheet.cssRules[0];
    if (!rule) return null;
    return {
      cssText: rule.cssText,
      name: rule.name,
      returnType: rule.returnType,
      parameters: rule.getParameters().map(parameter => [
        Object.keys(parameter),
        parameter.name,
        parameter.type,
        Object.hasOwn(parameter, "defaultValue"),
        parameter.defaultValue ?? null,
      ]),
    };
  }), functionPreludeMatrix);
  assert.deepEqual(
    sheetOMPreludeMatrix,
    chromiumPreludeMatrix,
    "custom-function combinatorial prelude matrix",
  );
  const expected = await page.evaluate(testCases => {
    const styleSnapshot = style => {
      if (!style) return null;
      const items = Array.from({ length: style.length }, (_, index) => style.item(index));
      return {
        type: style.constructor.name,
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
      if (rule.constructor.name === "CSSLayerStatementRule") {
        snapshot.cssText = rule.cssText;
        snapshot.nameList = [...rule.nameList];
      }
      if (rule.constructor.name === "CSSNamespaceRule") {
        snapshot.cssText = rule.cssText;
        snapshot.namespaceURI = rule.namespaceURI;
        snapshot.prefix = rule.prefix;
      }
      if (rule.constructor.name === "CSSFontPaletteValuesRule") {
        snapshot.cssText = rule.cssText;
        for (const field of ["name", "fontFamily", "basePalette", "overrideColors"]) {
          snapshot[field] = rule[field];
        }
      }
      if (rule.constructor.name === "CSSViewTransitionRule") {
        snapshot.cssText = rule.cssText;
        snapshot.navigation = rule.navigation;
        snapshot.types = [...rule.types];
      }
      if (rule.constructor.name === "CSSFunctionRule") {
        snapshot.cssText = rule.cssText;
        snapshot.returnType = rule.returnType;
        snapshot.parameters = rule.getParameters();
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

  const functionSource = "@function --f(--x <length>: 1px) returns <length> { --local: 1px; result: 2px; }";
  const functionSheet = parseStyleSheet(functionSource);
  const functionRule = functionSheet.cssRules[0];
  const functionDescriptors = functionRule.cssRules[0].style;
  const actualFunctionMutations = [ruleSnapshot(functionRule)];
  functionDescriptors.cssText = "--local: 3px; color: red; result: 4px; --ignored: 5px !important; --tail: foo(a;b);";
  actualFunctionMutations.push(ruleSnapshot(functionRule));
  functionDescriptors.setProperty("result", "5px");
  actualFunctionMutations.push(ruleSnapshot(functionRule));
  const actualRemovedResult = functionDescriptors.removeProperty("result");
  actualFunctionMutations.push(ruleSnapshot(functionRule));
  functionDescriptors.result = "6px";
  actualFunctionMutations.push(ruleSnapshot(functionRule));
  functionDescriptors.cssText = "--local: 3px; result: 7px;";
  actualFunctionMutations.push(ruleSnapshot(functionRule));
  functionDescriptors.setProperty("result", "", "bogus");
  actualFunctionMutations.push(ruleSnapshot(functionRule));
  functionDescriptors.setProperty("result", "");
  actualFunctionMutations.push(ruleSnapshot(functionRule));
  functionRule.insertRule("@media (width:1px){result:3px;}", functionRule.cssRules.length);
  actualFunctionMutations.push(ruleSnapshot(functionRule));
  functionRule.insertRule("@media (width:1px){.x{}}", functionRule.cssRules.length);
  actualFunctionMutations.push(ruleSnapshot(functionRule));

  const expectedFunctionMutation = await page.evaluate(source => {
    const styleSnapshot = style => {
      if (!style) return null;
      const items = Array.from({ length: style.length }, (_, index) => style.item(index));
      return {
        type: style.constructor.name,
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
      if (rule.constructor.name === "CSSFunctionRule") {
        snapshot.cssText = rule.cssText;
        snapshot.returnType = rule.returnType;
        snapshot.parameters = rule.getParameters();
      }
      return snapshot;
    };

    const sheet = new CSSStyleSheet();
    sheet.replaceSync(source);
    const rule = sheet.cssRules[0];
    const descriptors = rule.cssRules[0].style;
    const snapshots = [ruleSnapshot(rule)];
    descriptors.cssText = "--local: 3px; color: red; result: 4px; --ignored: 5px !important; --tail: foo(a;b);";
    snapshots.push(ruleSnapshot(rule));
    descriptors.setProperty("result", "5px");
    snapshots.push(ruleSnapshot(rule));
    const removedResult = descriptors.removeProperty("result");
    snapshots.push(ruleSnapshot(rule));
    descriptors.result = "6px";
    snapshots.push(ruleSnapshot(rule));
    descriptors.cssText = "--local: 3px; result: 7px;";
    snapshots.push(ruleSnapshot(rule));
    descriptors.setProperty("result", "", "bogus");
    snapshots.push(ruleSnapshot(rule));
    descriptors.setProperty("result", "");
    snapshots.push(ruleSnapshot(rule));
    rule.insertRule("@media (width:1px){result:3px;}", rule.cssRules.length);
    snapshots.push(ruleSnapshot(rule));
    rule.insertRule("@media (width:1px){.x{}}", rule.cssRules.length);
    snapshots.push(ruleSnapshot(rule));
    return { snapshots, removedResult };
  }, functionSource);
  assert.equal(actualRemovedResult, expectedFunctionMutation.removedResult);
  assert.deepEqual(
    actualFunctionMutations,
    expectedFunctionMutation.snapshots,
    "custom-function descriptor and grouping mutations",
  );

  const functionInsertionSources = [
    "@media(width:1px){result:3px;} trailing",
    "@media(width:1px){result:3px;} @bad",
    "@media(width:1px){result:3px;} .x{}",
    "@media(width:1px){result:3px;} ;",
    "@media(width:1px){result:3px;} color:red",
    "/* before */ @media(width:1px){result:3px;} /* recovered EOF",
  ];
  const actualFunctionInsertions = functionInsertionSources.map(source => {
    const rule = parseStyleSheet("@function --f(){}").cssRules[0];
    try {
      const index = rule.insertRule(source);
      return { index, cssText: rule.cssText };
    } catch (error) {
      return { error: error.name };
    }
  });
  const expectedFunctionInsertions = await page.evaluate(sources => sources.map(source => {
    const sheet = new CSSStyleSheet();
    sheet.replaceSync("@function --f(){}");
    const rule = sheet.cssRules[0];
    try {
      const index = rule.insertRule(source);
      return { index, cssText: rule.cssText };
    } catch (error) {
      return { error: error.name };
    }
  }), functionInsertionSources);
  assert.deepEqual(
    actualFunctionInsertions,
    expectedFunctionInsertions,
    "custom-function insertRule exact-source recovery",
  );
} finally {
  await browser.close();
}

console.log(
  `${cases.length} native-backed public rule trees, ${functionRuleCorpus.cases.length} versioned custom-function preludes, and ${functionPreludeMatrix.length} combinatorial preludes match Chromium.`,
);
