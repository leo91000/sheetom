import assert from "node:assert/strict";
import { test } from "vitest";

import {
  CSSCounterStyleRule,
  CSSFontFaceRule,
  CSSFontFeatureValuesMap,
  CSSFontFeatureValuesRule,
  parseStyleSheet,
} from "../src/index.js";

function fontFaceRule(): CSSFontFaceRule {
  const sheet = parseStyleSheet("@font-face {}");
  const rule = sheet.cssRules[0];
  assert.ok(rule instanceof CSSFontFaceRule);
  return rule;
}

test("font-face descriptors use their own Chromium grammar context", () => {
  const rule = fontFaceRule();
  const cases = [
    ["ascent-override", "90%", "90%"],
    ["descent-override", "calc(20%)", "calc(20%)"],
    ["font-display", "SWAP", "swap"],
    ["font-family", '"A B"', '"A B"'],
    ["font-feature-settings", '"kern" 1, "liga" off', '"kern", "liga" 0'],
    ["font-stretch", "75% 125%", "75% 125%"],
    ["font-style", "oblique 10deg 20deg", "oblique 10deg 20deg"],
    ["font-variation-settings", '"wght" 500', '"wght" 500'],
    ["font-weight", "100 900", "100 900"],
    ["line-gap-override", "normal", "normal"],
    ["size-adjust", "1e2%", "100%"],
    ["src", "local(Test), url(test.woff2) tech(color-COLRv1)", 'local("Test"), url("test.woff2") tech(color-colrv1)'],
    ["unicode-range", "U+??", "U+0-FF"],
  ] as const;

  for (const [name, input, expected] of cases) {
    rule.style.setProperty(name, input);
    assert.equal(rule.style.getPropertyValue(name), expected, name);
  }
  rule.style.setProperty("font-variant", "small-caps");
  assert.equal(rule.style.getPropertyValue("font-variant"), "");
  assert.match(rule.style.cssText, /font-variant: small-caps;/u);
  assert.equal(rule.style.item(rule.style.length - 1), "font-variant");
});

test("font-face descriptor mutations reject invalid values atomically", () => {
  const rule = fontFaceRule();
  rule.style.setProperty("font-display", "swap");
  rule.style.setProperty("font-display", "var(--display)");
  rule.style.setProperty("font-display", "initial");
  rule.style.setProperty("font-display", "swap; src: url(evil.woff2)");
  rule.style.setProperty("unknown-descriptor", "red");
  assert.equal(rule.style.cssText, "font-display: swap;");

  rule.style.setProperty("--source", "var(--fallback");
  assert.equal(rule.style.getPropertyValue("--source"), "var(--fallback");
  assert.equal(rule.style.length, 2);

  rule.style.setProperty("src", "url(font.woff2)", "important");
  assert.equal(rule.style.getPropertyPriority("src"), "important");
  assert.match(rule.style.cssText, /src: url\("font\.woff2"\) !important;/u);
});

test("counter-style descriptors are live properties", () => {
  const sheet = parseStyleSheet(
    '@counter-style thumbs { system: cyclic; symbols: "a" "b"; suffix: " "; }',
  );
  const rule = sheet.cssRules[0];
  assert.ok(rule instanceof CSSCounterStyleRule);
  assert.equal(rule.name, "thumbs");
  assert.equal(rule.system, "cyclic");
  assert.equal(rule.symbols, '"a" "b"');

  rule.name = "icons";
  rule.prefix = '"["';
  assert.equal(
    rule.cssText,
    '@counter-style icons { system: cyclic; symbols: "a" "b"; prefix: "["; suffix: " "; }',
  );

  rule.name = "bad name";
  assert.equal(rule.name, "icons");
  rule.name = "\\69 cons";
  assert.equal(rule.name, "icons");
});

