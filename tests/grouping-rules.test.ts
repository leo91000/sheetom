import assert from "node:assert/strict";
import { test } from "vitest";

import {
  CSSConditionRule,
  CSSContainerRule,
  CSSGroupingRule,
  CSSLayerBlockRule,
  CSSMarginRule,
  CSSMediaRule,
  CSSNestedDeclarations,
  CSSPageRule,
  CSSScopeRule,
  CSSStartingStyleRule,
  CSSStyleRule,
  CSSSupportsRule,
  MediaList,
  parseStyleSheet,
} from "../src/index.js";

test("media rules expose a live nested rule tree", () => {
  const sheet = parseStyleSheet("@media screen { .a { color: red; } }");
  const media = sheet.cssRules[0];

  assert.ok(media instanceof CSSMediaRule);
  assert.ok(media instanceof CSSConditionRule);
  assert.ok(media instanceof CSSGroupingRule);
  assert.ok(media.media instanceof MediaList);
  assert.equal(media.conditionText, "screen");
  assert.equal(media.media.mediaText, "screen");
  assert.equal(media.cssRules.length, 1);

  const child = media.cssRules[0];
  assert.ok(child instanceof CSSStyleRule);
  assert.equal(child.parentRule, media);
  assert.equal(child.parentStyleSheet, sheet);

  assert.equal(media.insertRule(".b {}", 1), 1);
  assert.equal(media.cssRules[1]?.cssText, ".b { }");

  media.deleteRule(0);
  assert.equal(child.parentRule, null);
  assert.equal(child.parentStyleSheet, null);
  assert.equal(
    media.cssText,
    "@media screen {\n  .b { }\n}",
  );
});

test("condition and structural grouping rules expose specialized interfaces", () => {
  const sheet = parseStyleSheet(`
    @supports (display: grid) { .supports { display: grid; } }
    @container card (width > 1px) { .container { color: red; } }
    @layer theme { .layer { color: red; } }
    @scope (.start) to (.end) { .scope { color: red; } }
    @starting-style { .starting { opacity: 0; } }
  `);

  const supports = sheet.cssRules[0];
  assert.ok(supports instanceof CSSSupportsRule);
  assert.equal(supports.conditionText, "(display: grid)");

  const container = sheet.cssRules[1];
  assert.ok(container instanceof CSSContainerRule);
  assert.equal(container.containerName, "card");
  assert.equal(container.containerQuery, "(width > 1px)");

  const layer = sheet.cssRules[2];
  assert.ok(layer instanceof CSSLayerBlockRule);
  assert.equal(layer.name, "theme");

  const scope = sheet.cssRules[3];
  assert.ok(scope instanceof CSSScopeRule);
  assert.equal(scope.start, ".start");
  assert.equal(scope.end, ".end");

  assert.ok(sheet.cssRules[4] instanceof CSSStartingStyleRule);
});

test("style rules expose live nested rules with Chromium insertion behavior", () => {
  const sheet = parseStyleSheet(
    ".outer { color: red; & .inner { color: blue; } }",
  );
  const outer = sheet.cssRules[0];
  assert.ok(outer instanceof CSSStyleRule);
  assert.equal(outer.style.cssText, "color: red;");
  assert.equal(outer.cssRules.length, 1);
  assert.equal(outer.cssRules[0]?.parentRule, outer);

  assert.equal(outer.insertRule(".inserted {}"), 0);
  const inserted = outer.cssRules[0];
  assert.ok(inserted instanceof CSSStyleRule);
  assert.equal(inserted.selectorText, "& .inserted");
  assert.equal(inserted.parentStyleSheet, sheet);
});

test("declarations after nested rules retain ordered nested-declaration identity", () => {
  const sheet = parseStyleSheet(
    ".outer { color: red; & .inner { color: blue; } width: 1px; height: 2px; }",
  );
  const outer = sheet.cssRules[0];
  assert.ok(outer instanceof CSSStyleRule);
  assert.equal(outer.style.cssText, "color: red;");
  assert.equal(outer.cssRules.length, 2);

  const trailing = outer.cssRules[1];
  assert.ok(trailing instanceof CSSNestedDeclarations);
  assert.equal(trailing.style.cssText, "width: 1px; height: 2px;");
  assert.equal(trailing.parentRule, outer);
  const style = trailing.style;
  trailing.style = "inset: 3px";
  assert.equal(trailing.style, style);
  assert.equal(trailing.style.cssText, "inset: 3px;");
  assert.equal(
    outer.cssText,
    ".outer {\n  color: red;\n  & .inner { color: blue; }\n  inset: 3px;\n}",
  );
});

test("group insertion enforces import and page child-rule hierarchies", () => {
  const sheet = parseStyleSheet("@media screen {} @page {}");
  const media = sheet.cssRules[0];
  const page = sheet.cssRules[1];
  assert.ok(media instanceof CSSMediaRule);
  assert.ok(page instanceof CSSPageRule);

  assert.throws(
    () => media.insertRule('@import url("theme.css");'),
    error => error instanceof DOMException && error.name === "HierarchyRequestError",
  );
  assert.throws(
    () => page.insertRule(".invalid {}"),
    error => error instanceof DOMException && error.name === "SyntaxError",
  );
  assert.equal(page.insertRule("@top-left {}"), 0);
  assert.ok(page.cssRules[0] instanceof CSSMarginRule);
});
