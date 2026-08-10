import assert from "node:assert/strict";
import { test } from "vitest";

import { CSSGroupingRule, CSSStyleRule, CSSStyleSheet } from "../src/index.js";

test("deep rule trees hydrate and serialize without using the JavaScript call stack", () => {
  const depth = 2_048;
  const source = `${"@media all{".repeat(depth)}.leaf{color:red}${"}".repeat(depth)}`;
  const sheet = new CSSStyleSheet();
  sheet.replaceSync(source);

  const root = sheet.cssRules[0];
  assert.ok(root);
  let current = root;
  let observedDepth = 0;
  while (current instanceof CSSGroupingRule && !(current instanceof CSSStyleRule)) {
    observedDepth += 1;
    const child = current.cssRules[0];
    assert.ok(child);
    current = child;
  }
  assert.equal(observedDepth, depth);
  assert.ok(current instanceof CSSStyleRule);
  assert.equal(current.style.getPropertyValue("color"), "red");

  const cssText = root.cssText;
  assert.match(cssText, /\.leaf \{ color: red; \}/u);
  const serialized = sheet.serialize();
  assert.match(serialized, /\.leaf \{\n\s+color: red;\n\s+\}/u);

  const reparsed = new CSSStyleSheet();
  reparsed.replaceSync(serialized);
  assert.equal(reparsed.serialize(), serialized);
});
