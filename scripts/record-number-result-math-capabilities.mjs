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
const baseProbes = [
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
const compositeCandidates = [
  { property: "animation", branch: "animation-components", input: "1s ease sign(1em) foo" },
  {
    property: "animation",
    branch: "animation-list",
    input: "1s ease sign(1em) foo, 2s linear sign(1%) bar",
  },
  {
    property: "animation",
    branch: "animation-mixed-contextual-list",
    input: "1s ease sign(1em) foo, 2s linear bar",
  },
  { property: "columns", branch: "columns-width-count", input: "10px sign(1em)" },
  { property: "columns", branch: "columns-count-width", input: "sign(1em) 10px" },
  { property: "flex", branch: "flex-contextual-grow", input: "sign(1em) 2 10px" },
  { property: "flex", branch: "flex-contextual-shrink", input: "1 sign(1em) 10px" },
  {
    property: "flex",
    branch: "flex-contextual-basis",
    input: "1 2 calc(1px * sign(1em))",
  },
  { property: "grid-column", branch: "grid-contextual-end", input: "2 / sign(1em)" },
  { property: "grid-row", branch: "grid-contextual-start", input: "sign(1em) / 3" },
  {
    property: "grid-area",
    branch: "grid-contextual-four-lines",
    input: "1 / sign(1em) / 3 / sign(1%)",
  },
  {
    property: "border-image",
    branch: "border-image-contextual-sections",
    input: "url(a.png) sign(1em) / calc(1px * sign(1em)) / sign(1%) round",
  },
  {
    property: "-webkit-mask-box-image",
    branch: "mask-box-image-contextual-sections",
    input: "url(a.png) sign(1em) / calc(1px * sign(1em)) / sign(1%) round",
  },
  {
    property: "-webkit-border-image",
    branch: "webkit-border-image-contextual-sections",
    input: "url(a.png) sign(1em) / calc(1px * sign(1em)) / sign(1%) round",
  },
  {
    property: "aspect-ratio",
    branch: "aspect-ratio-contextual-pair",
    input: "sign(1em) / calc(sign(1%) + 1)",
  },
  {
    property: "animation",
    branch: "animation-invalid-contextual-dimension",
    input: "1s ease calc(sign(1em) * 1px) foo",
  },
  {
    property: "flex",
    branch: "flex-invalid-contextual-grow-dimension",
    input: "calc(sign(1em) * 1px) 2 10px",
  },
  {
    property: "grid-column",
    branch: "grid-invalid-contextual-dimension",
    input: "2 / calc(sign(1em) * 1px)",
  },
  {
    property: "border-image",
    branch: "border-image-invalid-contextual-slice-dimension",
    input: "url(a.png) calc(sign(1em) * 1px) / 2",
  },
  {
    property: "aspect-ratio",
    branch: "aspect-ratio-invalid-contextual-dimension",
    input: "sign(1em) / calc(sign(1%) * 1px)",
  },
];
const probes = [
  ...baseProbes,
  ...compositeCandidates.map(({ branch, input }) => ({ branch, input })),
];
const properties = [...chromiumSupportedProperties].sort();
const candidates = [
  ...properties.flatMap(property => baseProbes.map(probe => ({
    property,
    ...probe,
  }))),
  ...compositeCandidates,
];

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
