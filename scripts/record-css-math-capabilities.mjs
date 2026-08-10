import assert from "node:assert/strict";
import { readFile, writeFile } from "node:fs/promises";

import { chromium } from "playwright";

const corpusUrl = new URL("../compatibility/value-capabilities.json", import.meta.url);

const groups = [
  ["width", [
    ["number-result", "calc(1)"],
    ["zero-number-result", "calc(0)"],
    ["length-result", "calc(1px)"],
    ["percentage-result", "calc(1%)"],
    ["mixed-sum-dimension-first", "calc(1px + 1%)"],
    ["mixed-sum-percentage-first", "calc(1% + 1px)"],
    ["invalid-dimension-number-sum", "calc(1px + 1)"],
    ["invalid-number-dimension-sum", "calc(1 + 1px)"],
    ["product-dimension-first", "calc(1px * 2)"],
    ["product-number-first", "calc(2 * 1px)"],
    ["division-by-number", "calc(1px / 2)"],
    ["invalid-division-by-dimension", "calc(1px / 1px)"],
    ["min-same-dimension", "min(1px, 2px)"],
    ["invalid-min-number", "min(1px, 2)"],
    ["min-mixed-percentage", "min(1px, 2%)"],
    ["max-mixed-percentage", "max(1%, 2px)"],
    ["clamp-mixed-percentage", "clamp(1px, 2%, 3px)"],
    ["invalid-clamp-number", "clamp(1px, 2, 3px)"],
    ["round-same-dimension", "round(1px, 2px)"],
    ["invalid-round-number-step", "round(1px, 2)"],
    ["rem-same-dimension", "rem(5px, 2px)"],
    ["mod-same-dimension", "mod(5px, 2px)"],
    ["abs-dimension", "abs(-2px)"],
    ["invalid-sign-number-result", "sign(-2px)"],
    ["hypot-same-dimension", "hypot(3px, 4px)"],
    ["invalid-hypot-number", "hypot(3px, 4)"],
    ["invalid-sin-number-result", "sin(0)"],
    ["invalid-sin-angle-number-result", "sin(90deg)"],
    ["invalid-atan2-angle-result", "atan2(1px, 1px)"],
    ["invalid-pow-number-result", "pow(2, 3)"],
    ["invalid-sqrt-number-result", "sqrt(4)"],
    ["positive-infinity-dimension", "calc(infinity * 1px)"],
    ["negative-infinity-dimension", "calc(-infinity * 1px)"],
    ["positive-infinity-percentage", "calc(infinity * 1%)"],
    ["nan-dimension", "calc(NaN * 1px)"],
    ["nested-mixed-product", "calc(2 * (1px + 1%))"],
    ["round-up-dimension", "round(up, 1px, 2px)"],
    ["negative-rem-dimension", "rem(-5px, 2px)"],
    ["negative-mod-dimension", "mod(-5px, 2px)"],
    ["invalid-empty-min", "min()"],
    ["invalid-short-clamp", "clamp(1px, 2px)"],
  ]],
  ["letter-spacing", [
    ["number-result", "calc(1)"],
    ["length-result", "calc(1px)"],
    ["percentage-result", "calc(1%)"],
    ["mixed-sum", "calc(1px + 1%)"],
    ["min-length", "min(1px, 2px)"],
    ["min-mixed-percentage", "min(1px, 2%)"],
    ["invalid-sign-number-result", "sign(1px)"],
    ["hypot-length", "hypot(3px, 4px)"],
  ]],
  ["word-spacing", [
    ["percentage-result", "calc(1%)"],
    ["mixed-sum", "calc(1px + 1%)"],
    ["min-mixed-percentage", "min(1px, 2%)"],
  ]],
  ["transition-duration", [
    ["number-result", "calc(1)"],
    ["time-result", "calc(1s)"],
    ["invalid-time-number-sum", "calc(1s + 1)"],
    ["mixed-time-units", "calc(1s + 1ms)"],
    ["min-time", "min(1s, 2s)"],
    ["invalid-min-number", "min(1s, 2)"],
    ["invalid-sign-number-result", "sign(1s)"],
    ["hypot-time", "hypot(3s, 4s)"],
  ]],
  ["rotate", [
    ["number-result", "calc(1)"],
    ["angle-result", "calc(1deg)"],
    ["invalid-angle-number-sum", "calc(1deg + 1)"],
    ["mixed-angle-units", "calc(1deg + 1rad)"],
    ["min-angle", "min(1deg, 2deg)"],
    ["invalid-min-number", "min(1deg, 2)"],
    ["invalid-sign-number-result", "sign(1deg)"],
    ["atan2-angle-result", "atan2(1, 1)"],
  ]],
  ["opacity", [
    ["number-result", "calc(1)"],
    ["invalid-length-result", "calc(1px)"],
    ["sign-length-number-result", "sign(1px)"],
    ["sin-angle-number-result", "sin(90deg)"],
    ["invalid-atan2-angle-result", "atan2(1px, 1px)"],
    ["pow-number-result", "pow(2, 3)"],
    ["number-division", "calc(1 / 2)"],
    ["positive-infinity", "calc(infinity)"],
    ["negative-infinity", "calc(-infinity)"],
    ["nan", "calc(NaN)"],
    ["sign-negative-absolute-length", "sign(-1px)"],
    ["sign-absolute-length", "sign(1cm)"],
    ["sign-time", "sign(1s)"],
    ["sign-angle", "sign(1deg)"],
  ]],
  ["z-index", [
    ["number-result", "calc(1)"],
    ["fractional-number-result", "calc(1.5)"],
    ["invalid-length-result", "calc(1px)"],
    ["sign-length-number-result", "sign(1px)"],
    ["sin-angle-number-result", "sin(90deg)"],
    ["invalid-atan2-angle-result", "atan2(1px, 1px)"],
    ["pow-number-result", "pow(2, 3)"],
    ["number-division", "calc(1 / 2)"],
    ["negative-number-division", "calc(-1 / 2)"],
    ["sign-negative-absolute-length", "sign(-1px)"],
    ["sign-time", "sign(1s)"],
    ["sign-angle", "sign(1deg)"],
  ]],
];

