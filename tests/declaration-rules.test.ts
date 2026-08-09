import assert from "node:assert/strict";
import { test } from "vitest";

import {
  CSSFontFaceRule,
  CSSMarginRule,
  CSSPageRule,
  CSSPositionTryRule,
  parseStyleSheet,
} from "../src/index.js";

test("declaration-bearing at-rules expose live style objects", () => {
  const sheet = parseStyleSheet(`
    @font-face { font-family: Test; src: url(test.woff2); }
    @page :first { margin: 1cm; @top-left { content: "x"; } }
    @position-try --foo { top: 1px; left: 2px; }
  `);

  const fontFace = sheet.cssRules[0];
  assert.ok(fontFace instanceof CSSFontFaceRule);
  assert.equal(fontFace.style.getPropertyValue("font-family"), "Test");
  assert.equal(fontFace.style.getPropertyValue("src"), 'url("test.woff2")');

  const page = sheet.cssRules[1];
  assert.ok(page instanceof CSSPageRule);
  assert.equal(page.selectorText, ":first");
  assert.equal(page.style.getPropertyValue("margin"), "1cm");
  const margin = page.cssRules[0];
  assert.ok(margin instanceof CSSMarginRule);
  assert.equal(margin.name, "top-left");
  assert.equal(margin.style.getPropertyValue("content"), '"x"');

  const positionTry = sheet.cssRules[2];
  assert.ok(positionTry instanceof CSSPositionTryRule);
  assert.equal(positionTry.name, "--foo");
  assert.equal(positionTry.style.cssText, "top: 1px; left: 2px;");

  fontFace.style.setProperty("font-display", "swap");
  assert.equal(fontFace.style.getPropertyValue("font-display"), "swap");
});
