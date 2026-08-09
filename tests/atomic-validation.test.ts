import assert from "node:assert/strict";
import { test } from "vitest";

import {
  CSSStyleRule,
  CSSStyleSheet,
  type SheetOMDiagnosticCode,
} from "../src/index.js";
import { createStyleRule } from "./support/create-style-rule.js";

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

test("diagnostic codes expose a stable public string union", () => {
  const codes: SheetOMDiagnosticCode[] = [
    "INVALID_PRIORITY",
    "INVALID_PROPERTY_VALUE",
  ];

  assert.deepEqual(codes, ["INVALID_PRIORITY", "INVALID_PROPERTY_VALUE"]);
});

test("setProperty rejects embedded priority tokens without rejecting data", () => {
  const rule = createStyleRule(".x");
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

test("setProperty rejects values that escape into another declaration", () => {
  const sheet = new CSSStyleSheet({ diagnostics: true });
  sheet.insertRule(".x { width: 10px; }");

  const rule = sheet.cssRules[0];
  assert.ok(rule instanceof CSSStyleRule);

  for (const invalidValue of [
    "20px;",
    "20px; color: red",
    "var(--x, 20px; color: red)",
    "20px } .evil { color: red",
    "20px!important",
    "20px ! important",
  ]) {
    rule.style.setProperty("width", invalidValue);
    assert.equal(rule.style.getPropertyValue("width"), "10px");
    assert.equal(rule.style.getPropertyValue("color"), "");
    assert.equal(rule.style.cssText, "width: 10px;");
    assert.equal(sheet.serialize(), ".x {\n  width: 10px;\n}\n");
  }

  assert.deepEqual(
    sheet.takeDiagnostics().map(diagnostic => diagnostic.code),
    Array.from({ length: 6 }, () => "INVALID_PROPERTY_VALUE"),
  );
});

test("substitution fallbacks reject boundary delimiters but allow nested data", () => {
  const rule = createStyleRule(".x");
  rule.style.setProperty("width", "10px");

  rule.style.setProperty("width", "var(--x, 20px; color: red)");
  assert.equal(rule.style.getPropertyValue("width"), "10px");

  rule.style.setProperty("width", "var(--x, fn(!important))");
  assert.equal(
    rule.style.getPropertyValue("width"),
    "var(--x, fn(!important))",
  );
});
