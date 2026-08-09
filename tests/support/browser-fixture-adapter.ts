import type { FixtureAdapter } from "./operation-fixture.js";

export function createNativeBrowserFixtureAdapter(): FixtureAdapter {
  return {
    invoke(operation, target, args) {
      switch (operation.op) {
        case "constructStyleSheet":
          return new globalThis.CSSStyleSheet();
        case "constructStyleRule": {
          const sheet = new globalThis.CSSStyleSheet();
          sheet.insertRule(`${args[0]} {}`);
          return sheet.cssRules[0];
        }
        case "getStyle":
          return (target as globalThis.CSSStyleRule).style;
        case "replaceSync":
          return (target as globalThis.CSSStyleSheet).replaceSync(args[0] as string);
        case "getRule":
          return (target as globalThis.CSSStyleSheet | globalThis.CSSGroupingRule)
            .cssRules[args[0] as number];
        case "insertRule":
          return Reflect.apply(
            (target as globalThis.CSSStyleSheet | globalThis.CSSGroupingRule).insertRule,
            target,
            args,
          );
        case "deleteRule":
          return Reflect.apply(
            (target as globalThis.CSSStyleSheet | globalThis.CSSGroupingRule).deleteRule,
            target,
            args,
          );
        case "identity":
          return target;
        case "setProperty":
          return Reflect.apply(
            (target as globalThis.CSSStyleDeclaration).setProperty,
            target,
            args,
          );
        case "removeProperty":
          return Reflect.apply(
            (target as globalThis.CSSStyleDeclaration).removeProperty,
            target,
            args,
          );
        case "getPropertyValue":
          return Reflect.apply(
            (target as globalThis.CSSStyleDeclaration).getPropertyValue,
            target,
            args,
          );
        case "getPropertyPriority":
          return Reflect.apply(
            (target as globalThis.CSSStyleDeclaration).getPropertyPriority,
            target,
            args,
          );
        case "setCssText":
          (target as globalThis.CSSStyleDeclaration).cssText = args[0] as string;
          return undefined;
        default:
          throw new Error(`Unsupported browser fixture operation: ${operation.op}`);
      }
    },
  };
}
