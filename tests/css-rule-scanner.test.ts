import assert from "node:assert/strict";
import { test } from "vitest";

import { scanTopLevelRules } from "../src/internal/css-rule-scanner.js";

test("the CSS Syntax scanner keeps exact top-level rule spans", () => {
  assert.deepEqual(
    scanTopLevelRules(
      ' /* lead */ @unknown fn(a; b) { value: "}"; nested: fn({x}); } .ok { color: red; } @tail x;',
    ),
    [
      '@unknown fn(a; b) { value: "}"; nested: fn({x}); }',
      ".ok { color: red; }",
      "@tail x;",
    ],
  );
});

test("the scanner retains an EOF-recovered final rule", () => {
  assert.deepEqual(
    scanTopLevelRules(".first {} .recovered { color: var(--x,"),
    [".first {}", ".recovered { color: var(--x,"],
  );
});
