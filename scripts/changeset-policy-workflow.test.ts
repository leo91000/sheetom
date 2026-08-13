import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

const workflow = await readFile(
  new URL("../.github/workflows/ci.yml", import.meta.url),
  "utf8",
);

test("the generated Changesets release pull request needs no recursive Changeset", () => {
  assert.match(
    workflow,
    /github\.head_ref == 'changeset-release\/main'/u,
  );
  assert.match(
    workflow,
    /github\.ref_name == 'changeset-release\/main'/u,
  );
});
