import { expect, test } from "vitest";

import { CSSStyleRule, CSSStyleSheet } from "../src/index.js";

function createStyle() {
  const sheet = new CSSStyleSheet();
  sheet.insertRule(".palette {}");
  const rule = sheet.cssRules[0];
  if (!(rule instanceof CSSStyleRule)) throw new TypeError("Expected a style rule");
  return { sheet, style: rule.style };
}

test("font-palette owns keywords, percentages, color spaces, and recursive mixes", () => {
  for (const [input, expected] of [
    ["light", "light"],
    ["dark", "dark"],
    ["palette-mix(in lch, normal, normal)", "palette-mix(in lch, normal, normal)"],
    [
      "palette-mix(in srgb, normal, dark 20%)",
      "palette-mix(in srgb, normal 80%, dark)",
    ],
    [
      "palette-mix(in oklch longer hue, --brand 10%, palette-mix(in srgb-linear, normal, dark 40%))",
      "palette-mix(in oklch longer hue, --brand 10%, palette-mix(in srgb-linear, normal 60%, dark))",
    ],
    [
      "palette-mix(in xyz, normal, dark)",
      "palette-mix(in xyz-d65, normal, dark)",
    ],
    [
      "palette-mix(in lch increasing hue, normal, dark)",
      "palette-mix(in lch increasing hue, normal, dark)",
    ],
    [
      "palette-mix(in srgb, normal calc(-1%), dark)",
      "palette-mix(in srgb, normal calc(-1%), dark)",
    ],
  ] as const) {
    const { sheet, style } = createStyle();
    style.setProperty("font-palette", input, "important");
    expect(style.getPropertyValue("font-palette"), input).toBe(expected);
    expect(style.getPropertyPriority("font-palette"), input).toBe("important");
    expect(style.cssText, input).toBe(`font-palette: ${expected} !important;`);

    const serialized = sheet.serialize();
    const reparsed = new CSSStyleSheet();
    reparsed.replaceSync(serialized);
    expect(reparsed.serialize(), input).toBe(serialized);
  }
});

test("invalid font-palette mixes are atomic no-ops", () => {
  for (const input of [
    "palette-mix(normal, dark)",
    "palette-mix(in lch, normal)",
    "palette-mix(in lch, normal, dark, light)",
    "palette-mix(in lch, red, dark)",
    "palette-mix(in lch, normal -1%, dark)",
    "palette-mix(in lch, normal 0%, dark 0%)",
    "palette-mix(in lch specified hue, normal, dark)",
    "palette-mix(in --custom, normal, dark)",
  ]) {
    const { style } = createStyle();
    style.setProperty("font-palette", "palette-mix(in lch, light, dark)", "important");
    const before = style.cssText;
    style.setProperty("font-palette", input);
    expect(style.cssText, input).toBe(before);
  }
});
