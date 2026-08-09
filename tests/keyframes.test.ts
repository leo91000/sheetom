import assert from "node:assert/strict";
import { test } from "vitest";

import {
  CSSKeyframeRule,
  CSSKeyframesRule,
  parseStyleSheet,
} from "../src/index.js";

test("keyframes expose live frame rules and mutation methods", () => {
  const sheet = parseStyleSheet(
    "@keyframes spin { from { opacity: 0; } 50%, to { opacity: 1; } }",
  );
  const keyframes = sheet.cssRules[0];
  assert.ok(keyframes instanceof CSSKeyframesRule);
  assert.equal(keyframes.name, "spin");
  assert.equal(keyframes.length, 2);

  const from = keyframes.cssRules[0];
  assert.ok(from instanceof CSSKeyframeRule);
  assert.equal(from.keyText, "0%");
  assert.equal(from.parentRule, keyframes);
  assert.equal(keyframes.findRule("from"), from);

  assert.equal(keyframes.appendRule("25% { opacity: .25; }"), undefined);
  assert.equal(keyframes.findRule("25%")?.style.cssText, "opacity: 0.25;");

  keyframes.deleteRule("from");
  assert.equal(from.parentRule, null);
  assert.equal(from.parentStyleSheet, null);
});
