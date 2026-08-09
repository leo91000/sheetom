import assert from "node:assert/strict";
import { test } from "vitest";

import { CSSStyleRule, CSSStyleSheet } from "../src/index.js";
import { createStyleRule } from "./support/create-style-rule.js";

test("padding expands into indexed longhands and serializes opportunistically", () => {
  const sheet = new CSSStyleSheet();
  sheet.insertRule(".x {}");

  const rule = sheet.cssRules[0];
  assert.ok(rule instanceof CSSStyleRule);

  rule.style.setProperty("padding", "1px 2px");

  assert.equal(rule.style.length, 4);
  assert.deepEqual(
    Array.from({ length: rule.style.length }, (_, index) => rule.style.item(index)),
    ["padding-top", "padding-right", "padding-bottom", "padding-left"],
  );
  assert.equal(rule.style.getPropertyValue("padding"), "1px 2px");
  assert.equal(rule.style.cssText, "padding: 1px 2px;");

  rule.style.setProperty("padding-left", "3px");
  assert.equal(rule.style.getPropertyValue("padding"), "1px 2px 1px 3px");
  assert.equal(rule.style.cssText, "padding: 1px 2px 1px 3px;");

  assert.equal(rule.style.removeProperty("padding-left"), "3px");
  assert.equal(rule.style.getPropertyValue("padding"), "");
  assert.equal(
    rule.style.cssText,
    "padding-top: 1px; padding-right: 2px; padding-bottom: 1px;",
  );
});

test("common four-side shorthands share expanded record behavior", () => {
  const rule = createStyleRule(".x");
  rule.style.setProperty("margin", "1px 2px");

  assert.deepEqual(
    Array.from({ length: rule.style.length }, (_, index) => rule.style[index]),
    ["margin-top", "margin-right", "margin-bottom", "margin-left"],
  );
  assert.equal(rule.style.getPropertyValue("margin"), "1px 2px");
  assert.equal(rule.style.cssText, "margin: 1px 2px;");

  rule.style.setProperty("margin-left", "3px");
  assert.equal(rule.style.cssText, "margin: 1px 2px 1px 3px;");
});

test("removing a shorthand preserves neighboring declarations", () => {
  const rule = createStyleRule(".x");
  rule.style.setProperty("color", "red");
  rule.style.setProperty("padding", "1px 2px");
  rule.style.setProperty("width", "3px");

  assert.equal(rule.style.removeProperty("padding"), "1px 2px");
  assert.equal(rule.style.cssText, "color: red; width: 3px;");
  assert.deepEqual(
    Array.from({ length: rule.style.length }, (_, index) => rule.style[index]),
    ["color", "width"],
  );
});

