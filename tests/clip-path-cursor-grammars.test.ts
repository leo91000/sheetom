import assert from "node:assert/strict";
import { test } from "vitest";

import { CSSStyleRule, parseStyleSheet } from "../src/index.js";
import { createStyleRule } from "./support/create-style-rule.js";

test("modern clip paths retain Chromium canonical values and atomic replacement", () => {
  const rule = createStyleRule(".clip");
  rule.style.setProperty(
    "-webkit-clip-path",
    'content-box path(nonzero, "M0 0")',
    "important",
  );

  assert.equal(rule.style.length, 1);
  assert.equal(rule.style.item(0), "clip-path");
  assert.equal(rule.style.getPropertyValue("clip-path"), 'path("M 0 0") content-box');
  assert.equal(rule.style.getPropertyPriority("clip-path"), "important");
  assert.equal(
    rule.style.cssText,
    'clip-path: path("M 0 0") content-box !important;',
  );

  const beforeInvalid = rule.style.cssText;
  rule.style.setProperty("clip-path", 'path("M0")');
  assert.equal(rule.style.cssText, beforeInvalid);

  const serialized = rule.parentStyleSheet?.serialize();
  assert.ok(serialized);
  assert.equal(parseStyleSheet(serialized).serialize(), serialized);
});

test("cursor URL sets are canonical, safe, and reject invalid neighbors atomically", () => {
  const rule = createStyleRule(".cursor");
  rule.style.setProperty(
    "cursor",
    'image-set(url(a.png) 1x, url(b.png) 2x type("image/png")) 1 1, auto',
    "important",
  );

  const expected =
    'image-set(url("a.png") 1x, url("b.png") 2x type("image/png")) 1 1, auto';
  assert.equal(rule.style.getPropertyValue("cursor"), expected);
  assert.equal(rule.style.cssText, `cursor: ${expected} !important;`);

  for (const invalid of [
    "image-set(linear-gradient(red, blue) 1x), auto",
    "image-set(url(a.png) -1x), auto",
    "image-set(url(a.png) 1x) 1, auto",
    "image-set(url(a.png) 1x) 1 1 auto",
  ]) {
    const beforeInvalid: string = rule.style.cssText;
    rule.style.setProperty("cursor", invalid, "important");
    assert.equal(rule.style.cssText, beforeInvalid, invalid);
  }

  rule.style.setProperty("cursor", '-webkit-image-set("a.png") 2 3, pointer');
  assert.equal(
    rule.style.getPropertyValue("cursor"),
    'image-set(url("a.png") 1x) 2 3, pointer',
  );

  const serialized = rule.parentStyleSheet?.serialize();
  assert.ok(serialized);
  const reparsed = parseStyleSheet(serialized);
  assert.equal(reparsed.serialize(), serialized);
  const reparsedRule = reparsed.cssRules[0];
  assert.ok(reparsedRule instanceof CSSStyleRule);
  assert.equal(
    reparsedRule.style.getPropertyValue("cursor"),
    rule.style.getPropertyValue("cursor"),
  );
});
