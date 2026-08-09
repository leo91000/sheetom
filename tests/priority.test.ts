import assert from "node:assert/strict";
import { test } from "vitest";

import { CSSStyleRule, CSSStyleSheet } from "../src/index.js";

test("priority validation is atomic and follows the Chromium empty-value divergence", () => {
  const sheet = new CSSStyleSheet({ diagnostics: true });
  sheet.insertRule(".x {}");

  const rule = sheet.cssRules[0];
  assert.ok(rule instanceof CSSStyleRule);

  rule.style.setProperty("width", "1px", "important");
  rule.style.setProperty("width", "2px", "bogus");
  rule.style.setProperty("width", "", "bogus");

  assert.equal(rule.style.getPropertyValue("width"), "1px");
  assert.equal(rule.style.getPropertyPriority("width"), "important");
  assert.deepEqual(
    sheet.takeDiagnostics().map(diagnostic => diagnostic.code),
    ["INVALID_PRIORITY", "INVALID_PRIORITY"],
  );
});
