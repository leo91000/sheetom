import assert from "node:assert/strict";
import { test } from "vitest";
import * as api from "../src/index.js";
import { assertAuthoringRoundTrip, snapshotAuthoringRule } from "../scripts/css-authoring-roundtrip.ts";

// The browser authoring corpus compares these writes and snapshots independently.
test("all reset writes replace affected declarations only after successful validation", () => {
  for (const keyword of ["initial", "inherit", "unset", "revert", "revert-layer", "revert-rule"]) {
    for (const priority of ["", "important"]) {
      const sheet = new api.CSSStyleSheet();
      sheet.replaceSync(".reset { color: blue !important; margin: 1px; direction: rtl; unicode-bidi: isolate; --theme: blue; }");
      const rule = sheet.cssRules[0];
      assert.ok(rule instanceof api.CSSStyleRule);
      const style = rule.style;
      const before = style.cssText;
      style.setProperty("all", `${keyword} extra`, priority);
      assert.equal(style.cssText, before);
      style.setProperty("all", keyword, "invalid");
      assert.equal(style.cssText, before);
      style.setProperty("ALL", keyword, priority);
      assert.deepEqual(Array.from(style), ["direction", "unicode-bidi", "--theme", "all"]);
      assert.equal(style.getPropertyValue("all"), keyword);
      assert.equal(style.getPropertyPriority("all"), priority);
      for (const property of ["color", "width", "margin"]) {
        assert.equal(style.getPropertyValue(property), keyword);
        assert.equal(style.getPropertyPriority(property), priority);
      }
      assert.equal(style.getPropertyValue("direction"), "rtl");
      assert.equal(style.getPropertyValue("unicode-bidi"), "isolate");
      assert.equal(style.getPropertyValue("--theme"), "blue");
      const reparsed = new api.CSSStyleSheet();
      reparsed.replaceSync(sheet.serializeStrict());
      assertAuthoringRoundTrip(api, sheet, reparsed);
    }
  }
});

test("mask serialization retains omitted images, position keywords and layer state", () => {
  for (const [property, values] of [
    ["mask", ["add", "alpha", "left", "right", "repeat-x", "none", "none alpha", "alpha, none repeat-x", "url(https://example.com/mask.png) left, alpha"]],
    ["mask-position", ["left", "right", "left center, right bottom"]],
  ] as const) for (const value of values) {
    const sheet = new api.CSSStyleSheet();
    sheet.replaceSync(`.mask { ${property}: ${value} !important; }`);
    const rule = sheet.cssRules[0];
    assert.ok(rule instanceof api.CSSStyleRule);
    const before = rule.style.cssText;
    rule.style.setProperty(property, `${value}; color: red`);
    assert.equal(rule.style.cssText, before);
    const serialized = sheet.serializeStrict();
    const reparsed = new api.CSSStyleSheet();
    reparsed.replaceSync(serialized);
    // Omitted layers expose an initial marker that is not valid as part of
    // an independently authored image list. Compare exact state here instead
    // of feeding that contextual marker through scalar normalization.
    const snapshot = (rule: api.CSSRule | undefined) => { assert.ok(rule); return snapshotAuthoringRule(rule); };
    assert.deepEqual(Array.from(reparsed.cssRules, snapshot), Array.from(sheet.cssRules, snapshot));
    assert.equal(reparsed.serializeStrict(), serialized);
  }
});
