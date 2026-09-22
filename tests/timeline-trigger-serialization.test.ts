import assert from "node:assert/strict";
import { test } from "vitest";

import { CSSStyleRule, CSSStyleSheet } from "../src/index.js";

test("timeline-trigger omits none sources while preserving longhands and safe serialization", () => {
  for (const [input, expected, source] of [
    ["none none normal normal / auto auto", "none", "none"],
    ["--trigger none normal normal / auto auto", "--trigger", "none"],
    ["none none cover 10% normal / auto auto", "cover 10%", "none"],
    ["none none normal normal / 10% auto", " / 10%", "none"],
    ["--first none, --second none", "--first, --second", "none, none"],
    ["--trigger auto", "--trigger", "auto"],
    ["--trigger scroll()", "--trigger scroll()", "scroll()"],
  ] as const) {
    const sheet = new CSSStyleSheet();
    sheet.insertRule(".trigger {}");
    const rule = sheet.cssRules[0];
    assert.ok(rule instanceof CSSStyleRule);
    rule.style.setProperty("timeline-trigger", input, "important");
    assert.equal(rule.style.getPropertyValue("timeline-trigger"), expected, input);
    assert.equal(rule.style.cssText, `timeline-trigger: ${expected} !important;`, input);
    assert.equal(rule.style.getPropertyValue("timeline-trigger-source"), source, input);

    const serialized = sheet.serialize();
    const reparsed = new CSSStyleSheet();
    reparsed.replaceSync(serialized);
    assert.equal(reparsed.serialize(), serialized, input);
    const reparsedRule = reparsed.cssRules[0];
    assert.ok(reparsedRule instanceof CSSStyleRule);
    assert.equal(reparsedRule.style.getPropertyValue("timeline-trigger-source"), source, input);

    const before = rule.style.cssText;
    rule.style.setProperty("timeline-trigger", `${input} __invalid__`);
    assert.equal(rule.style.cssText, before, input);
  }
});
