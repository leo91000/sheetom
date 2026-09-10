import assert from "node:assert/strict";
import { mkdtemp, rm } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import { spawnSync } from "node:child_process";
import { test } from "node:test";
import { fileURLToPath } from "node:url";
import { backendEntry } from "./validation-backend.ts";

test("installed backend selection uses the requested prefix for both packages", () => {
  const prefix = path.join(os.tmpdir(), "cold install with spaces");
  assert.equal(fileURLToPath(backendEntry("native", prefix)), path.join(prefix, "node_modules/sheetom/dist/index.js"));
  assert.equal(fileURLToPath(backendEntry("wasm", prefix)), path.join(prefix, "node_modules/@sheetom/wasm/dist/index.js"));
  assert.throws(() => backendEntry("native", ""), /nonempty/);
});

test("missing installed packages fail before checks and never use local builds", async () => {
  const prefix = await mkdtemp(path.join(os.tmpdir(), "sheetom-missing-"));
  try {
    const result = spawnSync(process.execPath, [fileURLToPath(new URL("check-installed-packages.ts", import.meta.url)), "--package-root=" + prefix, "--report-dir=" + path.join(prefix, "reports")], { encoding: "utf8" });
    assert.notEqual(result.status, 0);
    assert.match(result.stderr, /ENOENT/);
    assert.ok(result.stderr.includes(prefix));
    assert.doesNotMatch(result.stdout, /Running modern-contracts/);
  } finally { await rm(prefix, { recursive: true, force: true }); }
});
