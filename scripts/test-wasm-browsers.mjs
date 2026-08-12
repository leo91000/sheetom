import assert from "node:assert/strict";
import { createReadStream } from "node:fs";
import { stat, writeFile } from "node:fs/promises";
import { createServer } from "node:http";
import path from "node:path";
import { fileURLToPath } from "node:url";

import { chromium, firefox, webkit } from "playwright";

const browserTypes = new Map([
  ["chromium", chromium],
  ["firefox", firefox],
  ["webkit", webkit],
]);
const requestedBrowser = process.env.SHEETOM_BROWSER;
if (requestedBrowser && !browserTypes.has(requestedBrowser)) {
  throw new Error(`Unsupported SHEETOM_BROWSER: ${requestedBrowser}`);
}
const selectedBrowsers = requestedBrowser
  ? [browserTypes.get(requestedBrowser)]
  : [...browserTypes.values()];

const repositoryRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const distributionRoot = path.join(repositoryRoot, "packages", "wasm", "dist");
const workerSource = `
  self.onmessage = async event => {
    try {
      const { createSheetOM } = await import(event.data.moduleUrl);
      const api = await createSheetOM();
      const sheet = new api.CSSStyleSheet();
      sheet.replaceSync('.worker { color: red; }');
      self.postMessage({ ok: true, css: sheet.serialize() });
    } catch (error) {
      self.postMessage({ ok: false, error: String(error?.stack ?? error) });
    }
  };
`;

const server = createServer(async (request, response) => {
  const url = new URL(request.url ?? "/", "http://127.0.0.1");
  if (url.pathname === "/worker.js") {
    response.writeHead(200, { "content-type": "text/javascript; charset=utf-8" });
    response.end(workerSource);
    return;
  }
  const relative = url.pathname === "/" ? "index.js" : url.pathname.slice(1);
  const filename = path.resolve(distributionRoot, relative);
  if (!filename.startsWith(`${distributionRoot}${path.sep}`)) {
    response.writeHead(403).end();
    return;
  }
  try {
    const metadata = await stat(filename);
    if (!metadata.isFile()) throw new Error("not a file");
    const contentType = filename.endsWith(".wasm")
      ? "application/wasm"
      : "text/javascript; charset=utf-8";
    response.writeHead(200, { "content-type": contentType });
    createReadStream(filename).pipe(response);
  } catch {
    response.writeHead(404).end();
  }
});
await new Promise(resolve => server.listen(0, "127.0.0.1", resolve));
const address = server.address();
if (!address || typeof address === "string") throw new Error("HTTP server has no port");
const origin = `http://127.0.0.1:${address.port}`;
const observations = [];

try {
  for (const browserType of selectedBrowsers) {
    if (!browserType) throw new Error("Missing selected browser");
    const browser = await browserType.launch({ headless: true });
    try {
      const page = await browser.newPage();
      await page.goto(origin);
      const result = await page.evaluate(async ({ moduleUrl, workerUrl }) => {
        const { createSheetOM } = await import(moduleUrl);
        const [first, concurrent] = await Promise.all([createSheetOM(), createSheetOM()]);
        const response = await fetch(new URL("./sheetom_wasm_bg.wasm", moduleUrl));
        const independent = await createSheetOM(response);
        const wasmBytes = await (
          await fetch(new URL("./sheetom_wasm_bg.wasm", moduleUrl))
        ).arrayBuffer();
        const bufferedFallback = await createSheetOM(new Response(wasmBytes, {
          headers: { "content-type": "application/octet-stream" },
        }));
        const sheet = new first.CSSStyleSheet();
        sheet.replaceSync(`
          @layer components {
            .card {
              background: image-set(url(a.png) 1x, url(b.png) 2x) center / cover no-repeat red;
            }
          }
        `);
        sheet.cssRules[0].cssRules[0].style.setProperty(
          "padding",
          "72px var(--space, var(--space,",
        );
        const workerResult = await new Promise((resolve, reject) => {
          const worker = new Worker(workerUrl, { type: "module" });
          worker.onmessage = event => {
            worker.terminate();
            resolve(event.data);
          };
          worker.onerror = event => reject(new Error(event.message));
          worker.postMessage({ moduleUrl });
        });
        return {
          frozen: Object.isFrozen(first),
          sharedDefault: first === concurrent,
          independentClasses: first.CSSStyleSheet !== independent.CSSStyleSheet,
          bufferedFallbackClasses:
            independent.CSSStyleSheet !== bufferedFallback.CSSStyleSheet,
          serialized: sheet.serialize(),
          workerResult,
        };
      }, {
        moduleUrl: `${origin}/index.js`,
        workerUrl: `${origin}/worker.js`,
      });
      assert.equal(result.frozen, true, `${browserType.name()} facade is frozen`);
      assert.equal(result.sharedDefault, true, `${browserType.name()} shares default init`);
      assert.equal(
        result.independentClasses,
        true,
        `${browserType.name()} isolates explicit instances`,
      );
      assert.equal(
        result.bufferedFallbackClasses,
        true,
        `${browserType.name()} buffers a non-WASM MIME response`,
      );
      assert.match(result.serialized, /image-set\(/u, browserType.name());
      assert.match(result.serialized, /var\(--space/u, browserType.name());
      assert.deepEqual(result.workerResult, {
        ok: true,
        css: ".worker {\n  color: red;\n}\n",
      });
      observations.push({
        browser: browserType.name(),
        version: browser.version(),
        mainThread: true,
        worker: true,
        streaming: true,
        bufferedFallback: true,
        independentInstances: true,
      });
    } finally {
      await browser.close();
    }
  }
} finally {
  await new Promise((resolve, reject) => server.close(error => {
    if (error) reject(error);
    else resolve();
  }));
}

console.log(
  `Verified the direct HTTP WASM backend in ${selectedBrowsers
    .map(browserType => browserType?.name())
    .join(", ")}.`,
);
const outputIndex = process.argv.indexOf("--output");
if (outputIndex !== -1) {
  const output = process.argv[outputIndex + 1];
  if (!output) throw new Error("--output requires a path");
  await writeFile(output, `${JSON.stringify({ schemaVersion: 1, observations }, null, 2)}\n`);
}
