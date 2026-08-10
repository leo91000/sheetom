import assert from "node:assert/strict";
import { test } from "vitest";

import {
  CSSContainerRule,
  CSSFunctionDeclarations,
  CSSFunctionDescriptors,
  CSSFunctionRule,
  CSSMediaRule,
  CSSStyleRule,
  CSSStyleSheet,
  CSSSupportsRule,
  parseStyleSheet,
} from "../src/index.js";

test("CSSFunctionRule exposes parameters, return types, escapes, and live descriptors", () => {
  const sheet = new CSSStyleSheet();
  sheet.replaceSync(String.raw`
    @function --f0() {}
    @function --f1(--x) {}
    @function --f2(--x <length>) {}
    @function --f3(--x: 10px) {}
    @function --f4(--x <length>: 10px) returns <length> {}
    @function --f5(--x type(<length> | auto): 10px, --y, --z: red)
      returns type(<length> | auto) {
      --local: 1px;
      syntax: "<length>";
      --local: 2px;
      color: red;
      result: 3px;
    }
    @function --escaped-\9 -tab(--param-\9 -tab) {
      --local-\9 -tab: 1px;
    }
  `);

  assert.equal(sheet.cssRules.length, 7);
  const functions = Array.from(sheet.cssRules);
  for (const rule of functions) assert.ok(rule instanceof CSSFunctionRule);
  assert.deepEqual((functions[0] as CSSFunctionRule).getParameters(), []);
  assert.deepEqual((functions[1] as CSSFunctionRule).getParameters(), [
    { name: "--x", type: "*" },
  ]);
  assert.deepEqual((functions[2] as CSSFunctionRule).getParameters(), [
    { name: "--x", type: "<length>" },
  ]);
  assert.deepEqual((functions[3] as CSSFunctionRule).getParameters(), [
    { name: "--x", type: "*", defaultValue: "10px" },
  ]);
  assert.equal((functions[4] as CSSFunctionRule).returnType, "<length>");
  assert.deepEqual((functions[5] as CSSFunctionRule).getParameters(), [
    { name: "--x", type: "<length> | auto", defaultValue: "10px" },
    { name: "--y", type: "*" },
    { name: "--z", type: "*", defaultValue: "red" },
  ]);
  assert.equal((functions[5] as CSSFunctionRule).returnType, "<length> | auto");

  const declarations = (functions[5] as CSSFunctionRule).cssRules[0];
  assert.ok(declarations instanceof CSSFunctionDeclarations);
  assert.ok(declarations.style instanceof CSSFunctionDescriptors);
  assert.equal(declarations.style.constructor, CSSFunctionDescriptors);
  assert.deepEqual(Array.from(declarations.style), ["--local", "result"]);
  assert.equal(declarations.style.getPropertyValue("--local"), "2px");
  assert.equal(declarations.style.result, "3px");
  assert.equal(declarations.style.cssText, "--local: 2px; result: 3px;");

  const escaped = functions[6] as CSSFunctionRule;
  assert.equal(escaped.name, "--escaped-\t-tab");
  assert.equal(escaped.getParameters()[0]?.name, "--param-\t-tab");
  const escapedDeclarations = escaped.cssRules[0];
  assert.ok(escapedDeclarations instanceof CSSFunctionDeclarations);
  assert.equal(escapedDeclarations.style.getPropertyValue("--local-\t-tab"), "1px");
});

test("custom function CSSOM interfaces reject direct construction", () => {
  assert.throws(() => new CSSFunctionRule("--f", [], "*"), TypeError);
  assert.throws(() => new CSSFunctionDeclarations(), TypeError);
  assert.throws(() => new CSSFunctionDescriptors(null as never), TypeError);
});

test("getParameters returns independent mutable records like Chromium", () => {
  const sheet = parseStyleSheet("@function --f(--x <length>: 1px) {}");
  const rule = sheet.cssRules[0];
  assert.ok(rule instanceof CSSFunctionRule);

  const first = rule.getParameters();
  const second = rule.getParameters();
  assert.notEqual(first, second);
  assert.notEqual(first[0], second[0]);
  assert.equal(Object.isFrozen(first), false);
  assert.equal(Object.isFrozen(first[0]), false);
  first[0]!.name = "--changed";
  assert.equal(rule.getParameters()[0]?.name, "--x");
  assert.deepEqual(Object.keys(rule.getParameters()[0]!), [
    "defaultValue",
    "name",
    "type",
  ]);
});

