import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

const workflow = await readFile(
  new URL("../.github/workflows/release.yml", import.meta.url),
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

test("the release pull request receives an exact-SHA pending status", () => {
  const evidence = job("record-evidence");
  assert.match(evidence, /statuses: write/u);
  assert.match(evidence, /head_sha=\$head_sha/u);
  assert.match(evidence, /context=sheetom\/release-validation/u);
  assert.match(evidence, /state=pending/u);
  assert.match(evidence, /\.headSha == \$head_sha/u);
});

test("release validation waits for CI and publishes its terminal result", () => {
  const validation = job("release-validation");
  assert.match(validation, /gh run watch "\$RUN_ID"/u);
  assert.match(
    validation,
    /gh run view "\$RUN_ID" --repo "\$GITHUB_REPOSITORY" --json conclusion/u,
  );
  assert.match(validation, /state=success/u);
  assert.match(validation, /state=failure/u);
  assert.match(validation, /context=sheetom\/release-validation/u);
});

test("stable publication has no calendar-duration soak prerequisite", () => {
  const publish = job("publish");
  assert.match(publish, /needs: changesets/u);
  assert.doesNotMatch(workflow, /^  soak-gate:/mu);
  assert.doesNotMatch(workflow, /verify-release-soak/u);
});
