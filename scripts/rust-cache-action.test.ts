import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

const action = await readFile(
  new URL("../.github/actions/setup-rust-cache/action.yml", import.meta.url),
  "utf8",
);

test("the optional Rust compiler cache cannot block correctness", () => {
  assert.match(action, /id: sccache/u);
  assert.match(action, /continue-on-error: true/u);
  assert.match(action, /if: steps\.sccache\.outcome == 'success'/u);
  assert.match(action, /if: steps\.sccache\.outcome != 'success'/u);
});
