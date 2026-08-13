import assert from "node:assert/strict";
import { test } from "vitest";

import { CSSStyleRule, CSSStyleSheet } from "../src/index.js";
import { chromiumShorthandLonghands } from "../src/chromium-properties.js";
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

test("reparsable serialization preserves independent substitution longhands", () => {
  const sheet = new CSSStyleSheet();
  sheet.insertRule(".x {}");
  const rule = sheet.cssRules[0];
  assert.ok(rule instanceof CSSStyleRule);

  rule.style.setProperty("align-content", "var(--align, revert-layer)");
  rule.style.setProperty("justify-content", "var(--justify, revert-layer)");

  assert.equal(
    sheet.serialize(),
    ".x {\n  align-content: var(--align, revert-layer);\n  justify-content: var(--justify, revert-layer);\n}\n",
  );
});

test("reparsable serialization retains an authored pending shorthand", () => {
  const sheet = new CSSStyleSheet();
  sheet.insertRule(".x {}");
  const rule = sheet.cssRules[0];
  assert.ok(rule instanceof CSSStyleRule);

  rule.style.setProperty(
    "place-content",
    "var(--align, revert-layer) var(--justify, revert-layer)",
  );

  assert.equal(
    sheet.serialize(),
    ".x {\n  place-content: var(--align, revert-layer) var(--justify, revert-layer);\n}\n",
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

test("removed layered background longhands leave valid concrete defaults", () => {
  const sheet = new CSSStyleSheet();
  sheet.insertRule(".x {}");
  const rule = sheet.cssRules[0];
  assert.ok(rule instanceof CSSStyleRule);

  rule.style.setProperty(
    "background",
    "linear-gradient(red, blue) center / cover no-repeat, green",
  );
  rule.style.setProperty("background-color", "blue");
  rule.style.removeProperty("background-color");

  assert.equal(
    rule.style.getPropertyValue("background-image"),
    "linear-gradient(red, #00f), initial",
  );
  assert.equal(rule.style.getPropertyValue("background-position-x"), "center, initial");
  assert.doesNotMatch(sheet.serialize(), /, initial/);
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

test("every manifested multi-longhand shorthand expands atomically", () => {
  for (const [shorthand, longhands] of Object.entries(chromiumShorthandLonghands)) {
    if (longhands.length < 2) continue;
    const rule = createStyleRule(".x");
    rule.style.setProperty(shorthand, "initial");

    assert.equal(rule.style.length, longhands.length, shorthand);
    assert.deepEqual(
      Array.from({ length: rule.style.length }, (_, index) => rule.style.item(index)),
      longhands,
      shorthand,
    );
    assert.equal(rule.style.getPropertyValue(shorthand), "initial", shorthand);
  }
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

test("intrinsic flex branches expand into Chromium longhand state", () => {
  const cases = [
    ["content", "1 1 content"],
    ["content 2 3", "2 3 content"],
    ["2 max-content", "2 1 max-content"],
    ["2 3 fit-content", "2 3 fit-content"],
    ["stretch 2", "2 1 stretch"],
  ] as const;

  for (const [input, expected] of cases) {
    const rule = createStyleRule(".x");
    rule.style.setProperty("flex", input);

    assert.deepEqual(
      Array.from({ length: rule.style.length }, (_, index) => rule.style.item(index)),
      ["flex-grow", "flex-shrink", "flex-basis"],
      input,
    );
    assert.equal(rule.style.getPropertyValue("flex"), expected, input);
    assert.equal(rule.style.cssText, `flex: ${expected};`, input);
  }
});

test("flex-basis owns typed calc-size state without weakening flex atomicity", () => {
  const rule = createStyleRule(".x");
  rule.style.setProperty("flex-basis", "calc-size(auto, size / 2 + 1px)", "important");

  assert.equal(
    rule.style.getPropertyValue("flex-basis"),
    "calc-size(auto, 1px + (0.5 * size))",
  );
  assert.equal(rule.style.getPropertyPriority("flex-basis"), "important");

  rule.style.setProperty("flex", "0 0 auto", "important");
  const before = rule.style.cssText;
  rule.style.setProperty("flex", "1 1 calc-size(auto, size)");
  assert.equal(rule.style.cssText, before);

  rule.style.setProperty("flex-basis", "calc-size(any, size)");
  assert.equal(rule.style.cssText, before);
});

test("removing an intrinsic flex longhand cannot reactivate shorthand state", () => {
  const rule = createStyleRule(".x");
  rule.style.setProperty("flex", "2 3 max-content", "important");
  rule.style.setProperty("flex-basis", "content", "important");

  assert.equal(rule.style.getPropertyValue("flex"), "2 3 content");
  assert.equal(rule.style.removeProperty("flex-basis"), "content");
  assert.equal(rule.style.getPropertyValue("flex"), "");
  assert.equal(rule.style.cssText, "flex-grow: 2 !important; flex-shrink: 3 !important;");
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

test("structural shorthand codecs expand and synthesize Chromium state", () => {
  const cases = [
    {
      shorthand: "border-inline",
      value: "2px dashed blue",
      items: [
        "border-inline-start-width",
        "border-inline-end-width",
        "border-inline-start-style",
        "border-inline-end-style",
        "border-inline-start-color",
        "border-inline-end-color",
      ],
      getter: "2px dashed blue",
    },
    {
      shorthand: "grid-template",
      value: "100px / 1fr 2fr",
      items: ["grid-template-rows", "grid-template-columns", "grid-template-areas"],
      getter: "100px / 1fr 2fr",
    },
    {
      shorthand: "grid-area",
      value: "1 / 2 / 3 / 4",
      items: ["grid-row-start", "grid-column-start", "grid-row-end", "grid-column-end"],
      getter: "1 / 2 / 3 / 4",
    },
    {
      shorthand: "place-items",
      value: "center stretch",
      items: ["align-items", "justify-items"],
      getter: "center stretch",
    },
    {
      shorthand: "columns",
      value: "100px 2",
      items: ["column-width", "column-count", "column-height", "column-wrap"],
      getter: "100px 2",
    },
    {
      shorthand: "text-wrap",
      value: "wrap balance",
      items: ["text-wrap-mode", "text-wrap-style"],
      getter: "balance",
    },
    {
      shorthand: "scroll-timeline",
      value: "--x block",
      items: ["scroll-timeline-name", "scroll-timeline-axis"],
      getter: "--x",
    },
  ] as const;

  for (const fixture of cases) {
    const rule = createStyleRule(".x");
    rule.style.setProperty(fixture.shorthand, fixture.value);
    assert.deepEqual(
      Array.from({ length: rule.style.length }, (_, index) => rule.style.item(index)),
      fixture.items,
      fixture.shorthand,
    );
    assert.equal(
      rule.style.getPropertyValue(fixture.shorthand),
      fixture.getter,
      fixture.shorthand,
    );
    assert.equal(
      rule.style.cssText,
      `${fixture.shorthand}: ${fixture.getter};`,
      fixture.shorthand,
    );

    const overriddenLonghand = fixture.items[0];
    rule.style.setProperty(overriddenLonghand, "inherit");
    rule.style.removeProperty(overriddenLonghand);
    assert.equal(rule.style.getPropertyValue(fixture.shorthand), "", fixture.shorthand);
    assert.equal(rule.style.cssText.includes(`${fixture.shorthand}:`), false, fixture.shorthand);
  }
});

test("layered and image shorthands preserve complete longhand state", () => {
  const cases = [
    [
      "background",
      'url("a.png") center / cover no-repeat, red',
      'url("a.png") center center / cover no-repeat, red',
      9,
    ],
    [
      "mask",
      'url("m.png") center / cover no-repeat',
      'url("m.png") center center / cover no-repeat',
      9,
    ],
    [
      "border-image",
      'url("x.png") 30 / 10 / 0 stretch',
      'url("x.png") 30 / 10 / 0 stretch',
      5,
    ],
    [
      "offset",
      'path("M 0 0 L 100 100") 50% auto 45deg / center',
      'path("M 0 0 L 100 100") 50% auto 45deg / center center',
      5,
    ],
  ] as const;

  for (const [shorthand, input, expected, length] of cases) {
    const sheet = new CSSStyleSheet();
    sheet.insertRule(".x {}");
    const rule = sheet.cssRules[0];
    assert.ok(rule instanceof CSSStyleRule);
    rule.style.setProperty(shorthand, input);

    assert.equal(rule.style.length, length, shorthand);
    assert.equal(rule.style.getPropertyValue(shorthand), expected, shorthand);
    assert.equal(rule.style.cssText, `${shorthand}: ${expected};`, shorthand);

    const serialized = sheet.serialize();
    const reparsed = new CSSStyleSheet();
    reparsed.replaceSync(serialized);
    assert.equal(reparsed.serialize(), serialized, shorthand);
  }
});

test("grid template areas expand through the accepted typed representation", () => {
  const rule = createStyleRule(".x");
  rule.style.setProperty(
    "grid-template",
    '"a a" 100px "b c" 1fr / 1fr 2fr',
  );

  assert.deepEqual(
    Array.from({ length: rule.style.length }, (_, index) => rule.style.item(index)),
    ["grid-template-rows", "grid-template-columns", "grid-template-areas"],
  );
  assert.equal(rule.style.getPropertyValue("grid-template-rows"), "100px 1fr");
  assert.equal(rule.style.getPropertyValue("grid-template-columns"), "1fr 2fr");
  assert.equal(
    rule.style.getPropertyValue("grid-template"),
    '"a a" 100px "b c" 1fr / 1fr 2fr',
  );
});