test("counter-style descriptors use Chromium canonicalization and mutation rules", () => {
  const sheet = parseStyleSheet(
    '@counter-style digits { system: fixed; symbols: "a" "b"; additive-symbols: +010 "x", 001 "i"; negative: "(" ")"; prefix: "["; suffix: "]"; range: infinite -02, +004 infinite; pad: +03 "0"; speak-as: SPELL-OUT; fallback: DECIMAL; }',
  );
  const rule = sheet.cssRules[0];
  assert.ok(rule instanceof CSSCounterStyleRule);
  assert.equal(rule.system, "fixed 1");
  assert.equal(rule.additiveSymbols, '10 "x", 1 "i"');
  assert.equal(rule.range, "infinite -2, 4 infinite");
  assert.equal(rule.pad, '3 "0"');
  assert.equal(rule.speakAs, "SPELL-OUT");
  assert.equal(rule.fallback, "decimal");

  rule.system = "numeric";
  rule.symbols = '"z"';
  rule.range = "auto";
  rule.fallback = "disc";
  assert.equal(rule.system, "fixed 1");
  assert.equal(
    rule.cssText,
    '@counter-style digits { system: fixed 1; symbols: "z"; additive-symbols: 10 "x", 1 "i"; negative: "(" ")"; prefix: "["; suffix: "]"; pad: 3 "0"; range: auto; fallback: disc; speak-as: SPELL-OUT; }',
  );
});

test("counter-style descriptor mutations reject invalid and empty values atomically", () => {
  const sheet = parseStyleSheet('@counter-style marks { symbols: "a"; range: 1 10; }');
  const rule = sheet.cssRules[0];
  assert.ok(rule instanceof CSSCounterStyleRule);

  for (const value of ["", "var(--symbols)", '"x"; suffix: "evil"']) {
    rule.symbols = value;
  }
  rule.range = "10 1";
  assert.equal(rule.symbols, '"a"');
  assert.equal(rule.range, "1 10");
  assert.equal(rule.cssText, '@counter-style marks { symbols: "a"; range: 1 10; }');
});

test("font feature value maps expose mutable map behavior", () => {
  const sheet = parseStyleSheet(
    "@font-feature-values Test { @styleset { nice: 1 2; other: 3; } }",
  );
  const rule = sheet.cssRules[0];
  assert.ok(rule instanceof CSSFontFeatureValuesRule);
  assert.ok(rule.styleset instanceof CSSFontFeatureValuesMap);
  assert.deepEqual(rule.styleset.get("nice"), [1, 2]);

  rule.styleset.set("new", [4, 5]);
  rule.styleset.delete("nice");
  rule.fontFamily = "Other";

  assert.deepEqual([...rule.styleset], [
    ["other", [3]],
    ["new", [4, 5]],
  ]);
  assert.equal(
    rule.cssText,
    "@font-feature-values Other { @styleset { other: 3; new: 4 5; } }",
  );

  rule.annotation.set("bad name", [-1, 1.5, Number.NaN]);
  rule.annotation.set("", []);
  rule.fontFamily = '"A B", Test';
  assert.deepEqual(rule.annotation.get("bad name"), [4_294_967_295, 1, 0]);
  assert.equal(
    rule.cssText,
    '@font-feature-values "\\\"A B\\\"", Test { @annotation { bad\\ name: 4294967295 1 0; :; } @styleset { other: 3; new: 4 5; } }',
  );
});

test("font feature parsing merges valid maps and rejects invalid subrules", () => {
  const sheet = parseStyleSheet(
    '@font-feature-values "A B", Test { @styleset { a: 1; } @styleset { b: 2; a: 3; } @annotation { mark: 0; } @swash { good: 1; bad: -1; } }',
  );
  const rule = sheet.cssRules[0];
  assert.ok(rule instanceof CSSFontFeatureValuesRule);
  assert.equal(rule.fontFamily, '"A B", Test');
  assert.deepEqual([...rule.styleset], [["a", [3]], ["b", [2]]]);
  assert.deepEqual([...rule.annotation], [["mark", [0]]]);
  assert.deepEqual([...rule.swash], []);
});
