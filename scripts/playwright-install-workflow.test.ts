import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

const action = await readFile(
  new URL("../.github/actions/install-playwright/action.yml", import.meta.url),
  "utf8",
);
const workflows = await Promise.all(
  ["ci.yml", "native-oracles.yml", "release.yml"].map(async (name) => ({
    name,
    source: await readFile(
      new URL(`../.github/workflows/${name}`, import.meta.url),
      "utf8",
    ),
  })),
);

test("Playwright installs use a resilient Ubuntu package source", () => {
  assert.match(action, /\/etc\/apt\/apt-mirrors\.txt/u);
  assert.match(action, /https:\/\/archive\.ubuntu\.com\/ubuntu/u);
  assert.match(action, /Acquire::Retries "5";/u);
  assert.match(action, /Acquire::https::Timeout "30";/u);
  assert.match(
    action,
    /playwright install --with-deps chromium firefox webkit/u,
  );
});

test("release-critical workflows share the Playwright installer", () => {
  const expectedUses = new Map([
    ["ci.yml", 2],
    ["native-oracles.yml", 1],
    ["release.yml", 1],
  ]);

  for (const workflow of workflows) {
    const uses = workflow.source.match(
      /uses: \.\/\.github\/actions\/install-playwright/gu,
    );
    assert.equal(uses?.length, expectedUses.get(workflow.name), workflow.name);
    assert.doesNotMatch(workflow.source, /playwright install --with-deps/u);
  }
});