const candidates = groups.flatMap(([property, values]) => values.map(([branch, input]) => ({
  id: `math.${property}.${branch}`,
  property,
  input,
})));
assert.equal(new Set(candidates.map(candidate => candidate.id)).size, candidates.length);

const browser = await chromium.launch({ headless: true });
let observations;
let userAgent;
try {
  const page = await browser.newPage();
  ({ observations, userAgent } = await page.evaluate(testCases => ({
    userAgent: navigator.userAgent,
    observations: testCases.map(testCase => {
      const style = document.createElement("div").style;
      style.setProperty(testCase.property, testCase.input);
      return {
        ...testCase,
        accepted: style.length === 1,
        observable: style.getPropertyValue(testCase.property),
      };
    }),
  }), candidates));
} finally {
  await browser.close();
}

const chromiumMajor = Number(/(?:Chrome|HeadlessChrome)\/(\d+)/.exec(userAgent)?.[1]);
assert.equal(chromiumMajor, 151, `expected pinned Chromium 151, got ${userAgent}`);

const corpus = JSON.parse(await readFile(corpusUrl, "utf8"));
const retained = corpus.cases.filter(candidate => !candidate.id.startsWith("math."));
const mathCases = observations.map(candidate => candidate.accepted ? candidate : {
  id: candidate.id,
  property: candidate.property,
  input: candidate.input,
  accepted: false,
});
corpus.cases = [...retained, ...mathCases];
await writeFile(corpusUrl, `${JSON.stringify(corpus, null, 2)}\n`);
console.log(`Recorded ${mathCases.length} CSS Math cases from Chromium ${chromiumMajor}.`);
