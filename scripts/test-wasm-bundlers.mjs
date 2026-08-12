import { execFileSync } from "node:child_process";
import { cp, mkdir, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { createRequire } from "node:module";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

import { nodeResolve } from "@rollup/plugin-node-resolve";
import { build as esbuild } from "esbuild";
import { chromium, firefox, webkit } from "playwright";
import { rollup } from "rollup";
import { build as viteBuild } from "vite";
import webpack from "webpack";

const repositoryRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const require = createRequire(import.meta.url);
const selectedBrowser = process.env.SHEETOM_BROWSER;
const browsers = selectedBrowser === undefined
  ? [["chromium", chromium], ["firefox", firefox], ["webkit", webkit]]
  : [[selectedBrowser, { chromium, firefox, webkit }[selectedBrowser]]];
if (browsers.some(([, browserType]) => browserType === undefined)) {
  throw new Error(`Unsupported SHEETOM_BROWSER: ${selectedBrowser}`);
}

const temporaryRoot = await mkdtemp(path.join(os.tmpdir(), "sheetom-wasm-bundlers-"));
const packageOutput = path.join(temporaryRoot, "package");
const consumerRoot = path.join(temporaryRoot, "consumer");
const sourceRoot = path.join(consumerRoot, "src");
const packageRoot = path.join(consumerRoot, "node_modules", "@sheetom", "wasm");

function runNpm(arguments_, cwd) {
  execFileSync("npm", arguments_, { cwd, stdio: "inherit" });
}

async function writeConsumerSources() {
  const scenario = `
    const api = await createSheetOM();
    const sheet = new api.CSSStyleSheet();
    sheet.replaceSync('.card { background: image-set(url(a.png) 1x, url(b.png) 2x) red; }');
    sheet.cssRules[0].style.setProperty('padding', '72px var(--space, var(--space,');
    const serialized = sheet.serialize();
    if (!serialized.includes('image-set(') || !serialized.includes('var(--space')) {
      throw new Error('bundled SheetOM mutation failed');
    }
    return serialized;
  `;
  await writeFile(path.join(sourceRoot, "app.js"), `
    import { createSheetOM } from '@sheetom/wasm';
    globalThis.sheetomMain = (async () => {${scenario}})();
    const worker = new Worker(new URL('./worker.js', import.meta.url), { type: 'module' });
    globalThis.sheetomWorker = new Promise((resolve, reject) => {
      worker.onmessage = event => event.data.ok ? resolve(event.data.value) : reject(new Error(event.data.error));
      worker.onerror = reject;
    });
  `);
  await writeFile(path.join(sourceRoot, "worker.js"), `
    import { createSheetOM } from '@sheetom/wasm';
    (async () => {${scenario}})()
      .then(value => postMessage({ ok: true, value }))
      .catch(error => postMessage({ ok: false, error: error.stack ?? String(error) }));
  `);
  await writeFile(path.join(sourceRoot, "index.html"), "<script type=\"module\" src=\"./app.js\"></script>\n");
}

async function prepareOutput(name) {
  const output = path.join(consumerRoot, "dist", name);
  await mkdir(output, { recursive: true });
  await cp(path.join(sourceRoot, "index.html"), path.join(output, "index.html"));
  return output;
}

async function copyWasm(output) {
  await cp(
    path.join(packageRoot, "dist", "sheetom_wasm_bg.wasm"),
    path.join(output, "sheetom_wasm_bg.wasm"),
  );
}

async function bundleEsbuild() {
  const output = await prepareOutput("esbuild");
  for (const entry of ["app", "worker"]) {
    await esbuild({
      entryPoints: [path.join(sourceRoot, `${entry}.js`)],
      outfile: path.join(output, `${entry}.js`),
      bundle: true,
      format: "esm",
      platform: "browser",
      target: ["es2022"],
      sourcemap: false,
    });
  }
  await copyWasm(output);
}

async function bundleRollup() {
  const output = await prepareOutput("rollup");
  for (const entry of ["app", "worker"]) {
    const bundle = await rollup({
      input: path.join(sourceRoot, `${entry}.js`),
      plugins: [nodeResolve({ browser: true })],
      onwarn(warning) {
        throw new Error(`Rollup warning: ${warning.message}`);
      },
    });
    await bundle.write({ file: path.join(output, `${entry}.js`), format: "es" });
    await bundle.close();
  }
  await copyWasm(output);
}

async function bundleVite() {
  const output = await prepareOutput("vite");
  for (const entry of ["app", "worker"]) {
    await viteBuild({
      configFile: false,
      root: consumerRoot,
      logLevel: "error",
      build: {
        emptyOutDir: false,
        lib: {
          entry: path.join(sourceRoot, `${entry}.js`),
          formats: ["es"],
          fileName: () => `${entry}.js`,
        },
        outDir: output,
        target: "es2022",
        minify: false,
      },
    });
  }
  await copyWasm(output);
}

async function bundleWebpack() {
  const output = await prepareOutput("webpack");
  const compiler = webpack({
    mode: "production",
    target: ["web", "es2022"],
    context: consumerRoot,
    entry: {
      app: path.join(sourceRoot, "app.js"),
      worker: path.join(sourceRoot, "worker.js"),
    },
    output: {
      path: output,
      filename: "[name].js",
      chunkFilename: "[name].js",
      clean: false,
    },
    optimization: { minimize: false },
    performance: { hints: false },
  });
  await new Promise((resolve, reject) => {
    compiler.run((error, stats) => {
      compiler.close(() => {});
      if (error) return reject(error);
      if (stats?.hasErrors()) return reject(new Error(stats.toString({ all: false, errors: true })));
      if (stats?.hasWarnings()) return reject(new Error(stats.toString({ all: false, warnings: true })));
      resolve();
    });
  });
  if (!(await readFile(path.join(output, "app.js"), "utf8")).includes("sheetom_wasm_bg")) {
    await copyWasm(output);
  }
}

function mimeType(filename) {
  if (filename.endsWith(".html")) return "text/html";
  if (filename.endsWith(".js")) return "text/javascript";
  if (filename.endsWith(".wasm")) return "application/wasm";
  return "application/octet-stream";
}

async function startServer(root) {
  const { createServer } = await import("node:http");
  const server = createServer(async (request, response) => {
    try {
      const pathname = new URL(request.url ?? "/", "http://localhost").pathname;
      const filename = path.join(root, pathname === "/" ? "index.html" : pathname.slice(1));
      response.writeHead(200, { "content-type": mimeType(filename) });
      response.end(await readFile(filename));
    } catch {
      response.writeHead(404);
      response.end();
    }
  });
  await new Promise(resolve => server.listen(0, "127.0.0.1", resolve));
  const address = server.address();
  if (!address || typeof address === "string") throw new Error("Unable to bind test server");
  return { server, url: `http://127.0.0.1:${address.port}/` };
}

try {
  await mkdir(packageOutput);
  await mkdir(sourceRoot, { recursive: true });
  const pack = JSON.parse(execFileSync(
    "npm",
    ["pack", "--workspace", "@sheetom/wasm", "--json", "--pack-destination", packageOutput],
    { cwd: repositoryRoot, encoding: "utf8" },
  ))[0];
  if (!pack?.filename) throw new Error("Unable to pack @sheetom/wasm");
  await writeFile(path.join(consumerRoot, "package.json"), '{"private":true,"type":"module"}\n');
  runNpm(["install", "--ignore-scripts", path.join(packageOutput, pack.filename)], consumerRoot);
  await writeConsumerSources();

  await bundleEsbuild();
  await bundleRollup();
  await bundleVite();
  await bundleWebpack();

  for (const bundler of ["esbuild", "rollup", "vite", "webpack"]) {
    const { server, url } = await startServer(path.join(consumerRoot, "dist", bundler));
    try {
      for (const [browserName, browserType] of browsers) {
        const browser = await browserType.launch({ headless: true });
        try {
          const page = await browser.newPage();
          await page.goto(url);
          await page.evaluate(() => Promise.all([globalThis.sheetomMain, globalThis.sheetomWorker]));
          console.log(`Verified ${bundler} in ${browserName} on the main thread and worker.`);
        } finally {
          await browser.close();
        }
      }
    } finally {
      await new Promise(resolve => server.close(resolve));
    }
  }
} finally {
  await rm(temporaryRoot, { recursive: true, force: true });
}
