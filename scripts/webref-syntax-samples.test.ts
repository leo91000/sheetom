import assert from "node:assert/strict";
import test from "node:test";

import { generateWebrefSyntaxSamples } from "./lib/webref-syntax-samples.ts";

test("the Content Module legacy symbolic counter style name uses Webref's glyph definition", () => {
  const { samples, issues } = generateWebrefSyntaxSamples({
    definitions: {
      properties: {},
      types: {
        "symbolic-glyph": {
          syntax: "disc | circle | square | disclosure-open | disclosure-closed",
        },
      },
      functions: {},
    },
    property: "content",
    syntax: "<symbolic-counter-style>",
  });

  assert.deepEqual(issues, []);
  assert.deepEqual(
    new Set(samples.map(sample => sample.value)),
    new Set(["disc", "circle", "square", "disclosure-open", "disclosure-closed"]),
  );
});
