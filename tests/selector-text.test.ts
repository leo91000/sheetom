import assert from "node:assert/strict";
import { test } from "vitest";

import { CSSGroupingRule, CSSStyleRule, CSSStyleSheet, parseStyleSheet } from "../src/index.js";

test("selectorText normalizes valid lists and ignores invalid replacements", () => {
  const sheet = parseStyleSheet(".initial { color: red; }");
  const rule = sheet.cssRules[0];
  assert.ok(rule instanceof CSSStyleRule);

  rule.selectorText = ".a,.b";
  assert.equal(rule.selectorText, ".a, .b");
  assert.equal(rule.cssText, ".a, .b { color: red; }");

  rule.selectorText = "::not-a-pseudo(";
  assert.equal(rule.selectorText, ".a, .b");
});

test("selector serialization uses CSSOM spelling and stylesheet namespace context", () => {
  for (const [source, expected] of [
    ["::before", "::before"], [":after", "::after"], [":first-line", "::first-line"],
    [":nth-child(odd of .item)", ":nth-child(2n+1 of .item)"],
    [":has(> .item)", ":has(> .item)"],
    [":nth-child(2n of .odd)", ":nth-child(2n of .odd)"],
    [":nth-child(odd of [data-value=even])", ':nth-child(2n+1 of [data-value="even"])'], ["*|a", "a"],
    ['[data-value=":before"]', '[data-value=":before"]'],
  ]) {
    const sheet = parseStyleSheet(`${source} { color: red; }`);
    const rule = sheet.cssRules[0]; assert.ok(rule instanceof CSSStyleRule);
    assert.equal(rule.selectorText, expected);
    rule.selectorText = source!;
    assert.equal(rule.selectorText, expected);
  }
  const sheet = parseStyleSheet('@namespace url("urn:default"); @namespace svg url("urn:svg"); *|a {}');
  const rule = sheet.cssRules[2]; assert.ok(rule instanceof CSSStyleRule);
  assert.equal(rule.selectorText, "*|a");
  rule.selectorText = "svg|path";
  assert.equal(rule.selectorText, "svg|path");
  rule.selectorText = "missing|path";
  assert.equal(rule.selectorText, "svg|path");
});

test("namespace rule parsing and mutation reject unknown prefixes atomically", () => {
  const sheet = parseStyleSheet('@namespace svg url("urn:svg");');
  sheet.insertRule("svg|path {}", 1);
  const before = sheet.serializeStrict();
  assert.throws(() => sheet.insertRule("missing|path {}", 2), { name: "SyntaxError" });
  assert.throws(() => sheet.insertRule('@namespace other url("urn:other");', 1), { name: "InvalidStateError" });
  assert.throws(() => sheet.deleteRule(0), { name: "InvalidStateError" });
  assert.equal(sheet.serializeStrict(), before);
  sheet.deleteRule(1);
  sheet.deleteRule(0);
  assert.throws(() => sheet.insertRule("svg|path {}"), { name: "SyntaxError" });

  const constructed = new CSSStyleSheet();
  constructed.replaceSync('missing|path {} .a{} @namespace svg url("urn:svg"); svg|path{}');
  assert.equal(constructed.cssRules.length, 1);
  assert.throws(() => constructed.insertRule("svg|path {}"), { name: "SyntaxError" });
  constructed.replaceSync('@namespace svg url("urn:svg"); svg|path{}');
  const rule = constructed.cssRules[1]; assert.ok(rule instanceof CSSStyleRule);
  assert.equal(rule.selectorText, "svg|path");
  rule.selectorText = "svg|circle";
  assert.equal(rule.selectorText, "svg|circle");
  constructed.insertRule('@media all { missing|path {} svg|path {} }', 2);
  const media = constructed.cssRules[2];
  assert.ok(media instanceof CSSGroupingRule);
  assert.equal(media.cssRules.length, 1);
  for (const source of [constructed.serialize(), constructed.serializeStrict()]) {
    const reparsed = new CSSStyleSheet();
    reparsed.replaceSync(source);
    assert.equal(reparsed.cssRules.length, 3);
    const circle = reparsed.cssRules[1];
    assert.ok(circle instanceof CSSStyleRule);
    assert.equal(circle.selectorText, "svg|circle");
    assert.equal(reparsed.serializeStrict(), source);
  }
  assert.equal(constructed.cssRules.length, 3);
});
