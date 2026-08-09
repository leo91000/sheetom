import {
  CSSStyleRule,
  CSSStyleSheet,
} from "../../src/index.js";

export function createStyleRule(selector = ".sheetom-test"): CSSStyleRule {
  const sheet = new CSSStyleSheet();
  sheet.insertRule(`${selector} {}`);
  const rule = sheet.cssRules[0];
  if (!(rule instanceof CSSStyleRule)) {
    throw new TypeError("Expected CSSStyleRule");
  }
  return rule;
}
