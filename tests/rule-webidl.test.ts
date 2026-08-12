import assert from "node:assert/strict";
import { test } from "vitest";

import {
  CSSMediaRule,
  CSSRuleList,
  CSSStyleDeclaration,
  CSSStyleRule,
  CSSStyleSheet,
  MediaList,
  parseStyleSheet,
} from "../src/index.js";

test("browser-created rule interfaces reject direct construction", () => {
  assert.throws(() => Reflect.construct(CSSStyleRule, [".x"]), TypeError);
  assert.throws(() => Reflect.construct(CSSMediaRule, ["screen"]), TypeError);
  assert.throws(() => Reflect.construct(MediaList, []), TypeError);
  assert.throws(() => Reflect.construct(CSSStyleDeclaration, [null]), TypeError);
  assert.throws(() => Reflect.construct(CSSRuleList, [[]]), TypeError);
});

test("assigning rule cssText is a successful no-op", () => {
  const sheet = parseStyleSheet("@media screen { .x { color: red; } }");
  const media = sheet.cssRules[0];
  const style = media instanceof CSSMediaRule ? media.cssRules[0] : null;
  assert.ok(media instanceof CSSMediaRule);
  assert.ok(style instanceof CSSStyleRule);

  const mediaText = media.cssText;
  const styleText = style.cssText;
  assert.equal(Reflect.set(media, "cssText", "garbage"), true);
  assert.equal(Reflect.set(style, "cssText", "garbage"), true);
  assert.equal(media.cssText, mediaText);
  assert.equal(style.cssText, styleText);
  assert.equal(Reflect.set(style, "parentRule", null), false);
  assert.equal(Reflect.set(style, "parentStyleSheet", null), false);
  assert.equal(Reflect.set(style, "type", 99), false);
  assert.equal(Reflect.set(style, "style", null), false);
  assert.equal(Reflect.set(media, "cssRules", null), false);
  assert.equal(Reflect.set(media.cssRules, "0", null), false);
  assert.equal(style.parentRule, media);
  assert.equal(style.parentStyleSheet, sheet);
  assert.equal(style.type, CSSStyleRule.STYLE_RULE);
  assert.equal(style.STYLE_RULE, 1);
  assert.equal(Reflect.set(style, "STYLE_RULE", 99), false);
});

test("required WebIDL arguments throw before conversion or mutation", () => {
  const sheet = new CSSStyleSheet();
  sheet.insertRule(".x {}");
  const rule = sheet.cssRules[0];
  assert.ok(rule instanceof CSSStyleRule);

  assert.throws(() => Reflect.apply(rule.style.setProperty, rule.style, []), TypeError);
  assert.throws(() => Reflect.apply(rule.style.setProperty, rule.style, ["color"]), TypeError);
  assert.throws(() => Reflect.apply(rule.style.getPropertyValue, rule.style, []), TypeError);
  assert.throws(() => Reflect.apply(sheet.insertRule, sheet, []), TypeError);
  assert.throws(() => Reflect.apply(sheet.deleteRule, sheet, []), TypeError);
  assert.equal(sheet.cssRules.length, 1);
});

test("stylesheet constructor dictionaries follow WebIDL null conversion", () => {
  const empty = new CSSStyleSheet(null);
  const media = new CSSStyleSheet({ media: null as never });
  assert.equal(empty.media.mediaText, "");
  assert.equal(media.media.mediaText, "null");
});
