import {
  CSSGroupingRule,
  CSSStyleRule,
  CSSStyleSheet,
} from "../../src/index.js";
import type { FixtureAdapter } from "./operation-fixture.js";

export function createSheetOMFixtureAdapter(): FixtureAdapter {
  return {
    invoke(operation, target, args) {
      switch (operation.op) {
        case "constructStyleSheet":
          return new CSSStyleSheet();
        case "constructStyleRule":
          return constructStyleRule(args[0] as string);
        case "replaceSync":
          return (target as CSSStyleSheet).replaceSync(args[0] as string);
        case "getRule":
          return getRule(target as CSSStyleSheet | CSSGroupingRule, args[0] as number);
        case "getStyle":
          return (target as CSSStyleRule).style;
        case "insertRule":
          return Reflect.apply(
            (target as CSSStyleSheet | CSSGroupingRule).insertRule,
            target,
            args,
          );
        case "deleteRule":
          return Reflect.apply(
            (target as CSSStyleSheet | CSSGroupingRule).deleteRule,
            target,
            args,
          );
        case "identity":
          return target;
        case "setProperty":
          return Reflect.apply(
            (target as CSSStyleDeclaration).setProperty,
            target,
            args,
          );
        case "getPropertyValue":
          return Reflect.apply(
            (target as CSSStyleDeclaration).getPropertyValue,
            target,
            args,
          );
        default:
          throw new Error(`Unsupported SheetOM fixture operation: ${operation.op}`);
      }
    },
  };
}

function getRule(target: CSSStyleSheet | CSSGroupingRule, index: number): unknown {
  return target.cssRules[index];
}

function constructStyleRule(selector: string): CSSStyleRule {
  const sheet = new CSSStyleSheet();
  sheet.insertRule(`${selector} {}`);
  const rule = sheet.cssRules[0];
  if (!(rule instanceof CSSStyleRule)) {
    throw new TypeError("Expected CSSStyleRule");
  }
  return rule;
}
