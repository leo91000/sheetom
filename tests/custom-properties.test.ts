import assert from "node:assert/strict";
import { test } from "vitest";

import { CSSStyleRule, CSSStyleSheet, parseStyleSheet } from "../src/index.js";
import { createStyleRule } from "./support/create-style-rule.js";

test("custom properties preserve case and expose empty-but-present entries", () => {
  const sheet = new CSSStyleSheet();
  sheet.insertRule(".x {}");

  const rule = sheet.cssRules[0];
  assert.ok(rule instanceof CSSStyleRule);

  rule.style.setProperty("--X", "false");
  rule.style.setProperty("--x", " ");

  assert.equal(rule.style.length, 2);
  assert.equal(rule.style[0], "--X");
  assert.equal(rule.style[1], "--x");
  assert.equal(rule.style.getPropertyValue("--X"), "false");
  assert.equal(rule.style.getPropertyValue("--x"), "");
  assert.equal(rule.style.cssText, "--X: false; --x: ;");
});

test("declaration-block parsing preserves empty custom-property records", () => {
  const rule = createStyleRule(".x");
  rule.style.cssText = "--empty: ; --flag: false;";

  assert.equal(rule.style.length, 2);
  assert.equal(rule.style[0], "--empty");
  assert.equal(rule.style.getPropertyValue("--empty"), "");
  assert.equal(rule.style.cssText, "--empty: ; --flag: false;");
});

test("CSSOM custom-property names retain logical text and serialize as identifiers", () => {
  const rule = createStyleRule(".x");
  rule.style.setProperty("-- x", "red");
  rule.style.setProperty("--x!", "blue");
  rule.style.setProperty("--é", "rouge");
  rule.style.setProperty("--", "green");

  assert.equal(rule.style.length, 3);
  assert.equal(rule.style[0], "-- x");
  assert.equal(rule.style[1], "--x!");
  assert.equal(rule.style[2], "--é");
  assert.equal(rule.style.getPropertyValue("-- x"), "red");
  assert.equal(rule.style.getPropertyValue("--é"), "rouge");
  assert.equal(rule.style.cssText, "--\\ x: red; --x\\!: blue; --é: rouge;");
});

test("custom-property delimiters survive reparsable serialization", () => {
  const sheet = new CSSStyleSheet();
  sheet.insertRule(".x {}");

  const rule = sheet.cssRules[0];
  assert.ok(rule instanceof CSSStyleRule);
  rule.style.setProperty("--foo:bar", "red");
  rule.style.setProperty("--foo\\:bar", "blue");

  assert.equal(rule.style.getPropertyValue("--foo:bar"), "red");
  assert.equal(rule.style.getPropertyValue("--foo\\:bar"), "blue");
  assert.equal(
    rule.style.cssText,
    "--foo\\:bar: red; --foo\\\\\\:bar: blue;",
  );

  const serialized = sheet.serialize();
  assert.equal(
    serialized,
    ".x {\n  --foo\\:bar: red;\n  --foo\\\\\\:bar: blue;\n}\n",
  );
  const reparsed = parseStyleSheet(serialized);
  const reparsedRule = reparsed.cssRules[0];
  assert.ok(reparsedRule instanceof CSSStyleRule);
  assert.equal(reparsedRule.style.getPropertyValue("--foo:bar"), "red");
  assert.equal(reparsedRule.style.getPropertyValue("--foo\\:bar"), "blue");
  assert.equal(reparsed.serialize(), serialized);
});

test("reassigning a custom property replaces its priority", () => {
  const rule = createStyleRule(".x");
  rule.style.setProperty("--token", "red", "important");

  assert.equal(rule.style.getPropertyPriority("--token"), "important");

  rule.style.setProperty("--token", "red");

  assert.equal(rule.style.getPropertyPriority("--token"), "");
  assert.equal(rule.style.cssText, "--token: red;");
});
