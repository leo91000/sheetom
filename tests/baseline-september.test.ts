import assert from "node:assert/strict";
import { test } from "vitest";
import { CSS, CSSStyleSheet, CSSStyleDeclaration, CSSPositionTryRule, CSSPositionTryDescriptors } from "../src/index.js";

test("sibling functions retain element context across numeric dimensions", () => {
  for (const name of ["sibling-count", "sibling-index"]) {
    for (const [property, value] of [
      ["order", `${name}()`],
      ["opacity", `calc(1 / ${name}())`],
      ["width", `calc(100% / ${name}())`],
      ["animation-delay", `calc(0.1s * ${name}())`],
      ["transform", `rotate(calc(1deg * ${name}()))`],
      ["transform", `scale(${name}())`],
      ["animation", `fade calc(1s * ${name}()) ease`],
      ["margin", `calc(1px * ${name}()) 2px`],
    ] as const) {
      const sheet = new CSSStyleSheet();
      sheet.replaceSync(`.a { ${property}: ${value}; }`);
      assert.equal(CSS.supports(property, value), true, `${property}: ${value}`);
      const style = (sheet.cssRules[0] as import("../src/index.js").CSSStyleRule).style;
      assert.match(style.getPropertyValue(property), new RegExp(`${name}\\(\\)`));
      const before = style.cssText;
      style.setProperty(property, `${name}(1)`);
      assert.equal(style.cssText, before);
      const reparse = new CSSStyleSheet();
      reparse.replaceSync(sheet.serializeStrict());
      assert.equal(reparse.cssRules[0]!.cssText, sheet.cssRules[0]!.cssText);
    }
  }
  for (const [property, value] of [["width", "sibling-count()"], ["order", "sibling-index(1)"], ["order", "sibling-count(,)"], ["order", "calc(1px + sibling-index())"]] as const) {
    assert.equal(CSS.supports(property, value), false, `${property}: ${value}`);
  }
});

test("progress validates its argument types independently of the enclosing property", () => {
  const sheet = new CSSStyleSheet();
  sheet.replaceSync(".a {}");
  const style = (sheet.cssRules[0] as import("../src/index.js").CSSStyleRule).style;
  for (const value of ["progress(5, 0, 10)", "progress(50px, 0px, 100px)", "progress(500ms, 0s, 1s)", "progress(180deg, 0turn, 1turn)", "progress(50%, 0%, 100%)", "progress(1Hz, 0Hz, 2Hz)", "progress(1dppx, 0dpi, 192dpi)", "progress(1em, 0em, 2em)"]) {
    assert.equal(CSS.supports("opacity", value), true, value);
    style.setProperty("opacity", value);
    assert.equal(style.getPropertyValue("opacity"), "calc(0.5)");
  }
  const value = "calc(100px * progress(50vw, 0px, 1000px))";
  style.setProperty("width", value);
  assert.equal(style.getPropertyValue("width"), value);
  for (const invalid of ["progress()", "progress(1, 2)", "progress(1, 2, 3, 4)", "progress(1px, 0, 10px)", "progress(1s, 0px, 2s)", "progress(1%, 0px, 2%)"]) {
    assert.equal(CSS.supports("opacity", invalid), false, invalid);
    const before = style.cssText;
    style.setProperty("opacity", invalid);
    assert.equal(style.cssText, before);
  }
  assert.equal(CSS.supports("width", "progress(5, 0, 10)"), false);
});

test("Baseline media state selectors report authoring support", () => {
  for (const name of ["playing", "paused", "seeking", "buffering", "stalled", "muted", "volume-locked"]) {
    assert.equal(CSS.supports(`selector(video:${name})`), true);
    assert.equal(CSS.supports(`selector(:is(video:${name}))`), true);
    assert.equal(CSS.supports(`selector(video:${name}(x))`), false);
    const sheet = new CSSStyleSheet();
    sheet.replaceSync(`video:${name} { color: red; }`);
    assert.equal(sheet.cssRules.length, 1);
  }
});

test("position-try has a live descriptor interface and filters every mutation path", () => {
  assert.throws(() => Reflect.construct(CSSPositionTryDescriptors, []), TypeError);
  const sheet = new CSSStyleSheet();
  sheet.replaceSync("@position-try --fallback { top: 10px; width: 20px; color: red; display: block; --x: 1; height: 5px !important; }");
  const rule = sheet.cssRules[0] as CSSPositionTryRule;
  assert.ok(rule instanceof CSSPositionTryRule);
  assert.ok(rule.style instanceof CSSPositionTryDescriptors);
  assert.ok(rule.style instanceof CSSStyleDeclaration);
  const style = rule.style;
  assert.equal(style.parentRule, rule);
  assert.equal(style.cssText, "top: 10px; width: 20px;");
  style.setProperty("color", "blue");
  style.display = "grid";
  style.applyMutations([{ kind: "set", property: "--x", value: "2" }, { kind: "set", property: "margin", value: "1px 2px" }]);
  assert.equal(style.getPropertyValue("--x"), "");
  assert.equal(style.getPropertyValue("margin-left"), "2px");
  style.cssText = "inset: 3px; color: red; width: 5px !important; position-area: top;";
  assert.equal(rule.style, style);
  assert.equal(style.cssText, "inset: 3px; position-area: top;");
  style.setProperty("width", "6px", "important");
  assert.equal(style.getPropertyPriority("width"), "important");
  style.removeProperty("width");
  assert.equal(typeof Object.getOwnPropertyDescriptor(CSSPositionTryDescriptors.prototype, "marginTop")?.get, "function");
  assert.equal(Object.hasOwn(CSSPositionTryDescriptors.prototype, "color"), false);
  rule.style = "left: 4px; color: red;";
  assert.equal(rule.style, style);
  assert.equal(style.cssText, "left: 4px;");
  const copy = new CSSStyleSheet();
  copy.replaceSync(sheet.serializeStrict());
  assert.equal(copy.cssRules[0]!.cssText, rule.cssText);
  sheet.deleteRule(0);
  style.setProperty("left", "7px");
  assert.equal(style.getPropertyValue("left"), "7px");
  assert.equal(rule.parentStyleSheet, null);
});
