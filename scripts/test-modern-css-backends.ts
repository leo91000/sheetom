import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { spawnSync } from "node:child_process";
import { transform } from "esbuild";
import * as roundtrip from "./css-authoring-roundtrip.ts";

const backend = process.argv.find(argument => argument.startsWith("--backend="))?.slice(10);
if (!backend) {
  for (const backend of ["native", "wasm"]) {
    const result = spawnSync(process.execPath, [import.meta.filename, `--backend=${backend}`], { encoding: "utf8", timeout: 120_000 });
    assert.equal(result.signal, null, `${backend}: ${result.stderr}`);
    assert.equal(result.status, 0, `${backend}: ${result.stdout}\n${result.stderr}`);
    process.stdout.write(result.stdout);
  }
} else {
  const api = backend === "native" ? await import("../dist/index.js")
    : await (await import("../packages/wasm/dist/index.js")).createSheetOM(new Uint8Array(await readFile(new URL("../packages/wasm/dist/sheetom_wasm_bg.wasm", import.meta.url))).buffer);
  // Execute exactly the unit contract against each packaged public facade.
  // Only the test registration and module imports are substituted; assertions
  // remain shared so backend tests cannot silently omit new regression cases.
  let checks = 0;
  for (const filename of ["mixin-rules", "css-namespace", "selector-text", "modern-css-roundtrip", "baseline-september"]) {
    const source = await readFile(new URL(`../tests/${filename}.test.ts`, import.meta.url), "utf8");
    const { code } = await transform(source, { loader: "ts", format: "cjs", target: "es2022" });
    const require = name => {
      if (name === "node:assert/strict") return assert;
      if (name === "vitest") return { test: (_name, run) => { run(); checks++; } };
      if (name === "../src/index.js") return api;
      if (name === "../scripts/css-authoring-roundtrip.ts") return roundtrip;
      throw new Error(`Unexpected contract dependency: ${name}`);
    };
    new Function("require", "module", "exports", code)(require, { exports: {} }, {});
  }
  for (const depth of [256, 1024, 4000]) {
    assert.equal(api.CSS.supports("(".repeat(depth) + "color: red" + ")".repeat(depth)), true);
    assert.equal(api.CSS.supports("selector(" + ":is(".repeat(depth) + ".x" + ")".repeat(depth) + ")"), true);
    const sheet = new api.CSSStyleSheet();
    sheet.replaceSync("@mixin --deep {" + "@media all {".repeat(depth) + "color: red;" + "}".repeat(depth + 1));
    assert.ok(sheet.cssRules[0] instanceof api.CSSMixinRule);
    assert.match(sheet.cssRules[0].cssText, /color: red;/u);
    assert.match(sheet.serializeStrict(), /color: red;/u);
  }
  assert.throws(() => api.CSS.supports("(".repeat(4100) + "color:red" + ")".repeat(4100)), RangeError);
  assert.equal(api.CSS.supports("color", "red"), true, "resource failure leaves the instance usable");
  console.log(`${backend}: ${checks} shared modern CSS contracts and depth 256/1024/4000 passed.`);
}
