import assert from "node:assert/strict";
import { test } from "vitest";

import {
  CSSFontPaletteValuesRule,
  CSSLayerStatementRule,
  CSSNamespaceRule,
  CSSRule,
  CSSViewTransitionRule,
  parseStyleSheet,
} from "../src/index.js";

test("statement layers expose a fresh frozen nameList", () => {
  const sheet = parseStyleSheet("@layer reset, theme.components;");
  const rule = sheet.cssRules[0];

  assert.ok(rule instanceof CSSLayerStatementRule);
  assert.deepEqual(rule.nameList, ["reset", "theme.components"]);
  assert.ok(Object.isFrozen(rule.nameList));
  assert.notEqual(rule.nameList, rule.nameList);
  assert.equal(rule.cssText, "@layer reset, theme.components;");
});

test("namespace rules expose their logical prefix and URI", () => {
  const sheet = parseStyleSheet(
    '@namespace \\73 vg "urn:svg"; @namespace "urn:default";',
  );
  const prefixed = sheet.cssRules[0];
  const defaultNamespace = sheet.cssRules[1];

  assert.ok(prefixed instanceof CSSNamespaceRule);
  assert.equal(prefixed.type, CSSRule.NAMESPACE_RULE);
  assert.equal(prefixed.prefix, "svg");
  assert.equal(prefixed.namespaceURI, "urn:svg");
  assert.equal(prefixed.cssText, '@namespace svg url("urn:svg");');

  assert.ok(defaultNamespace instanceof CSSNamespaceRule);
  assert.equal(defaultNamespace.prefix, "");
  assert.equal(defaultNamespace.namespaceURI, "urn:default");
  assert.equal(defaultNamespace.cssText, '@namespace url("urn:default");');
});

test("font palette descriptors use Chromium winners and observable colors", () => {
  const sheet = parseStyleSheet(`
    @font-palette-values --brand {
      font-family: "A B", Test;
      font-family: serif;
      base-palette: invalid;
      base-palette: 2;
      override-colors: 0 red, 3 #00ff00;
      unknown: x;
    }
  `);
  const rule = sheet.cssRules[0];

  assert.ok(rule instanceof CSSFontPaletteValuesRule);
  assert.equal(rule.name, "--brand");
  assert.equal(rule.fontFamily, '"A B", Test');
  assert.equal(rule.basePalette, "2");
  assert.equal(rule.overrideColors, "0 red, 3 rgb(0, 255, 0)");
  assert.equal(
    rule.cssText,
    '@font-palette-values --brand { font-family: "A B", Test; base-palette: 2; override-colors: 0 red, 3 rgb(0, 255, 0); }',
  );
});

test("view transition types are a frozen same-object list", () => {
  const sheet = parseStyleSheet(`
    @view-transition {
      navigation: bad;
      navigation: auto;
      types: old;
      types: foo\\ bar \\62 az;
      unknown: x;
    }
  `);
  const rule = sheet.cssRules[0];

  assert.ok(rule instanceof CSSViewTransitionRule);
  assert.equal(rule.navigation, "auto");
  assert.deepEqual(rule.types, ["foo bar", "baz"]);
  assert.ok(Object.isFrozen(rule.types));
  assert.equal(rule.types, rule.types);
  assert.equal(
    rule.cssText,
    "@view-transition { navigation: auto; types: foo\\ bar baz; }",
  );
});
