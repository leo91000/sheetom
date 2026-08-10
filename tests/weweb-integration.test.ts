import assert from "node:assert/strict";
import { test } from "vitest";

import {
  CSSKeyframesRule,
  CSSLayerBlockRule,
  CSSMediaRule,
  CSSPropertyRule,
  CSSStyleRule,
  parseStyleSheet,
} from "../src/index.js";

const weWebStyleSheet = `
  @layer ww-style-reset, ww-style-library, ww-style-section, ww-style-element;
  @property --dynamic-width {
    syntax: "<length>";
    inherits: false;
    initial-value: 0px;
  }
  @layer ww-style-element {
    .ww-element-a { color: red; }
    @media (max-width: 767px) {
      .ww-element-a:hover { width: 100px; }
    }
  }
  @keyframes animation-a {
    from { opacity: 0; }
    to { opacity: 1; }
  }
`;

test("a WeWeb authoring tree remains live, detachable, and reparsable", () => {
  const sheet = parseStyleSheet(weWebStyleSheet);
  assert.equal(sheet.cssRules.length, 4);
  assert.equal(
    sheet.cssRules[0]?.cssText,
    "@layer ww-style-reset, ww-style-library, ww-style-section, ww-style-element;",
  );
  const property = sheet.cssRules[1];
  assert.equal(
    property?.cssText,
    "@property --dynamic-width { syntax: \"<length>\"; inherits: false; initial-value: 0px; }",
  );
  assert.ok(property instanceof CSSPropertyRule);
  assert.equal(property.name, "--dynamic-width");
  assert.equal(property.syntax, "<length>");
  assert.equal(property.inherits, false);
  assert.equal(property.initialValue, "0px");

  const layer = sheet.cssRules[2];
  const keyframes = sheet.cssRules[3];
  assert.ok(layer instanceof CSSLayerBlockRule);
  assert.ok(keyframes instanceof CSSKeyframesRule);

  const styleRule = layer.cssRules[0];
  const media = layer.cssRules[1];
  assert.ok(styleRule instanceof CSSStyleRule);
  assert.ok(media instanceof CSSMediaRule);
  const hoverRule = media.cssRules[0];
  assert.ok(hoverRule instanceof CSSStyleRule);

  styleRule.style.setProperty("padding", "72px var(--space, var(--space,");
  assert.equal(
    styleRule.style.getPropertyValue("padding"),
    "72px var(--space, var(--space,",
  );

  const serialized = sheet.serialize();
  assert.match(serialized, /@media \(max-width: 767px\)/);
  assert.match(serialized, /padding: 72px var\(--space, var\(--space, \)\)/);
  assert.equal(parseStyleSheet(serialized).serialize(), serialized);

  const layerRules = layer.cssRules;
  assert.equal(layer.insertRule(".inserted {}", 1), 1);
  const inserted = layer.cssRules[1];
  assert.ok(inserted instanceof CSSStyleRule);
  layer.deleteRule(1);
  assert.equal(layer.cssRules, layerRules);
  assert.equal(inserted.parentRule, null);
  assert.equal(inserted.parentStyleSheet, null);

  sheet.deleteRule(2);
  assert.equal(layer.parentRule, null);
  assert.equal(layer.parentStyleSheet, null);
  assert.equal(styleRule.parentRule, layer);
  assert.equal(styleRule.parentStyleSheet, null);
  assert.equal(media.parentRule, layer);
  assert.equal(media.parentStyleSheet, null);
  assert.equal(hoverRule.parentRule, media);
  assert.equal(hoverRule.parentStyleSheet, null);
});
