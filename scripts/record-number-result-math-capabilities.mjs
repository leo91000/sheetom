import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { readFile, writeFile } from "node:fs/promises";

import { chromium } from "playwright";

import {
  chromiumPropertyBaseline,
  chromiumShorthandLonghands,
  chromiumSupportedProperties,
} from "../src/chromium-properties.ts";

const outputUrl = new URL(
  "../compatibility/number-result-math-capabilities.json",
  import.meta.url,
);
const manifestUrl = new URL("../src/chromium-properties.ts", import.meta.url);
const probes = [
  { branch: "relative-length", input: "sign(1em)" },
  { branch: "relative-length-negative", input: "sign(-1em)" },
  { branch: "percentage", input: "sign(1%)" },
  {
    branch: "mixed-relative-length",
    input: "sign(calc(1px - 2em))",
  },
  {
    branch: "nested-number-product",
    input: "calc(sign(1em) * 2)",
  },
  {
    branch: "nested-number-sum",
    input: "calc(sign(1em) + 1)",
  },
  {
    branch: "dimension-result-neighbor",
    input: "calc(sign(1em) * 1px)",
  },
  {
    branch: "dimension-quotient",
    input: "calc(1px / sign(1em))",
  },
  {
    branch: "dynamic-number-product",
    input: "calc(sign(1em) * sign(1rem))",
  },
  {
    branch: "dynamic-number-quotient",
    input: "calc(sign(1em) / sign(1rem))",
  },
  {
    branch: "invalid-dimension-product",
    input: "calc(1px * 1em)",
  },
  { branch: "invalid-relative-sine", input: "sin(1em)" },
];
const properties = [...chromiumSupportedProperties].sort();
const candidates = properties.flatMap(property => probes.map(probe => ({
  property,
  ...probe,
})));

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
      const accepted = style.length > 0;
      return {
        ...testCase,
        accepted,
        ...(accepted ? {
          observable: style.getPropertyValue(testCase.property),
          items: Array.from(style),
          cssText: style.cssText,
        } : {}),
      };
    }),
  }), candidates));
} finally {
  await browser.close();
}

const chromiumMajor = Number(/(?:Chrome|HeadlessChrome)\/(\d+)/.exec(userAgent)?.[1]);
assert.equal(chromiumMajor, 151, `expected pinned Chromium 151, got ${userAgent}`);
assert.equal(
  userAgent,
  chromiumPropertyBaseline,
  "CSS Math and property manifest baselines must use the same Chromium build",
);

const supportedProperties = new Set(
  observations
    .filter(candidate => candidate.branch === "relative-length" && candidate.accepted)
    .map(candidate => candidate.property),
);
const shorthandProperties = new Set(
  observations
    .filter(candidate =>
      candidate.branch === "relative-length" &&
      candidate.accepted &&
      (candidate.items.length > 1 || Object.hasOwn(chromiumShorthandLonghands, candidate.property)))
    .map(candidate => candidate.property),
);
const directNumberProperties = new Set(
  observations
    .filter(candidate =>
      candidate.branch === "relative-length" &&
      candidate.accepted &&
      candidate.items.length === 1 &&
      candidate.observable === candidate.input)
    .map(candidate => candidate.property),
);
const cases = observations
  .filter(candidate => supportedProperties.has(candidate.property))
  .map(candidate => ({
    id: `number-result-math.${candidate.property}.${candidate.branch}`,
    shorthand: shorthandProperties.has(candidate.property),
    integration: directNumberProperties.has(candidate.property)
      ? "direct-number"
      : "composite-property",
    ...candidate,
  }));
assert.equal(new Set(cases.map(candidate => candidate.id)).size, cases.length);

const manifestSha256 = createHash("sha256")
  .update(await readFile(manifestUrl))
  .digest("hex");
const corpus = {
  schemaVersion: 1,
  baseline: {
    browser: "chromium",
    major: chromiumMajor,
    userAgent,
    propertyManifestSha256: manifestSha256,
  },
  probes,
  cases,
};
await writeFile(outputUrl, `${JSON.stringify(corpus, null, 2)}\n`);
console.log(
  `Recorded ${cases.length} CSS Math observations for ` +
  `${supportedProperties.size} Chromium properties.`,
);