test("complex static shorthands expand to browser longhand state", () => {
  const rule = createStyleRule(".x");
  rule.style.setProperty("border", "1px solid red");

  assert.equal(rule.style.length, 17);
  assert.equal(rule.style.item(0), "border-top-width");
  assert.equal(rule.style.getPropertyValue("border"), "1px solid red");
  assert.equal(rule.style.getPropertyValue("border-top-width"), "1px");
  assert.equal(rule.style.cssText, "border: 1px solid red;");

  rule.style.setProperty("border-left-color", "blue");
  assert.equal(rule.style.getPropertyValue("border"), "");
  assert.match(rule.style.cssText, /blue|#00f/);
});

test("removing an overridden background longhand cannot reactivate the shorthand", () => {
  const rule = createStyleRule(".x");
  rule.style.setProperty("background", "red");

  assert.equal(rule.style.length, 9);
  assert.equal(rule.style.getPropertyValue("background"), "red");
  assert.equal(rule.style.getPropertyValue("background-image"), "initial");
  assert.equal(rule.style.cssText, "background: red;");

  rule.style.setProperty("background-color", "blue");
  assert.equal(rule.style.getPropertyValue("background"), "blue");

  assert.equal(rule.style.removeProperty("background-color"), "blue");
  assert.equal(rule.style.getPropertyValue("background"), "");
  assert.equal(rule.style.getPropertyValue("background-color"), "");
  assert.equal(rule.style.length, 8);
  assert.equal(rule.style.cssText.includes("background: red"), false);
});

test("static shorthand families own only their expanded longhand state", () => {
  const cases = [
    ["overflow", "hidden", 2, "overflow-x", "scroll"],
    ["border-radius", "10px", 4, "border-top-left-radius", "20px"],
    ["font", "italic 16px serif", 19, "font-size", "20px"],
    ["animation", "1s linear foo", 11, "animation-duration", "2s"],
    ["transition", "color 1s linear", 5, "transition-duration", "2s"],
    ["container", "card / inline-size", 2, "container-name", "other"],
    ["white-space", "pre-wrap", 2, "white-space-collapse", "collapse"],
  ] as const;

  for (const [shorthand, value, length, longhand, override] of cases) {
    const rule = createStyleRule(".x");
    rule.style.setProperty(shorthand, value);

    assert.equal(rule.style.length, length, shorthand);
    assert.notEqual(rule.style.getPropertyValue(shorthand), "", shorthand);

    rule.style.setProperty(longhand, override);
    assert.equal(rule.style.removeProperty(longhand), override, shorthand);
    assert.equal(rule.style.getPropertyValue(shorthand), "", shorthand);
    assert.equal(rule.style.length, length - 1, shorthand);
    assert.equal(rule.style.cssText.includes(`${shorthand}:`), false, shorthand);
  }
});

test("animation shorthand getters include Chromium's observable default fields", () => {
  const rule = createStyleRule(".x");
  rule.style.setProperty("animation", "1s linear foo");

  assert.equal(
    rule.style.getPropertyValue("animation"),
    "1s linear 0s 1 normal none running foo",
  );
  assert.equal(
    rule.style.cssText,
    "animation: 1s linear 0s 1 normal none running foo;",
  );
});

test("static shorthand mutations preserve order and mixed priorities", () => {
  const rule = createStyleRule(".x");
  rule.style.setProperty("overflow", "hidden");
  rule.style.setProperty("overflow-x", "scroll", "important");

  assert.equal(rule.style.getPropertyValue("overflow"), "");
  assert.equal(rule.style.getPropertyPriority("overflow"), "");
  assert.equal(
    rule.style.cssText,
    "overflow-x: scroll !important; overflow-y: hidden;",
  );

  rule.style.setProperty("overflow", "auto", "important");
  assert.equal(rule.style.getPropertyValue("overflow"), "auto");
  assert.equal(rule.style.getPropertyPriority("overflow"), "important");
  assert.deepEqual([rule.style.item(0), rule.style.item(1)], ["overflow-x", "overflow-y"]);
});

test("cssText replacement uses the same shorthand state model", () => {
  const rule = createStyleRule(".x");
  rule.style.cssText = "overflow: hidden; color: red";
  rule.style.setProperty("overflow-x", "scroll");
  rule.style.removeProperty("overflow-x");

  assert.equal(rule.style.getPropertyValue("overflow"), "");
  assert.equal(rule.style.getPropertyValue("overflow-y"), "hidden");
  assert.equal(rule.style.cssText, "overflow-y: hidden; color: red;");
});

test("representative static shorthands never become standalone records", () => {
  const cases = [
    ["flex", "1 1 auto"],
    ["gap", "1px 2px"],
    ["grid-column", "1 / 3"],
    ["list-style", "inside square"],
    ["outline", "1px solid red"],
    ["text-decoration", "underline solid red"],
  ] as const;

  for (const [shorthand, value] of cases) {
    const rule = createStyleRule(".x");
    rule.style.setProperty(shorthand, value);
    assert.ok(rule.style.length > 1, shorthand);
    assert.equal(
      Array.from({ length: rule.style.length }, (_, index) => rule.style.item(index))
        .includes(shorthand),
      false,
      shorthand,
    );
  }
});

test("an unexpanded shorthand never creates parallel shorthand state", () => {
  const rule = createStyleRule(".x");
  rule.style.setProperty("mask", "none");

  assert.equal(
    Array.from({ length: rule.style.length }, (_, index) => rule.style.item(index))
      .includes("mask"),
    false,
  );
});
