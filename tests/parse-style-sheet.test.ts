import assert from "node:assert/strict";
import { test } from "vitest";

import {
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
  assert.ok(importRule instanceof CSSRule);
  assert.equal(importRule.type, CSSRule.IMPORT_RULE);
  assert.equal(importRule.cssText, '@import "theme.css";');
  assert.equal(importRule.parentStyleSheet, sheet);

  assert.ok(sheet.cssRules[1] instanceof CSSStyleRule);
  assert.equal(
    sheet.serialize(),
    '@import "theme.css";\n.card {\n  color: red;\n}\n',
  );

  const constructed = new CSSStyleSheet();
  assert.throws(
    () => constructed.insertRule('@import "theme.css";'),
    error => error instanceof DOMException && error.name === "SyntaxError",
  );
});
