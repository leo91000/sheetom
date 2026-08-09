import type { FixtureAdapter } from "./operation-fixture.js";

export function createNativeBrowserFixtureAdapter(): FixtureAdapter {
  return {
    invoke(operation, target, args) {
      switch (operation.op) {
        case "constructStyleRule": {
          const sheet = new globalThis.CSSStyleSheet();
          sheet.insertRule(`${args[0]} {}`);
          return sheet.cssRules[0];
        }
        case "getStyle":
          return (target as globalThis.CSSStyleRule).style;
        case "setProperty":
          return Reflect.apply(
            (target as globalThis.CSSStyleDeclaration).setProperty,
            target,
            args,
          );
        case "getPropertyValue":
          return Reflect.apply(
            (target as globalThis.CSSStyleDeclaration).getPropertyValue,
            target,
            args,
          );
        default:
          throw new Error(`Unsupported browser fixture operation: ${operation.op}`);
      }
    },
  };
}
