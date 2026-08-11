import assert from "node:assert/strict";
import { chromium } from "playwright";

import { CSSFontFaceRule, parseStyleSheet } from "../dist/index.js";

const operations = [
  ["set", "ascent-override", "90%", ""],
  ["set", "descent-override", "calc(20%)", ""],
  ["set", "font-display", "SWAP", ""],
  ["set", "font-family", '"A B"', ""],
  ["set", "font-feature-settings", '"kern" 1, "liga" off', ""],
  ["set", "font-stretch", "75% 125%", ""],
  ["set", "font-stretch", "condensed expanded", ""],
  ["set", "font-stretch", "75% -1%", ""],
  ["set", "font-style", "oblique 10deg 20deg", ""],
  ["set", "font-variant", "small-caps", ""],
  ["set", "font-variation-settings", '"wght" 500', ""],
  ["set", "font-weight", "100 900", ""],
  ["set", "line-gap-override", "normal", ""],
  ["set", "size-adjust", "1e2%", ""],
  ["set", "src", "local(Test), url(test.woff2) tech(color-COLRv1)", ""],
  ["set", "unicode-range", "U+??", ""],
  ["set", "--source", "var(--fallback", ""],
  ["set", "font-display", "var(--display)", ""],
  ["set", "font-family", "A, B", ""],
  ["set", "src", "none", ""],
  ["set", "size-adjust", "-1%", ""],
  ["set", "unknown-descriptor", "red", ""],
  ["set", "src", "url(font.woff2)", "important"],
  ["remove", "descent-override"],
  ["cssText", "font-family: Test; src: url(final.woff2); font-display: invalid; unknown: red"],
];

function snapshot(style) {
  const items = Array.from({ length: style.length }, (_, index) => style.item(index));
  return {
    cssText: style.cssText,
    length: style.length,
    items,
    declarations: items.map(name => ({
      name,
      value: style.getPropertyValue(name),
      priority: style.getPropertyPriority(name),
    })),
  };
}

function apply(style, operation) {
  if (operation[0] === "set") {
    style.setProperty(operation[1], operation[2], operation[3]);
  } else if (operation[0] === "remove") {
    style.removeProperty(operation[1]);
  } else {
    style.cssText = operation[1];
  }
  return snapshot(style);
}

const sheet = parseStyleSheet("@font-face {}");
const rule = sheet.cssRules[0];
assert.ok(rule instanceof CSSFontFaceRule);
const actual = operations.map(operation => apply(rule.style, operation));

const browser = await chromium.launch({ headless: true });
try {
  const page = await browser.newPage();
  const expected = await page.evaluate(operations => {
    const sheet = new CSSStyleSheet();
    sheet.insertRule("@font-face {}");
    const style = sheet.cssRules[0].style;
    const snapshot = () => {
      const items = Array.from({ length: style.length }, (_, index) => style.item(index));
      return {
        cssText: style.cssText,
        length: style.length,
        items,
        declarations: items.map(name => ({
          name,
          value: style.getPropertyValue(name),
          priority: style.getPropertyPriority(name),
        })),
      };
    };
    return operations.map(operation => {
      if (operation[0] === "set") {
        style.setProperty(operation[1], operation[2], operation[3]);
      } else if (operation[0] === "remove") {
        style.removeProperty(operation[1]);
      } else {
        style.cssText = operation[1];
      }
      return snapshot();
    });
  }, operations);
  assert.deepEqual(actual, expected);
  console.log(`${operations.length} font-face declaration operations match Chromium.`);
} finally {
  await browser.close();
}
