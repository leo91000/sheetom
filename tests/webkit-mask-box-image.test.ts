import { expect, test } from "vitest";

import { CSSStyleRule, CSSStyleSheet } from "../src/index.js";

const longhands = [
  "-webkit-mask-box-image-source",
  "-webkit-mask-box-image-slice",
  "-webkit-mask-box-image-width",
  "-webkit-mask-box-image-outset",
  "-webkit-mask-box-image-repeat",
] as const;

test("webkit mask box images own the complete prefixed shorthand state", () => {
  const sheet = new CSSStyleSheet();
  sheet.replaceSync(`
    .x {
      -webkit-mask-box-image:
        image-set(url(a.png) 1x, url(b.png) 2x)
        repeat round 1 2 fill / auto 3px / 4px 5px;
      color: red;
    }
  `);

  const rule = sheet.cssRules[0];
  expect(rule).toBeInstanceOf(CSSStyleRule);
  if (!(rule instanceof CSSStyleRule)) throw new TypeError("expected a style rule");

  expect(Array.from(rule.style)).toEqual([...longhands, "color"]);
  expect(rule.style.getPropertyValue("-webkit-mask-box-image")).toBe("");
  expect(longhands.map(name => rule.style.getPropertyValue(name))).toEqual([
    'image-set(url("a.png") 1x, url("b.png") 2x)',
    "1 2 fill",
    "auto 3px",
    "4px 5px",
    "repeat round",
  ]);

  const beforeInvalid = rule.style.cssText;
  rule.style.setProperty("-webkit-mask-box-image", "repeat none round");
  expect(rule.style.cssText).toBe(beforeInvalid);

  rule.style.setProperty("-webkit-mask-box-image-width", "6px 7px");
  expect(rule.style.getPropertyValue("-webkit-mask-box-image-width")).toBe("6px 7px");
  expect(rule.style.removeProperty("-webkit-mask-box-image-width")).toBe("6px 7px");
  expect(rule.style.getPropertyValue("-webkit-mask-box-image-width")).toBe("");
  expect(Array.from(rule.style)).toEqual([
    "-webkit-mask-box-image-source",
    "-webkit-mask-box-image-slice",
    "-webkit-mask-box-image-outset",
    "-webkit-mask-box-image-repeat",
    "color",
  ]);

  const serialized = sheet.serialize();
  const reparsed = new CSSStyleSheet();
  reparsed.replaceSync(serialized);
  expect(reparsed.serialize()).toBe(serialized);
});

test("webkit mask box image longhands accept and compress one-to-four values", () => {
  const sheet = new CSSStyleSheet();
  sheet.insertRule(".x {}");
  const rule = sheet.cssRules[0];
  if (!(rule instanceof CSSStyleRule)) throw new TypeError("expected a style rule");

  rule.style.setProperty("-webkit-mask-box-image-slice", "1 1 1 1 FILL");
  rule.style.setProperty("-webkit-mask-box-image-width", "auto 2px auto 2px");
  rule.style.setProperty("-webkit-mask-box-image-outset", "1px 2px 3px 2px");

  expect(rule.style.getPropertyValue("-webkit-mask-box-image-slice")).toBe("1 fill");
  expect(rule.style.getPropertyValue("-webkit-mask-box-image-width")).toBe("auto 2px");
  expect(rule.style.getPropertyValue("-webkit-mask-box-image-outset")).toBe("1px 2px 3px");
});
