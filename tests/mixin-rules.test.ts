import assert from "node:assert/strict";
import { test } from "vitest";
import {
  CSSApplyBlockRule, CSSApplyStatementRule, CSSContentsBlockRule,
  CSSContentsStatementRule, CSSGroupingRule, CSSMediaRule, CSSMixinRule,
  CSSNestedDeclarations, CSSStyleRule, CSSStyleSheet,
} from "../src/index.js";

test("mixin parameters and draft interfaces are available by default", () => {
  const sheet = new CSSStyleSheet();
  sheet.replaceSync(`@mixin --theme(--color <color>: red, --gap: 10px) {
    color: var(--color); @contents { padding: var(--gap); }
  }`);
  const rule = sheet.cssRules[0];
  assert.ok(rule instanceof CSSMixinRule);
  assert.ok(rule instanceof CSSGroupingRule);
  assert.equal(rule.name, "--theme");
  assert.equal(rule.contents, true);
  assert.equal(rule.type, 0);
  const parameters = rule.getParameters();
  assert.deepEqual(parameters, [
    { name: "--color", type: "<color>", defaultValue: "red" },
    { name: "--gap", type: "*", defaultValue: "10px" },
  ]);
  parameters[0]!.name = "--changed";
  assert.equal(rule.getParameters()[0]!.name, "--color");
  assert.ok(rule.cssRules[0] instanceof CSSNestedDeclarations);
  assert.ok(rule.cssRules[1] instanceof CSSContentsBlockRule);
  for (const constructor of [CSSMixinRule, CSSApplyBlockRule, CSSApplyStatementRule,
    CSSContentsBlockRule, CSSContentsStatementRule]) {
    assert.throws(() => Reflect.construct(constructor, []), TypeError);
  }
});

test("mixin rules preserve statement versus empty-block applications and arguments", () => {
  const sheet = new CSSStyleSheet();
  sheet.replaceSync(`@mixin --empty() {} .a {
    @apply --empty(); @apply --empty {};
    @apply --missing({ red, blue }, calc(1px + 2px), "a;b");
  }`);
  const mixin = sheet.cssRules[0];
  const style = sheet.cssRules[1];
  assert.ok(mixin instanceof CSSMixinRule);
  assert.equal(mixin.cssText, "@mixin --empty{  }");
  assert.ok(style instanceof CSSStyleRule);
  const statement = style.cssRules[0];
  const block = style.cssRules[1];
  const call = style.cssRules[2];
  assert.ok(statement instanceof CSSApplyStatementRule);
  assert.ok(block instanceof CSSApplyBlockRule);
  assert.ok(call instanceof CSSApplyStatementRule);
  assert.equal(statement.cssText, "@apply --empty;");
  assert.equal(block.cssText, "@apply --empty {  }");
  assert.deepEqual(call.getArguments(), ["red, blue", "calc(1px + 2px)", '"a;b"']);
  const args = call.getArguments(); args.pop();
  assert.equal(call.getArguments().length, 3);
  assert.equal(call.cssText, '@apply --missing({ red, blue }, calc(1px + 2px), "a;b");');
  const reparsed = new CSSStyleSheet(); reparsed.replaceSync(sheet.serializeStrict());
  assert.equal(reparsed.cssRules[1]!.cssText, style.cssText);
});

test("mixin declaration runs, conditional contents, and live mutation keep parentage", () => {
  const sheet = new CSSStyleSheet();
  sheet.replaceSync(`@mixin --card {
    color: red; @media (width > 1px) { @contents; color: blue; }
    padding: 2px; & > span { color: green; }
  }`);
  const mixin = sheet.cssRules[0]; assert.ok(mixin instanceof CSSMixinRule);
  assert.equal(mixin.cssRules.length, 4);
  const first = mixin.cssRules[0]; assert.ok(first instanceof CSSNestedDeclarations);
  const media = mixin.cssRules[1]; assert.ok(media instanceof CSSMediaRule);
  assert.ok(media.cssRules[0] instanceof CSSContentsStatementRule);
  assert.equal(media.cssRules[0].parentRule, media);
  assert.equal(media.cssRules[0].parentStyleSheet, sheet);
  first.style.setProperty("color", "blue", "important");
  assert.equal(first.style.getPropertyPriority("color"), "important");
  const list = mixin.cssRules;
  mixin.insertRule("@apply --other { margin: 1px; }", 2);
  assert.equal(mixin.cssRules, list);
  const inserted = mixin.cssRules[2]; assert.ok(inserted instanceof CSSApplyBlockRule);
  assert.equal(inserted.cssRules[0]!.parentStyleSheet, sheet);
  mixin.deleteRule(2);
  assert.equal(inserted.parentRule, null);
  assert.equal(inserted.cssRules[0]!.parentRule, inserted);
  assert.equal(inserted.cssRules[0]!.parentStyleSheet, null);
  media.insertRule("@supports (display: grid) { @contents {} }", 0);
  assert.ok(media.cssRules[0] instanceof CSSGroupingRule);
  assert.ok(media.cssRules[0].cssRules[0] instanceof CSSContentsBlockRule);
  const reparsed = new CSSStyleSheet(); reparsed.replaceSync(sheet.serializeStrict());
  const reparsedMixin = reparsed.cssRules[0]; assert.ok(reparsedMixin instanceof CSSMixinRule);
  const reparsedDeclaration = reparsedMixin.cssRules[0];
  assert.ok(reparsedDeclaration instanceof CSSNestedDeclarations);
  assert.equal(reparsedDeclaration.style.getPropertyPriority("color"), "important");
});

