import assert from "node:assert/strict";
import { test } from "vitest";
import * as api from "../src/index.js";
import { assertAuthoringRoundTrip } from "../scripts/css-authoring-roundtrip.ts";
const { CSSStyleSheet, CSSStyleRule } = api;

test("strict serialization keeps inactive animation settings for subsequent mutation", () => {
  const sheet = new CSSStyleSheet();
  sheet.replaceSync(".a { animation: 1s linear(0, 1) none; }");
  const rule = sheet.cssRules[0]; assert.ok(rule instanceof CSSStyleRule);
  const copy = new CSSStyleSheet(); copy.replaceSync(sheet.serializeStrict());
  const copied = copy.cssRules[0]; assert.ok(copied instanceof CSSStyleRule);
  for (const property of Array.from(rule.style)) {
    assert.ok(property);
    assert.equal(copied.style.getPropertyValue(property), rule.style.getPropertyValue(property), property);
  }
  copied.style.animationName = "fade";
  assert.equal(copied.style.animationDuration, "1s");
  assert.equal(copied.style.animationTimingFunction, "linear(0 0%, 1 100%)");
});

test("round-trip evidence rejects lost rules, declarations, values, and priorities", () => {
  const original = new CSSStyleSheet();
  original.replaceSync(".a { color: blue !important; width: 1px; } @media all { .b { height: 2px; } }");
  const copy = () => {
    const sheet = new CSSStyleSheet(); sheet.replaceSync(original.serializeStrict()); return sheet;
  };
  assertAuthoringRoundTrip(api, original, copy());
  const style = (sheet: api.CSSStyleSheet) => {
    const rule = sheet.cssRules[0]; assert.ok(rule instanceof CSSStyleRule); return rule.style;
  };
  for (const mutate of [
    (sheet: api.CSSStyleSheet) => sheet.deleteRule(1),
    (sheet: api.CSSStyleSheet) => style(sheet).removeProperty("width"),
    (sheet: api.CSSStyleSheet) => style(sheet).setProperty("width", "2px"),
    (sheet: api.CSSStyleSheet) => style(sheet).setProperty("color", "blue"),
  ]) {
    const changed = copy(); mutate(changed);
    assert.throws(() => assertAuthoringRoundTrip(api, original, changed), assert.AssertionError);
  }
});