test("invalid substitution defaults are omitted without dropping the function", () => {
  for (const invalidDefault of [
    "var(foo)",
    "env()",
    "attr()",
    "if()",
    "--fallback(,)",
  ]) {
    const sheet = parseStyleSheet(
      `@function --f(--x <length>: ${invalidDefault}) {}`,
    );
    const rule = sheet.cssRules[0];
    assert.ok(rule instanceof CSSFunctionRule, invalidDefault);
    assert.deepEqual(rule.getParameters(), [{ name: "--x", type: "<length>" }]);
    assert.equal(rule.cssText, "@function --f(--x <length>) { }");

    const invalidSheet = new CSSStyleSheet();
    invalidSheet.replaceSync(
      `@function --f(--x <length>: ${invalidDefault} ) {}`,
    );
    assert.equal(invalidSheet.cssRules.length, 0, `${invalidDefault} + space`);
  }
});

test("function parameter grammar treats comments as CSS whitespace", () => {
  const sheet = parseStyleSheet(
    "@function --f(/**/--x/**/, --y/**/:/**/1px/**/) returns/**/<length> {}",
  );
  const rule = sheet.cssRules[0];
  assert.ok(rule instanceof CSSFunctionRule);
  assert.deepEqual(rule.getParameters(), [
    { name: "--x", type: "*" },
    { name: "--y", type: "*", defaultValue: "1px" },
  ]);
  assert.equal(
    rule.cssText,
    "@function --f(--x, --y: 1px) returns <length> { }",
  );
});

test("function declaration mutations filter unknown descriptors and preserve CSSOM order", () => {
  const sheet = parseStyleSheet("@function --f() { --x: 1px; result: 2px; }");
  const rule = sheet.cssRules[0];
  assert.ok(rule instanceof CSSFunctionRule);
  const declarations = rule.cssRules[0];
  assert.ok(declarations instanceof CSSFunctionDeclarations);
  const { style } = declarations;

  style.cssText = "--x: 3px; color: red; result: 4px; --ignored: 5px !important; --y: foo(a;b);";
  assert.deepEqual(Array.from(style), ["--x", "result", "--y"]);
  assert.equal(style.cssText, "--x: 3px; result: 4px; --y: foo(a;b);");
  assert.equal(style.setProperty("result", "5px"), undefined);
  assert.equal(style.result, "4px");
  assert.equal(style.removeProperty("result"), "4px");
  assert.equal(style.result, "");
  style.result = "6px";
  assert.equal(style.result, "");
  style.cssText = "result: 7px;";
  style.setProperty("result", "");
  assert.equal(style.result, "");
});

test("function bodies preserve conditional declaration runs and parent identity", () => {
  const sheet = parseStyleSheet(`
    @function --f() {
      --before: foo(a;b);
      @supports (width: 100px) {
        result: 100px;
        @media print { result: 101px; }
      }
      @container (width > 1px) { result: 150px; }
      result: 200px;
    }
  `);
  const rule = sheet.cssRules[0];
  assert.ok(rule instanceof CSSFunctionRule);
  assert.equal(rule.cssRules.length, 4);
  assert.ok(rule.cssRules[0] instanceof CSSFunctionDeclarations);
  assert.ok(rule.cssRules[1] instanceof CSSSupportsRule);
  assert.ok(rule.cssRules[2] instanceof CSSContainerRule);
  assert.ok(rule.cssRules[3] instanceof CSSFunctionDeclarations);

  const supports = rule.cssRules[1] as CSSSupportsRule;
  assert.equal(supports.conditionText, "(width: 100px)");
  assert.equal(supports.parentRule, rule);
  assert.equal(supports.parentStyleSheet, sheet);
  assert.ok(supports.cssRules[0] instanceof CSSFunctionDeclarations);
  assert.ok(supports.cssRules[1] instanceof CSSMediaRule);
  assert.equal(supports.cssRules[0]?.parentRule, supports);
  assert.equal(
    (supports.cssRules[0] as CSSFunctionDeclarations).style.result,
    "100px",
  );
});

test("function bodies drop unsupported rules without swallowing later descriptors", () => {
  const sheet = parseStyleSheet(`
    @function --f() {
      --before: 1;
      @layer ignored {}
      --after-layer: 2;
      @unknown ignored {}
      --after-unknown: 3;
      @media (width: 1px) { .invalid {} result: 4; }
    }
  `);
  const rule = sheet.cssRules[0];
  assert.ok(rule instanceof CSSFunctionRule);
  assert.equal(rule.cssRules.length, 2);
  const declarations = rule.cssRules[0];
  assert.ok(declarations instanceof CSSFunctionDeclarations);
  assert.equal(
    declarations.style.cssText,
    "--before: 1; --after-layer: 2; --after-unknown: 3;",
  );
  const media = rule.cssRules[1];
  assert.ok(media instanceof CSSMediaRule);
  assert.equal(media.cssRules.length, 0);
});

