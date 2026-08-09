import { createHash } from "node:crypto";
import { readFile, writeFile } from "node:fs/promises";

import { chromium } from "playwright";

import { chromiumShorthandLonghands } from "../src/chromium-properties.ts";

const manifestUrl = new URL("../src/chromium-properties.ts", import.meta.url);
const outputUrl = new URL("../compatibility/shorthand-capabilities.json", import.meta.url);
const manifestSource = await readFile(manifestUrl);
const manifestSha256 = createHash("sha256").update(manifestSource).digest("hex");
const shorthands = Object.fromEntries(
  Object.entries(chromiumShorthandLonghands)
    .filter(([, longhands]) => longhands.length > 1),
);

const browser = await chromium.launch({ headless: true });

try {
  const page = await browser.newPage();
  const result = await page.evaluate(groups => {
    const element = document.createElement("div");
    document.body.append(element);
    const computed = getComputedStyle(element);
    const cases = [];

    for (const [property, expectedLonghands] of Object.entries(groups)) {
      const seedStyle = document.createElement("div").style;
      for (const longhand of expectedLonghands) {
        seedStyle.setProperty(longhand, computed.getPropertyValue(longhand));
      }

      let input = seedStyle.getPropertyValue(property);
      let source = "generated";
      if (property === "-webkit-mask-box-image" && input === "") {
        input = "none";
        source = "manual";
      }

      const probe = document.createElement("div").style;
      probe.setProperty(property, input);
      const items = Array.from(probe);
      const longhands = items.map(name => ({
        name,
        value: probe.getPropertyValue(name),
        priority: probe.getPropertyPriority(name),
      }));

      cases.push({
        id: `shorthand.${property}.concrete-default`,
        property,
        input,
        source,
        chromium: {
          accepted: items.length > 0,
          items,
          longhands,
          shorthandValue: probe.getPropertyValue(property),
          shorthandPriority: probe.getPropertyPriority(property),
          cssText: probe.cssText,
        },
        mutationProbe: {
          longhand: expectedLonghands[0],
          override: "inherit",
        },
      });
    }

    element.remove();
    return { userAgent: navigator.userAgent, cases };
  }, shorthands);

  const corpus = {
    schemaVersion: 1,
    baseline: {
      browser: "chromium",
      userAgent: result.userAgent,
      propertyManifestSha256: manifestSha256,
      derivation: "computed-initial-longhands@1",
    },
    cases: result.cases,
  };
  await writeFile(outputUrl, `${JSON.stringify(corpus, null, 2)}\n`);
  console.log(
    `Wrote ${corpus.cases.length} concrete shorthand cases from ${result.userAgent}`,
  );
} finally {
  await browser.close();
}