test("mixin context and invalid input are rejected atomically", () => {
  const sheet = new CSSStyleSheet();
  sheet.replaceSync("@contents; @apply --x; .a { @mixin --invalid {} @contents; color: red; }");
  assert.equal(sheet.cssRules.length, 1);
  const style = sheet.cssRules[0]; assert.ok(style instanceof CSSStyleRule);
  assert.equal(style.cssRules.length, 0);
  for (const source of ["@mixin foo {}", "@mixin --x(--a, --a) {}",
    "@mixin --x(--a <length>: red) {}", "@mixin --x() returns <color> {}",
    "@mixin --x {} .b {}", "@contents;", "@apply --x;"]) {
    assert.throws(() => sheet.insertRule(source), { name: "SyntaxError" });
    assert.equal(sheet.cssRules.length, 1);
  }
  for (const source of ["@apply foo;", "@apply --x(1 !important);", "@apply --x; junk",
    "@contents;", "@mixin --nested {}"])
    assert.throws(() => style.insertRule(source), { name: "SyntaxError" });
  assert.equal(style.style.color, "red");
});

test("mixin bodies retain substitution punctuation and custom-property blocks", () => {
  const sheet = new CSSStyleSheet();
  sheet.replaceSync(`@mixin --x {
    --tokens: { a: b; c: d }; color: if(style(--flag: yes): red; else: blue);
    @apply --y(if(style(--flag: yes): red; else: blue));
    padding: 2px;
  }`);
  const mixin = sheet.cssRules[0]; assert.ok(mixin instanceof CSSMixinRule);
  const declarations = mixin.cssRules[0]; assert.ok(declarations instanceof CSSNestedDeclarations);
  assert.equal(declarations.style.getPropertyValue("--tokens"), "{ a: b; c: d }");
  assert.match(declarations.style.color, /^if\(/);
  const call = mixin.cssRules[1]; assert.ok(call instanceof CSSApplyStatementRule);
  assert.deepEqual(call.getArguments(), ["if(style(--flag: yes): red; else: blue)"]);
  const reparsed = new CSSStyleSheet(); reparsed.replaceSync(sheet.serialize());
  assert.equal(reparsed.cssRules[0]!.cssText, mixin.cssText);
});

test("mixin insertions account for rules and nesting without partial mutation", () => {
  const sheet = new CSSStyleSheet({ resourceBudget: { maxRuleCount: 3, maxSyntaxDepth: 8 } });
  sheet.replaceSync("@mixin --x { color: red; }");
  const mixin = sheet.cssRules[0]; assert.ok(mixin instanceof CSSMixinRule);
  const before = sheet.serialize();
  assert.throws(() => mixin.insertRule("@apply --x { color: blue; }"), RangeError);
  assert.equal(sheet.serialize(), before);
  mixin.insertRule("@contents;");
  assert.equal(mixin.cssRules.length, 2);
});

test("free-form arguments reject misplaced braces and invalid nested tokens", () => {
  const sheet = new CSSStyleSheet(); sheet.replaceSync(".a {}");
  const style = sheet.cssRules[0]; assert.ok(style instanceof CSSStyleRule);
  for (const value of ["red { blue }", "{red} blue", "{red; blue}", "{red !important}", 'fn("bad\nstring")']) {
    assert.throws(() => style.insertRule(`@apply --x(${value});`), { name: "SyntaxError" }, value);
  }
  style.insertRule("@apply --x({ });");
  style.insertRule("@apply --x({ }) {}");
  const copy = new CSSStyleSheet(); copy.replaceSync(sheet.serializeStrict());
  const copied = copy.cssRules[0]; assert.ok(copied instanceof CSSStyleRule);
  for (const rule of Array.from(copied.cssRules)) {
    assert.ok(rule instanceof CSSApplyStatementRule || rule instanceof CSSApplyBlockRule);
    assert.deepEqual(rule.getArguments(), [""]);
  }
});

test("private property syntax is retained without inventing a draft CSSOM interface", () => {
  const sheet = new CSSStyleSheet();
  sheet.replaceSync("@mixin --x { @private { --local: red; } color: var(--local); }");
  const mixin = sheet.cssRules[0]; assert.ok(mixin instanceof CSSMixinRule);
  assert.equal(mixin.cssRules[0]!.cssText, "@private { --local: red; }");
  assert.equal(mixin.cssRules[0]!.parentRule, mixin);
  assert.throws(() => sheet.insertRule("@private { --x: red; }"), { name: "SyntaxError" });
  const copy = new CSSStyleSheet(); copy.replaceSync(sheet.serializeStrict());
  assert.equal(copy.cssRules[0]!.cssText, mixin.cssText);
});
