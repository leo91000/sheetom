import assert from "node:assert/strict";
import { test } from "vitest";
import * as api from "../src/index.js";
import { assertAuthoringRoundTrip } from "../scripts/css-authoring-roundtrip.ts";

// The November browser probes independently compare these grammars and state.
test("scroll-driven animation declarations preserve authored state and reject invalid writes", () => {
  const cases = [
    ["animation-duration", "auto, 2s", "auto extra"],
    ["animation-timeline", "scroll(root inline), view(y calc(1px + 5%) auto)", "scroll(root nearest)"],
    ["animation-range", "entry 10% exit 90%", "entry 1s"],
    ["animation-range-start", "cover 10%, contain 20%", "normal 10%"],
    ["animation-range-end", "cover 90%, contain 80%", "normal 90%"],
    ["scroll-timeline", "--progress inline, --other x", "--progress x y"],
    ["scroll-timeline-name", "--progress, --other", "progress"],
    ["scroll-timeline-axis", "block, inline, x, y", "block inline"],
    ["view-timeline", "--visible y auto 10%, --other inline 20%", "--visible x y"],
    ["view-timeline-name", "--visible, --other", "visible"],
    ["view-timeline-axis", "block, inline, x, y", "block inline"],
    ["view-timeline-inset", "calc(1px + 5%) auto", "auto auto auto"],
    ["timeline-scope", "--progress, --other", "none --progress"],
  ] as const;
  for (const [property, value, invalid] of cases) {
    assert.equal(api.CSS.supports(property, value), true, `${property}: ${value}`);
    assert.equal(api.CSS.supports(property, invalid), false, `${property}: ${invalid}`);
    const sheet = new api.CSSStyleSheet();
    sheet.replaceSync(`.animation { color: blue; ${property}: ${value} !important; }`);
    const rule = sheet.cssRules[0];
    assert.ok(rule instanceof api.CSSStyleRule);
    assert.notEqual(rule.style.getPropertyValue(property), "", property);
    assert.equal(rule.style.getPropertyPriority(property), "important", property);
    const before = rule.style.cssText;
    rule.style.setProperty(property, invalid);
    assert.equal(rule.style.cssText, before, `${property}: invalid grammar is atomic`);
    rule.style.setProperty(property, value, "invalid");
    assert.equal(rule.style.cssText, before, `${property}: invalid priority is atomic`);
    const reparsed = new api.CSSStyleSheet();
    reparsed.replaceSync(sheet.serializeStrict());
    assertAuthoringRoundTrip(api, sheet, reparsed);
    assert.equal(reparsed.serializeStrict(), sheet.serializeStrict(), property);
    assert.notEqual(rule.style.removeProperty(property), "", property);
    assert.equal(rule.style.getPropertyValue(property), "", property);
    assert.equal(rule.style.getPropertyValue("color"), "blue");
  }
});

