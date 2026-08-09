import assert from "node:assert/strict";
import { test } from "vitest";

import {
  CSSImportRule,
  CSSRule,
  CSSStyleRule,
  CSSStyleSheet,
  parseStyleSheet,
} from "../src/index.js";

test("parseStyleSheet preserves imports in a regular authoring sheet", () => {
  const sheet = parseStyleSheet(
    '@import "theme.css"; .card { color: red; }',
    { href: "https://example.test/css/app.css" },
  );

  assert.equal(sheet.href, "https://example.test/css/app.css");
  assert.equal(sheet.baseURL, "https://example.test/css/app.css");
  assert.equal(sheet.cssRules.length, 2);

  const importRule = sheet.cssRules[0];
  assert.ok(importRule instanceof CSSImportRule);
  assert.ok(importRule instanceof CSSRule);
  assert.equal(importRule.type, CSSRule.IMPORT_RULE);
  assert.equal(importRule.cssText, '@import url("theme.css");');
  assert.equal(importRule.href, "https://example.test/css/theme.css");
  assert.equal(importRule.styleSheet, null);
  assert.equal(importRule.media.mediaText, "");
  assert.equal(importRule.parentStyleSheet, sheet);

  assert.ok(sheet.cssRules[1] instanceof CSSStyleRule);
  assert.equal(
    sheet.serialize(),
    '@import url("theme.css");\n.card {\n  color: red;\n}\n',
  );

  const constructed = new CSSStyleSheet();
  assert.throws(
    () => constructed.insertRule('@import "theme.css";'),
    error => error instanceof DOMException && error.name === "SyntaxError",
  );
});

test("import rules expose mutable media and read-only import metadata", () => {
  const sheet = parseStyleSheet(
    '@import url("theme.css") layer(theme) supports(display: grid) screen and (width > 1px);',
  );
  const rule = sheet.cssRules[0];
  assert.ok(rule instanceof CSSImportRule);
  assert.equal(rule.layerName, "theme");
  assert.equal(rule.supportsText, "display:grid");
  assert.equal(rule.media.mediaText, "screen and (width > 1px)");

  rule.media.mediaText = "print";
  assert.equal(
    rule.cssText,
    '@import url("theme.css") layer(theme) supports(display:grid) print;',
  );
});
