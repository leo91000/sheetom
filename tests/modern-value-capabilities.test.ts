import assert from "node:assert/strict";
import { test } from "vitest";

import { createStyleRule } from "./support/create-style-rule.js";
import valueCapabilities from "../compatibility/value-capabilities.json" with { type: "json" };

test("measured modern value families are not dropped by parser fallbacks", () => {
  const style = createStyleRule(".x").style;
  for (const candidate of valueCapabilities.cases) {
    if (!candidate.accepted) continue;
    style.setProperty(candidate.property, candidate.input);
    assert.equal(
      style.getPropertyValue(candidate.property),
      candidate.observable,
      candidate.id,
    );
  }
});

test("neighboring invalid capability cases are atomic no-ops", () => {
  const style = createStyleRule(".x").style;
  for (const candidate of valueCapabilities.cases) {
    if (candidate.accepted) continue;
    style.setProperty(candidate.property, "initial");
    style.setProperty(candidate.property, candidate.input);
    assert.equal(style.getPropertyValue(candidate.property), "initial", candidate.id);
  }
});

test("content recovery and feature support follow the Chromium baseline", () => {
  const style = createStyleRule(".x").style;

  style.setProperty("content", "var(--x");
  assert.equal(style.getPropertyValue("content"), "var(--x");

  style.setProperty("content", '"safe"');
  style.setProperty("content", "leader(.)");
  assert.equal(style.getPropertyValue("content"), '"safe"');

  style.setProperty("content", "target-text(url(#x))");
  assert.equal(style.getPropertyValue("content"), '"safe"');

  style.setProperty("content", "target-text(attr(href url))");
  assert.equal(style.getPropertyValue("content"), "target-text(attr(href url))");
});
