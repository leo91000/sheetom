import assert from "node:assert/strict";
import { test } from "vitest";

import {
  CSSFunctionRule,
  CSSRule,
  CSSStyleRule,
  CSSStyleSheet,
  parseStyleSheet,
} from "../src/index.js";

test("constructed sheets drop invalid selectors like native CSSOM", () => {
  const sheet = new CSSStyleSheet();
  sheet.replaceSync(".a,,.b { color: red; } .ok { color: blue; }");
  assert.equal(sheet.cssRules.length, 1);
  assert.ok(sheet.cssRules[0] instanceof CSSStyleRule);
});

test("regular parsing preserves dropped rules as immutable opaque nodes", () => {
  const invalid = ".a,,.b { color: red; }";
  const sheet = parseStyleSheet(`${invalid} .ok { color: blue; }`);
  const opaque = sheet.cssRules[0];
  assert.ok(opaque instanceof CSSRule);
  assert.ok(!(opaque instanceof CSSStyleRule));
  assert.equal(opaque.cssText, invalid);
  assert.equal(Reflect.set(opaque, "cssText", ".changed {}"), true);
  assert.equal(opaque.cssText, invalid);
  assert.match(sheet.serialize(), /^\.a,,\.b \{ color: red; \}/);
});

test("regular parsing preserves invalid known functions without exposing function CSSOM", () => {
  const invalid = "@function --value(--x <length>: 10deg) {}";
  const sheet = parseStyleSheet(`${invalid} .ok { color: blue; }`);
  const opaque = sheet.cssRules[0];
  assert.ok(opaque instanceof CSSRule);
  assert.ok(!(opaque instanceof CSSFunctionRule));
  assert.equal(opaque.cssText, invalid);
  assert.equal("getParameters" in opaque, false);
  assert.equal(sheet.cssRules[1] instanceof CSSStyleRule, true);
});
