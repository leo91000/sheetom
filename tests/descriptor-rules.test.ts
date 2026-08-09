import assert from "node:assert/strict";
import { test } from "vitest";

import {
  CSSCounterStyleRule,
  CSSFontFeatureValuesMap,
  CSSFontFeatureValuesRule,
  parseStyleSheet,
} from "../src/index.js";

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
});