test("timeline shorthands expose their named longhands and animation resets timeline state", () => {
  const sheet = new api.CSSStyleSheet();
  sheet.replaceSync(".animation { scroll-timeline: --progress inline; view-timeline: --visible y auto 10%; animation-range: entry 10% exit 90%; animation-timeline: --progress; }");
  const rule = sheet.cssRules[0];
  assert.ok(rule instanceof api.CSSStyleRule);
  for (const [property, expected] of [
    ["scroll-timeline-name", "--progress"], ["scroll-timeline-axis", "inline"],
    ["view-timeline-name", "--visible"], ["view-timeline-axis", "y"],
    ["view-timeline-inset", "auto 10%"],
    ["animation-range-start", "entry 10%"], ["animation-range-end", "exit 90%"],
  ] as const) assert.equal(rule.style.getPropertyValue(property), expected, property);
  const initialReparse = new api.CSSStyleSheet();
  initialReparse.replaceSync(sheet.serializeStrict());
  assertAuthoringRoundTrip(api, sheet, initialReparse);
  rule.style.setProperty("animation", "2s ease slide");
  assert.equal(rule.style.getPropertyValue("animation-timeline"), "auto");
  assert.equal(rule.style.getPropertyValue("animation-range-start"), "normal");
  assert.equal(rule.style.getPropertyValue("animation-range-end"), "normal");
  assert.equal(rule.style.getPropertyValue("scroll-timeline-name"), "--progress");
  const reparsed = new api.CSSStyleSheet();
  reparsed.replaceSync(sheet.serializeStrict());
  const reparsedRule = reparsed.cssRules[0];
  assert.ok(reparsedRule instanceof api.CSSStyleRule);
  assert.equal(reparsed.cssRules.length, 1);
  assert.equal(reparsedRule.selectorText, rule.selectorText);
  // Chromium also moves reset-only longhands to the shorthand's expansion
  // order on reparse. Preserve the complete name/value/priority set here;
  // the independent declaration round trips above still check exact order.
  const state = (style: api.CSSStyleDeclaration) => Array.from(style, name => {
    assert.ok(name);
    return [name, style.getPropertyValue(name), style.getPropertyPriority(name)] as const;
  }).sort(([a], [b]) => a.localeCompare(b));
  assert.deepEqual(state(reparsedRule.style), state(rule.style));
  assert.equal(reparsed.serializeStrict(), sheet.serializeStrict());
});

test("scroll-timeline requires a name before the optional axis in every list entry", () => {
  for (const value of ["inline --progress", "x none", "--valid y, inline --progress"]) {
    assert.equal(api.CSS.supports("scroll-timeline", value), false, value);
    assert.equal(api.CSS.supports(`(scroll-timeline: ${value})`), false, value);
    const sheet = new api.CSSStyleSheet();
    sheet.replaceSync(`.invalid { color: blue; scroll-timeline: ${value}; }`);
    const rule = sheet.cssRules[0];
    assert.ok(rule instanceof api.CSSStyleRule);
    assert.equal(rule.style.cssText, "color: blue;", value);
    rule.style.setProperty("scroll-timeline", "--progress inline", "important");
    const before = rule.style.cssText;
    const names = Array.from(rule.style);
    rule.style.setProperty("scroll-timeline", value);
    assert.equal(rule.style.cssText, before, value);
    assert.deepEqual(Array.from(rule.style), names, value);
    assert.equal(rule.style.getPropertyValue("scroll-timeline-name"), "--progress");
    assert.equal(rule.style.getPropertyValue("scroll-timeline-axis"), "inline");
    assert.equal(rule.style.getPropertyPriority("scroll-timeline"), "important");
    rule.style.scrollTimeline = value;
    assert.equal(rule.style.cssText, before, value);
    rule.style.cssText = `color: blue; scroll-timeline: ${value};`;
    assert.equal(rule.style.cssText, "color: blue;", value);
    sheet.insertRule(`.inserted { color: blue; scroll-timeline: ${value}; }`, 1);
    const inserted = sheet.cssRules[1];
    assert.ok(inserted instanceof api.CSSStyleRule);
    assert.equal(inserted.style.cssText, "color: blue;", value);
  }
  for (const [value, names, axes] of [
    ["--progress", "--progress", "block"],
    ["--progress inline", "--progress", "inline"],
    ["none x, --progress y", "none, --progress", "x, y"],
    ["--progress inline, --other", "--progress, --other", "inline, block"],
  ] as const) {
    assert.equal(api.CSS.supports("scroll-timeline", value), true, value);
    const sheet = new api.CSSStyleSheet();
    sheet.replaceSync(`.valid { scroll-timeline: ${value} !important; }`);
    const rule = sheet.cssRules[0];
    assert.ok(rule instanceof api.CSSStyleRule);
    assert.equal(rule.style.getPropertyValue("scroll-timeline-name"), names);
    assert.equal(rule.style.getPropertyValue("scroll-timeline-axis"), axes);
    const reparsed = new api.CSSStyleSheet();
    reparsed.replaceSync(sheet.serializeStrict());
    assertAuthoringRoundTrip(api, sheet, reparsed);
  }
});
