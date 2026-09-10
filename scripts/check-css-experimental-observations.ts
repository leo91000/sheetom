import assert from "node:assert/strict";
import { readFile, writeFile } from "node:fs/promises";
import { chromium } from "playwright";
const browser = await chromium.launch({ args: ["--enable-blink-features=CSSMixins"] });
try {
  const page = await browser.newPage();
  const sources = [
    "@mixin --theme(--color <color>: red) { color: var(--color); @contents; } .a { @apply --theme(blue); }",
    "@mixin --empty() {} .a { @apply --empty(); @apply --empty() {} }",
    "@mixin --card() { @result { color: red; @contents { color: blue; } } }",
  ];
  const observations = await page.evaluate(sources => sources.map(source => {
    const sheet = new CSSStyleSheet(); sheet.replaceSync(source);
    const snapshot = rule => ({ interface: rule.constructor.name, cssText: rule.cssText,
      ...(typeof rule.getParameters === "function" ? { parameters: rule.getParameters() } : {}),
      ...(typeof rule.getArguments === "function" ? { arguments: rule.getArguments() } : {}),
      children: rule.cssRules ? Array.from(rule.cssRules, snapshot) : [] });
    return { source, rules: Array.from(sheet.cssRules, snapshot) };
  }), sources);
  const report = { schemaVersion: 1, browser: browser.version(), flags: ["--enable-blink-features=CSSMixins"], authority: "Experimental browser observation only. The five SheetOM mixin interfaces follow CSSWG revision 5e68d5c1ca6656dd7a4b32f821d187aa9652ad5d; these browser interfaces and older body grammar do not override it.", observations };
  const url = new URL("../compatibility/css-experimental-observations.json", import.meta.url);
  const serialized = `${JSON.stringify(report, null, 2)}\n`;
  if (process.argv.includes("--record")) await writeFile(url, serialized);
  else assert.equal(await readFile(url, "utf8"), serialized, "Experimental browser evidence drifted; review the draft/browser resolution.");
  console.log(`Experimental mixin observations verified against Chromium ${browser.version()}.`);
} finally { await browser.close(); }
