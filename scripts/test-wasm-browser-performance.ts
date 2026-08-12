import assert from "node:assert/strict";
import { createReadStream } from "node:fs";
import { createServer } from "node:http";
import { stat, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

import { chromium, firefox, webkit } from "playwright";

const browserTypes = new Map([["chromium", chromium], ["firefox", firefox], ["webkit", webkit]]);
const requestedBrowser = process.env.SHEETOM_BROWSER;
if (requestedBrowser && !browserTypes.has(requestedBrowser)) {
  throw new Error(`Unsupported SHEETOM_BROWSER: ${requestedBrowser}`);
}
const selectedBrowsers = requestedBrowser
  ? [[requestedBrowser, browserTypes.get(requestedBrowser)]]
  : [...browserTypes.entries()];
const repositoryRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const distributionRoot = path.join(repositoryRoot, "packages", "wasm", "dist");

const server = createServer(async (request, response) => {
  const pathname = new URL(request.url ?? "/", "http://127.0.0.1").pathname;
  const relative = pathname === "/" ? "index.js" : pathname.slice(1);
  const filename = path.resolve(distributionRoot, relative);
  if (!filename.startsWith(`${distributionRoot}${path.sep}`)) {
    response.writeHead(403).end();
    return;
  }
  try {
    if (!(await stat(filename)).isFile()) throw new Error("not a file");
    response.writeHead(200, {
      "content-type": filename.endsWith(".wasm")
        ? "application/wasm"
        : "text/javascript; charset=utf-8",
    });
    createReadStream(filename).pipe(response);
  } catch {
    response.writeHead(404).end();
  }
});
await new Promise(resolve => server.listen(0, "127.0.0.1", resolve));
const address = server.address();
if (!address || typeof address === "string") throw new Error("HTTP server has no port");
const moduleUrl = `http://127.0.0.1:${address.port}/index.js`;
const observations = [];

try {
  for (const [browserName, browserType] of selectedBrowsers) {
    const browser = await browserType.launch({ headless: true });
    try {
      const page = await browser.newPage();
      await page.goto(moduleUrl);
      const result = await page.evaluate(async url => {
        const initializationStarted = performance.now();
        const { createSheetOM } = await import(url);
        const api = await createSheetOM();
        const initializationMilliseconds = performance.now() - initializationStarted;
        const rule = (selector, index) =>
          `${selector}{width:${index % 200}px;height:20px;color:rgb(10 20 30);padding:1px 2px;margin:3px}`;
        const source = (prefix, count) => {
          const direct = [];
          const responsive = [];
          for (let index = 0; index < count; index += 1) {
            const target = index < Math.floor(count / 2) ? direct : responsive;
            target.push(rule(`.${prefix}-${index}${target === responsive ? ":hover" : ""}`, index));
          }
          return `@layer ${prefix}{${direct.join("")}@media (max-width:767px){${responsive.join("")}}}`;
        };
        const collectStyleRules = ruleList => {
          const result = [];
          const pending = [];
          for (let index = 0; index < ruleList.length; index += 1) pending.push(ruleList[index]);
          while (pending.length > 0) {
            const candidate = pending.pop();
            if (!candidate) continue;
            if (candidate instanceof api.CSSStyleRule) result.push(candidate);
            if ("cssRules" in candidate) {
              for (let index = 0; index < candidate.cssRules.length; index += 1) {
                pending.push(candidate.cssRules[index]);
              }
            }
          }
          return result;
        };
        const sources = [source("shared", 1_000)];
        for (let index = 0; index < 20; index += 1) sources.push(source(`page-${index}`, 500));

        const totalStarted = performance.now();
        const parseStarted = performance.now();
        const sheets = sources.map(css => api.parseStyleSheet(css));
        const parseMilliseconds = performance.now() - parseStarted;
        const mutationStarted = performance.now();
        let mutationCount = 0;
        for (const sheet of sheets) {
          const rules = collectStyleRules(sheet.cssRules);
          for (let index = 0; index < rules.length; index += 10) {
            rules[index].style.setProperty("width", `${index % 200}px`);
            rules[index].style.setProperty("padding-left", `${index % 20}px`);
            rules[index].style.setProperty("--publisher-token", `${index}`);
            mutationCount += 3;
          }
        }
        const mutationMilliseconds = performance.now() - mutationStarted;
        const serializationStarted = performance.now();
        const serialized = sheets.map(sheet => sheet.serialize());
        const serializationMilliseconds = performance.now() - serializationStarted;
        const secondStarted = performance.now();
        const second = sheets.map(sheet => sheet.serialize());
        const secondSerializationMilliseconds = performance.now() - secondStarted;
        if (serialized.some((value, index) => value !== second[index])) {
          throw new Error("browser Publisher serialization is not idempotent");
        }
        return {
          initializationMilliseconds,
          totalMilliseconds: performance.now() - totalStarted,
          parseMilliseconds,
          mutationMilliseconds,
          serializationMilliseconds,
          secondSerializationMilliseconds,
          stylesheetCount: sheets.length,
          ruleCount: 11_000,
          mutationCount,
        };
      }, moduleUrl);

      assert.ok(result.initializationMilliseconds <= 10_000, `${browserName} initialization exceeded 10s`);
      assert.ok(result.totalMilliseconds <= 30_000, `${browserName} Publisher workload exceeded 30s`);
      assert.ok(result.serializationMilliseconds <= 10_000, `${browserName} serialization exceeded 10s`);
      assert.ok(result.secondSerializationMilliseconds <= 5_000, `${browserName} cached serialization exceeded 5s`);
      observations.push({ browser: browserName, version: browser.version(), ...result });
      console.log(JSON.stringify({ browser: browserName, ...result }));
    } finally {
      await browser.close();
    }
  }
} finally {
  await new Promise((resolve, reject) => server.close(error => error ? reject(error) : resolve()));
}
const outputIndex = process.argv.indexOf("--output");
if (outputIndex !== -1) {
  const output = process.argv[outputIndex + 1];
  if (!output) throw new Error("--output requires a path");
  await writeFile(output, `${JSON.stringify({ schemaVersion: 1, observations }, null, 2)}\n`);
}
