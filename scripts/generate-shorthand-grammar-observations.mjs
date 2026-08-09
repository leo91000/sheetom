import { readFile, writeFile } from "node:fs/promises";

import { chromium } from "playwright";

const contractsUrl = new URL("../compatibility/shorthand-grammar-contracts.json", import.meta.url);
const outputUrl = new URL("../compatibility/shorthand-grammar-observations.json", import.meta.url);
const mode = process.argv[2] ?? "--check";
if (!["--check", "--record"].includes(mode)) {
  throw new Error("Usage: generate-shorthand-grammar-observations.mjs [--check|--record]");
}

const contracts = JSON.parse(await readFile(contractsUrl, "utf8"));
const cases = contracts.profiles.flatMap(profile => profile.cases);
const browser = await chromium.launch({ headless: true });

try {
  const page = await browser.newPage();
  const result = await page.evaluate(inputs => {
    const observations = [];
    for (const input of inputs) {
      const style = document.createElement("div").style;
      style.setProperty(input.property, input.input);
      const items = Array.from(style);
      observations.push({
        id: input.id,
        accepted: items.length > 0,
        items,
        longhands: items.map(name => ({
          name,
          value: style.getPropertyValue(name),
          priority: style.getPropertyPriority(name),
        })),
        shorthandValue: style.getPropertyValue(input.property),
        priority: style.getPropertyPriority(input.property),
        cssText: style.cssText,
      });
    }
    return { userAgent: navigator.userAgent, observations };
  }, cases);
  const document = {
    schemaVersion: 1,
    browser: "chromium",
    userAgent: result.userAgent,
    cases: result.observations,
  };
  const serialized = `${JSON.stringify(document, null, 2)}\n`;
  if (mode === "--record") {
    await writeFile(outputUrl, serialized);
    console.log(`Recorded ${document.cases.length} grammar branch observations.`);
  } else {
    const current = await readFile(outputUrl, "utf8");
    if (current !== serialized) {
      throw new Error(
        "Shorthand grammar observations drifted; review the diff and run " +
        "npm run record:shorthand-grammar to accept it",
      );
    }
    console.log(`Verified ${document.cases.length} grammar branch observations.`);
  }
} finally {
  await browser.close();
}
