import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import path from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

import { computeEngineAbiIdentity } from "./engine-abi.mjs";

const repositoryRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");

test("the checked-in Engine ABI Identity matches the Syntax Engine Set", async () => {
  const checkedIn = JSON.parse(await readFile(
    path.join(repositoryRoot, "engine-abi.json"),
    "utf8",
  ));
  assert.deepEqual(checkedIn, await computeEngineAbiIdentity(repositoryRoot));
});
