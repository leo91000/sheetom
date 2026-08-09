import assert from "node:assert/strict";
import { test } from "vitest";

import { CSSStyleRule } from "../src/index.js";

test("current Chromium attr() and if() substitutions pass the value gate", () => {
  const rule = new CSSStyleRule(".card");

  const attrValue = "attr(data-width type(<length>), 1px)";
  rule.style.setProperty("width", attrValue);
  assert.equal(rule.style.getPropertyValue("width"), attrValue);

  const ifValue = "if(style(--theme: dark): white; else: black)";
  rule.style.setProperty("color", ifValue);
  assert.equal(rule.style.getPropertyValue("color"), ifValue);

  rule.style.setProperty("width", "attr()");
  rule.style.setProperty("color", "if()");
  assert.equal(rule.style.getPropertyValue("width"), attrValue);
  assert.equal(rule.style.getPropertyValue("color"), ifValue);
});
