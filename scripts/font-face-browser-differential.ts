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

const descriptorCases = [
  ["ascent-override", "normal", "10%", true],
  ["ascent-override", "0%", "10%", true],
  ["ascent-override", "min(10%, 20%)", "10%", true],
  ["ascent-override", "max(10%, 20%)", "10%", true],
  ["ascent-override", "clamp(10%, 20%, 30%)", "10%", true],
  ["ascent-override", "round(10%, 3%)", "10%", true],
  ["ascent-override", "min(max(10%, 20%), 30%)", "10%", true],
  ["ascent-override", "calc(-1%)", "10%", true],
  ["ascent-override", "sign(1%)", "10%", false],
  ["ascent-override", "-1%", "10%", false],
  ["size-adjust", "100%", "90%", true],
  ["size-adjust", "calc(50% + 50%)", "90%", true],
  ["size-adjust", "calc(sign(1%) * 100%)", "90%", true],
  ["size-adjust", "normal", "90%", false],
  ["size-adjust", "-1%", "90%", false],
  ["font-display", "auto", "swap", true],
  ["font-display", "optional", "swap", true],
  ["font-display", "initial", "swap", false],
  ["font-display", "swap block", "swap", false],
  ["font-family", "Test", "Fallback", true],
  ["font-family", '"A B"', "Fallback", true],
  ["font-family", "A, B", "Fallback", false],
  ["font-family", "generic(sans-serif)", "Fallback", false],
  ["font-feature-settings", "normal", '"kern"', true],
  ["font-feature-settings", '"kern" on, "liga" off', '"kern"', true],
  ["font-feature-settings", '"abc"', '"kern"', false],
  ["font-feature-settings", '"kern" -1', '"kern"', true],
  ["font-variation-settings", "normal", '"wght" 400', true],
  ["font-variation-settings", '"wght" 500, "wdth" 90', '"wght" 400', true],
  ["font-variation-settings", '"abc" 1', '"wght" 400', false],
  ["font-variation-settings", '"wght" calc(400 + 100)', '"wght" 400', true],
  ["font-style", "normal", "italic", true],
  ["font-style", "oblique", "italic", true],
  ["font-style", "oblique 10deg 20deg", "italic", true],
  ["font-style", "oblique 20deg 10deg", "italic", true],
  ["font-weight", "normal", "bold", true],
  ["font-weight", "100 900", "bold", true],
  ["font-weight", "900 100", "bold", true],
  ["font-weight", "calc(400 + 100)", "bold", true],
  ["font-stretch", "normal", "condensed", true],
  ["font-stretch", "75% 125%", "condensed", true],
  ["font-stretch", "125% 75%", "condensed", true],
  ["font-stretch", "calc(-1%)", "75% 125%", true],
  ["font-stretch", "min(75%, 125%)", "75% 125%", true],
  [
    "font-stretch",
    "min(75%, 125%) max(100%, 150%)",
    "75% 125%",
    true,
  ],
  ["font-stretch", "calc(75%) 125%", "75% 125%", true],
  ["font-stretch", "75% calc(125%)", "75% 125%", true],
  ["font-stretch", "calc(-1%) 125%", "75% 125%", true],
  ["font-stretch", "75% calc(-1%)", "75% 125%", true],
  ["font-stretch", "round(100%, 25%)", "75% 125%", true],
  ["font-stretch", "calc(sign(1%) * 100%)", "75% 125%", true],
  ["font-stretch", "sign(1%)", "75% 125%", false],
  ["font-stretch", "condensed expanded", "condensed", false],
  ["font-stretch", "normal 125%", "condensed", false],
  ["font-stretch", "75% normal", "75% 125%", false],
  ["font-stretch", "75% -1%", "75% 125%", false],
  ["src", "local(Test)", "local(Fallback)", true],
  ["src", "url(a.woff2) format(woff2)", "local(Fallback)", true],
  ["src", "url(a.woff2) tech(color-colrv1, variations)", "local(Fallback)", true],
  ["src", "none", "local(Fallback)", false],
  ["unicode-range", "U+??", "U+0", true],
  ["unicode-range", "U+26, U+0-7F", "U+0", true],
  ["unicode-range", "U+110000", "U+0", false],
  ["unicode-range", "U+2??-3??", "U+0", false],
  ["font-variant", "normal", "small-caps", true],
  ["font-variant", "small-caps", "normal", true],
  ["font-variant", "historical-forms", "normal", false],
  ["font-variant", "small-caps oldstyle-nums", "normal", false],
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

function createSheetomFontFaceStyle() {
  const sheet = parseStyleSheet("@font-face {}");
  const rule = sheet.cssRules[0];
  assert.ok(rule instanceof CSSFontFaceRule);
  return rule.style;
}

function evaluateDescriptorCase(createStyle, [name, input, baseline, accepted]) {
  const blank = createStyle();
  blank.setProperty(name, input);
  const blankSnapshot = snapshot(blank);
  assert.equal(blankSnapshot.length, accepted ? 1 : 0, `${name}: ${input}`);

  const atomic = createStyle();
  atomic.setProperty(name, baseline);
  const before = snapshot(atomic);
  atomic.setProperty(name, input);
  const after = snapshot(atomic);
  if (!accepted) assert.deepEqual(after, before, `${name}: ${input}`);
  return { blank: blankSnapshot, atomic: after };
}

const actualStyle = createSheetomFontFaceStyle();
const actual = operations.map(operation => apply(actualStyle, operation));
const actualCases = descriptorCases.map(candidate =>
  evaluateDescriptorCase(createSheetomFontFaceStyle, candidate));

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
  const expectedCases = await page.evaluate(cases => {
    const snapshot = style => {
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
    const createStyle = () => {
      const sheet = new CSSStyleSheet();
      sheet.insertRule("@font-face {}");
      return sheet.cssRules[0].style;
    };
    return cases.map(([name, input, baseline, accepted]) => {
      const blank = createStyle();
      blank.setProperty(name, input);
      const blankSnapshot = snapshot(blank);
      if (blankSnapshot.length !== (accepted ? 1 : 0)) {
        throw new Error(`stale font-face capability: ${name}: ${input}`);
      }
      const atomic = createStyle();
      atomic.setProperty(name, baseline);
      const before = snapshot(atomic);
      atomic.setProperty(name, input);
      const after = snapshot(atomic);
      if (!accepted && JSON.stringify(after) !== JSON.stringify(before)) {
        throw new Error(`non-atomic font-face capability: ${name}: ${input}`);
      }
      return { blank: blankSnapshot, atomic: after };
    });
  }, descriptorCases);
  assert.deepEqual(actualCases, expectedCases);
  console.log(
    `${operations.length} font-face declaration operations and ` +
    `${descriptorCases.length} descriptor branches match Chromium.`,
  );
} finally {
  await browser.close();
}
