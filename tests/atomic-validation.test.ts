import assert from "node:assert/strict";
import { test } from "vitest";

import { CSSStyleRule, CSSStyleSheet } from "../src/index.js";

test("an invalid value leaves the declaration unchanged and reports a diagnostic", () => {
  const sheet = new CSSStyleSheet({ diagnostics: true });
  sheet.insertRule(".x {}");

  const rule = sheet.cssRules[0];
  assert.ok(rule instanceof CSSStyleRule);

  rule.style.setProperty("width", "1px");
  rule.style.setProperty("width", "totallybogus");
  rule.style.setProperty("totally-unknown", "var(--x)");

  assert.equal(rule.style.getPropertyValue("width"), "1px");
  assert.equal(rule.style.getPropertyValue("totally-unknown"), "");

  const diagnostics = sheet.takeDiagnostics();
  assert.equal(diagnostics.length, 2);
  assert.deepEqual(
    {
      code: diagnostics[0]?.code,
      severity: diagnostics[0]?.severity,
      operation: diagnostics[0]?.operation,
      property: diagnostics[0]?.property,
      input: diagnostics[0]?.input,
      location: diagnostics[0]?.location,
      messageType: typeof diagnostics[0]?.message,
    },
    {
      code: "INVALID_PROPERTY_VALUE",
      severity: "warning",
      operation: "setProperty",
      property: "width",
      input: "totallybogus",
      location: null,
      messageType: "string",
    },
  );
  assert.deepEqual(sheet.takeDiagnostics(), []);
});

test("setProperty rejects embedded priority tokens without rejecting data", () => {
  const rule = new CSSStyleRule(".x");
  rule.style.setProperty("color", "blue");
  rule.style.setProperty("color", "red !important");
  rule.style.setProperty("--fallback", "var(--x, !important)");
  rule.style.setProperty("--url", "url(foo!bar)");
  rule.style.setProperty("--escaped", "foo\\!bar");

  assert.equal(rule.style.getPropertyValue("color"), "blue");
  assert.equal(rule.style.getPropertyValue("--fallback"), "");
  assert.equal(rule.style.getPropertyValue("--url"), "url(foo!bar)");
  assert.equal(rule.style.getPropertyValue("--escaped"), "foo\\!bar");
});