test("group insertion inside a function matches Chromium recovery", () => {
  const sheet = parseStyleSheet("@function --f() { result: 1; }");
  const rule = sheet.cssRules[0];
  assert.ok(rule instanceof CSSFunctionRule);

  assert.equal(rule.insertRule("@media (width:1px){result:3;}", 1), 1);
  const emptyMedia = rule.cssRules[1];
  assert.ok(emptyMedia instanceof CSSMediaRule);
  assert.equal(emptyMedia.cssRules.length, 0);
  assert.equal(emptyMedia.conditionText, "(width: 1px)");

  assert.equal(rule.insertRule("@media (width:1px){.x{}}", 2), 2);
  const mediaWithRule = rule.cssRules[2];
  assert.ok(mediaWithRule instanceof CSSMediaRule);
  assert.ok(mediaWithRule.cssRules[0] instanceof CSSStyleRule);
  assert.throws(
    () => rule.insertRule("result: 3px", 3),
    error => error instanceof DOMException && error.name === "SyntaxError",
  );
  for (const trailing of ["junk", ";", "color: red", ".x {}", "@unknown"]) {
    assert.throws(
      () => rule.insertRule(`@media (width:1px){} ${trailing}`, 3),
      error => error instanceof DOMException && error.name === "SyntaxError",
      trailing,
    );
  }
  assert.equal(
    rule.insertRule("/* before */ @media (width:1px){} /* recovered EOF", 3),
    3,
  );
});

test("function CSS text canonicalizes the WPT branches and serializes safely", () => {
  const cases = [
    ["@function --empty() { }", "@function --empty() { }"],
    [
      "@function --ret-type() returns type(<length> | auto) { }",
      "@function --ret-type() returns type(<length> | auto) { }",
    ],
    [
      "@function --param-type(--x type(<length>)) { }",
      "@function --param-type(--x <length>) { }",
    ],
    [
      "@function --param-universal(--x type(*)) { }",
      "@function --param-universal(--x) { }",
    ],
    [
      "@function --body() { result: 10px; result: 20px; }",
      "@function --body() { result: 20px; }",
    ],
    [
      String.raw`@function --identy(--a I\ dent) returns type(I\ dent) { }`,
      String.raw`@function --identy(--a I\ dent) returns I\ dent { }`,
    ],
    [
      "@function --curly-default(--x: {a,b}) { result: var(--x); }",
      "@function --curly-default(--x: {a,b}) { result: var(--x); }",
    ],
  ] as const;

  for (const [source, expected] of cases) {
    const sheet = parseStyleSheet(source);
    const rule = sheet.cssRules[0];
    assert.ok(rule instanceof CSSFunctionRule, source);
    assert.equal(rule.cssText, expected, source);
    const serialized = sheet.serialize();
    assert.equal(parseStyleSheet(serialized).serialize(), serialized, source);
  }
});

test("invalid known function preludes are dropped by constructed stylesheets", () => {
  const invalid = [
    "@function --foo () {}",
    "@function --foo(--x <dino>) {}",
    "@function --foo(--x <length>: 10deg) {}",
    "@function --foo(--x: !) {}",
    "@function --foo(--x) returns * {}",
    "@function --foo(--x) returns <transform-list># {}",
  ];
  for (const source of invalid) {
    const sheet = new CSSStyleSheet();
    sheet.replaceSync(source);
    assert.equal(sheet.cssRules.length, 0, source);
  }
});

test("custom function calls are deferred substitutions in ordinary declarations", () => {
  const sheet = parseStyleSheet(".x { width: 10px; }");
  const rule = sheet.cssRules[0];
  assert.ok(rule instanceof CSSStyleRule);

  rule.style.setProperty("width", "calc(--double(1px) + 1px)");
  assert.equal(rule.style.getPropertyValue("width"), "calc(--double(1px) + 1px)");
  for (const invalid of ["--()", "--double(,)", "--double(1px,)", "--double(1px;2px)"]) {
    rule.style.setProperty("width", invalid);
    assert.equal(
      rule.style.getPropertyValue("width"),
      "calc(--double(1px) + 1px)",
      invalid,
    );
  }

  rule.style.setProperty("padding", "--spacing(1px, 2px)", "important");
  assert.equal(rule.style.getPropertyValue("padding"), "--spacing(1px, 2px)");
  assert.equal(rule.style.getPropertyPriority("padding-top"), "important");
  const serialized = sheet.serialize();
  assert.equal(parseStyleSheet(serialized).serialize(), serialized);
});
