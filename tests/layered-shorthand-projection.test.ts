import assert from "node:assert/strict";
import { test } from "vitest";

import { CSSStyleRule, CSSStyleSheet } from "../src/index.js";

type LayeredFixture = {
  property: "background" | "mask" | "-webkit-mask";
  compact: string;
  spaced: string;
  expected: string;
  positionX: string;
  positionY: string;
  size: string;
};

const fixtures: LayeredFixture[] = [
  {
    property: "background",
    compact: "image-set(url(a.png) 1x) center/cover no-repeat red",
    spaced: "image-set(url(a.png) 1x) center / cover no-repeat red",
    expected: 'image-set(url("a.png") 1x) center center / cover no-repeat red',
    positionX: "center",
    positionY: "center",
    size: "cover",
  },
  {
    property: "mask",
    compact: "image-set(url(a.png) 1x) center/cover no-repeat",
    spaced: "image-set(url(a.png) 1x) center / cover no-repeat",
    expected: 'image-set(url("a.png") 1x) center center / cover no-repeat',
    positionX: "center",
    positionY: "center",
    size: "cover",
  },
  {
    property: "-webkit-mask",
    compact: "image-set(url(a.png) 1x) center/cover no-repeat",
    spaced: "image-set(url(a.png) 1x) center / cover no-repeat",
    expected: 'image-set(url("a.png") 1x) center center / cover no-repeat',
    positionX: "center",
    positionY: "center",
    size: "cover",
  },
  {
    property: "background",
    compact:
      "none center/cover, linear-gradient(red, blue) left top/10px 20px no-repeat red",
    spaced:
      "none center / cover, linear-gradient(red, blue) left top / 10px 20px no-repeat red",
    expected:
      "none center center / cover, linear-gradient(red, blue) left top / 10px 20px no-repeat red",
    positionX: "center, left",
    positionY: "center, top",
    size: "cover, 10px 20px",
  },
  {
    property: "mask",
    compact:
      "none center/cover, linear-gradient(red, blue) left top/10px 20px no-repeat",
    spaced:
      "none center / cover, linear-gradient(red, blue) left top / 10px 20px no-repeat",
    expected:
      "center center / cover, linear-gradient(red, blue) left top / 10px 20px no-repeat",
    positionX: "center, left",
    positionY: "center, top",
    size: "cover, 10px 20px",
  },
  {
    property: "-webkit-mask",
    compact:
      "none center/cover, linear-gradient(red, blue) left top/10px 20px no-repeat",
    spaced:
      "none center / cover, linear-gradient(red, blue) left top / 10px 20px no-repeat",
    expected:
      "center center / cover, linear-gradient(red, blue) left top / 10px 20px no-repeat",
    positionX: "center, left",
    positionY: "center, top",
    size: "cover, 10px 20px",
  },
];

function createRule(): { sheet: CSSStyleSheet; rule: CSSStyleRule } {
  const sheet = new CSSStyleSheet();
  sheet.insertRule(".x {}");
  const rule = sheet.cssRules[0];
  assert.ok(rule instanceof CSSStyleRule);
  return { sheet, rule };
}

test.each(fixtures)(
  "$property compact position/size matches the spaced form",
  ({ property, compact, spaced, expected, positionX, positionY, size }) => {
    const compactRule = createRule();
    const spacedRule = createRule();
    compactRule.rule.style.setProperty(property, compact);
    spacedRule.rule.style.setProperty(property, spaced);

    assert.equal(compactRule.rule.style.getPropertyValue(property), expected);
    assert.equal(
      compactRule.rule.style.getPropertyValue(property),
      spacedRule.rule.style.getPropertyValue(property),
    );
    assert.equal(compactRule.rule.style.cssText, spacedRule.rule.style.cssText);

    const positionPrefix = property === "background" ? "background" : "-webkit-mask";
    assert.equal(
      compactRule.rule.style.getPropertyValue(`${positionPrefix}-position-x`),
      positionX,
    );
    assert.equal(
      compactRule.rule.style.getPropertyValue(`${positionPrefix}-position-y`),
      positionY,
    );
    assert.equal(
      compactRule.rule.style.getPropertyValue(
        property === "background" ? "background-size" : "mask-size",
      ),
      size,
    );

    const serialized = compactRule.sheet.serialize();
    const reparsed = new CSSStyleSheet();
    reparsed.replaceSync(serialized);
    assert.equal(reparsed.serialize(), serialized);
  },
);

test("a later layered longhand mutation uses the expanded semantic records", () => {
  const { rule } = createRule();
  rule.style.setProperty(
    "background",
    "image-set(url(a.png) 1x) center/cover no-repeat red",
  );

  rule.style.setProperty("background-position-x", "right");

  assert.equal(rule.style.getPropertyValue("background-position-x"), "right");
  assert.equal(
    rule.style.getPropertyValue("background"),
    'image-set(url("a.png") 1x) right center / cover no-repeat red',
  );
});
