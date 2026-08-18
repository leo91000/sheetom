import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

const workflow = await readFile(
  new URL("../.github/workflows/ci.yml", import.meta.url),
  "utf8",
);

function job(name: string): string {
  const marker = `  ${name}:\n`;
  const start = workflow.indexOf(marker);
  assert.notEqual(start, -1, `missing ${name} job`);
  const remainder = workflow.slice(start + marker.length);
  const next = remainder.search(/^  [a-z][a-z-]*:\n/mu);
  return next === -1
    ? workflow.slice(start)
    : workflow.slice(start, start + marker.length + next);
}

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

test("the package artifact no longer reruns the completed stable promotion", () => {
  const artifact = job("package-artifact");
  assert.doesNotMatch(artifact, /verify-stable-promotion/u);
  assert.doesNotMatch(artifact, /fetch-depth: 0/u);
});

test("superseded pull request and main push runs are cancelled", () => {
  assert.match(
    workflow,
    /cancel-in-progress: \$\{\{ github\.event_name == 'pull_request' \|\| github\.event_name == 'push' \}\}/u,
  );
});
