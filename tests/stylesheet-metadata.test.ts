import assert from "node:assert/strict";
import { test } from "vitest";

import {
  CSSKeyframeRule,
  CSSKeyframesRule,
  CSSMarginRule,
  CSSPageRule,
  CSSRule,
  CSSStyleSheet,
  MediaList,
  parseStyleSheet,
} from "../src/index.js";

test("stylesheets expose browser-shaped authoring metadata", () => {
  const sheet = new CSSStyleSheet({ media: "screen, print", disabled: true });

  assert.equal(sheet.type, "text/css");
  assert.equal(sheet.href, null);
  assert.equal(sheet.ownerNode, null);
  assert.equal(sheet.parentStyleSheet, null);
  assert.equal(sheet.ownerRule, null);
  assert.equal(sheet.title, null);
  assert.equal(sheet.disabled, true);
  assert.ok(sheet.media instanceof MediaList);
  assert.equal(sheet.media.mediaText, "screen, print");
});

test("specialized nested rules expose standard CSSRule type constants", () => {
  const sheet = parseStyleSheet(
    '@page { @top-left { content: "x"; } } @keyframes x { from { opacity: 0; } }',
  );
  const page = sheet.cssRules[0];
  assert.ok(page instanceof CSSPageRule);
  const margin = page.cssRules.item(0);
  assert.ok(margin instanceof CSSMarginRule);
  assert.equal(margin.type, CSSRule.MARGIN_RULE);

  const keyframes = sheet.cssRules[1];
  assert.ok(keyframes instanceof CSSKeyframesRule);
  const frame = keyframes.cssRules.item(0);
  assert.ok(frame instanceof CSSKeyframeRule);
  assert.equal(frame.type, CSSRule.KEYFRAME_RULE);
});
