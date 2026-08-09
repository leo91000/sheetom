import { expect, test } from "vitest";

const chromiumTest = test.skipIf(!navigator.userAgent.includes("Chrome"));

chromiumTest("Chromium preserves recovered setProperty values on the CSSOM surface", () => {
  const style = document.createElement("div").style;
  style.setProperty("padding", "1px");
  style.setProperty("padding", "72px var(--space, var(--space,");

  expect(style.getPropertyValue("padding")).toBe(
    "72px var(--space, var(--space,",
  );
  expect(style.cssText).toBe("padding: 72px var(--space, var(--space,;");
  expect(style.getPropertyValue("padding-top")).toBe("");
  expect(Array.from(style)).toEqual([
    "padding-top",
    "padding-right",
    "padding-bottom",
    "padding-left",
  ]);

  const reparsed = document.createElement("div").style;
  reparsed.cssText = style.cssText;
  expect(reparsed.cssText).toBe("");

  style.setProperty("padding-left", "3px");
  expect(style.getPropertyValue("padding")).toBe("");
  expect(style.cssText).toBe(
    "padding-top: ; padding-right: ; padding-bottom: ; padding-left: 3px;",
  );
});

chromiumTest("Chromium resolves parsed priorities and preserves custom-property presence", () => {
  const ordered = document.createElement("div").style;
  ordered.cssText =
    "width: 1px !important; color: red; width: 2px; height: 3px !important;";

  expect(Array.from(ordered)).toEqual(["color", "width", "height"]);
  expect(ordered.cssText).toBe(
    "color: red; width: 1px !important; height: 3px !important;",
  );

  ordered.setProperty("width", "", "bogus");
  expect(ordered.getPropertyValue("width")).toBe("1px");

  const custom = document.createElement("div").style;
  custom.setProperty("--X", " ");
  custom.setProperty("--x", "false");
  expect(Array.from(custom)).toEqual(["--X", "--x"]);
  expect(custom.getPropertyValue("--X")).toBe("");
  expect(custom.cssText).toBe("--X: ; --x: false;");
});

chromiumTest("Chromium applies WebIDL conversions at CSSOM method boundaries", () => {
  const style = document.createElement("div").style;
  style.cssText = "width: 1px; color: red;";
  expect(style.item(1.9)).toBe("color");
  expect(Reflect.apply(style.getPropertyValue, style, [null])).toBe("");

  const sheet = new CSSStyleSheet();
  sheet.insertRule(".first {}");
  sheet.insertRule(".second {}", 1);
  Reflect.apply(sheet.deleteRule, sheet, [Number.NaN]);
  expect(Array.from(sheet.cssRules, rule => rule.cssText)).toEqual([
    ".second { }",
  ]);
});

chromiumTest("Chromium accepts typed attr() and conditional if() substitutions", () => {
  const style = document.createElement("div").style;
  const attrValue = "attr(data-width type(<length>), 1px)";
  const ifValue = "if(style(--theme: dark): white; else: black)";

  style.setProperty("width", attrValue);
  style.setProperty("color", ifValue);
  expect(style.getPropertyValue("width")).toBe(attrValue);
  expect(style.getPropertyValue("color")).toBe(ifValue);

  style.setProperty("width", "attr()");
  style.setProperty("color", "if()");
  expect(style.getPropertyValue("width")).toBe(attrValue);
  expect(style.getPropertyValue("color")).toBe(ifValue);
});

chromiumTest("Chromium escapes logical custom-property names only when serializing", () => {
  const style = document.createElement("div").style;
  style.setProperty("-- x", "red");
  style.setProperty("--x!", "blue");
  style.setProperty("--", "green");

  expect(Array.from(style)).toEqual(["-- x", "--x!"]);
  expect(style.getPropertyValue("-- x")).toBe("red");
  expect(style.cssText).toBe("--\\ x: red; --x\\!: blue;");
});

chromiumTest("Chromium rejects priority tokens embedded in setProperty values", () => {
  const style = document.createElement("div").style;
  style.setProperty("color", "blue");
  style.setProperty("color", "red !important");
  style.setProperty("--fallback", "var(--x, !important)");
  style.setProperty("--url", "url(foo!bar)");
  style.setProperty("--escaped", "foo\\!bar");

  expect(style.getPropertyValue("color")).toBe("blue");
  expect(style.getPropertyValue("--fallback")).toBe("");
  expect(style.getPropertyValue("--url")).toBe("url(foo!bar)");
  expect(style.getPropertyValue("--escaped")).toBe("foo\\!bar");
});

chromiumTest("Chromium retains pending provenance for non-padding shorthands", () => {
  const style = document.createElement("div").style;
  const value = "12px var(--gap, var(--gap,";
  style.setProperty("margin", value);

  expect(Array.from(style)).toEqual([
    "margin-top",
    "margin-right",
    "margin-bottom",
    "margin-left",
  ]);
  expect(style.getPropertyValue("margin")).toBe(value);
  expect(style.getPropertyValue("margin-top")).toBe("");

  style.setProperty("margin-left", "3px");
  expect(style.cssText).toBe(
    "margin-top: ; margin-right: ; margin-bottom: ; margin-left: 3px;",
  );
});

chromiumTest("Chromium rule WebIDL constructors and cssText setters are inert", () => {
  expect(() => new CSSStyleRule()).toThrow(TypeError);
  const sheet = new CSSStyleSheet();
  sheet.replaceSync("@media screen { .x {} }");
  const media = sheet.cssRules[0] as CSSMediaRule;
  const before = media.cssText;
  expect(Reflect.set(media, "cssText", "garbage")).toBe(true);
  expect(media.cssText).toBe(before);
});
