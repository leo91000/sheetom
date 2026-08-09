import fc from "fast-check";
import { expect, test } from "vitest";

import {
  CSSStyleRule as SheetOMStyleRule,
  CSSStyleSheet as SheetOMStyleSheet,
} from "../../src/index.js";

type Mutation =
  | { operation: "remove"; name: string }
  | { operation: "set"; name: string; value: string; priority: string };

interface DeclarationLike {
  cssText: string;
  readonly length: number;
  getPropertyPriority(name: string): string;
  getPropertyValue(name: string): string;
  item(index: number): string;
  removeProperty(name: string): string;
  setProperty(name: string, value: string | null, priority?: string): void;
}

const trackedProperties = [
  "--token",
  "background-image",
  "color",
  "font-family",
  "padding-bottom",
  "padding-left",
  "padding-right",
  "padding-top",
  "transform",
  "width",
] as const;

function setMutation(name: string, values: readonly string[]) {
  return fc.record({
    operation: fc.constant("set" as const),
    name: fc.constant(name),
    value: fc.constantFrom(...values),
    priority: fc.constantFrom("", "important", "bogus"),
  });
}

const mutation = fc.oneof(
  setMutation("width", [
    "0",
    "10px",
    "calc(1px + 2px)",
    "calc(1px",
    "var(--token, 10px)",
    "10px; color:red",
  ]),
  setMutation("color", [
    "red",
    "rgb(1 2 3 / 50%)",
    "rgb(1 2 3",
    "red/*comment",
    "var(--token, rgb(1 2 3))",
    "red !important",
  ]),
  setMutation("font-family", ["serif", '"Gotham"']),
  setMutation("background-image", [
    "none",
    'url("https://example.com/a:b")',
    "url(foo",
    "linear-gradient(red, blue",
  ]),
  setMutation("transform", ["none", "translateX(1px)", "translateX(1px"]),
  setMutation("padding", ["1px", "1px 2px", "var(--token, 1px)"]),
  setMutation("--token", [
    "red",
    '"a;b"',
    "func(a;b)",
    "foo\\!bar",
    "url(foo!bar)",
    "var(--fallback, red)",
  ]),
  fc.record({
    operation: fc.constant("remove" as const),
    name: fc.constantFrom(...trackedProperties, "padding"),
  }),
);

function observe(style: DeclarationLike) {
  return {
    cssText: style.cssText,
    length: style.length,
    items: Array.from({ length: style.length }, (_, index) => style.item(index)),
    values: Object.fromEntries(
      trackedProperties.map(name => [name, style.getPropertyValue(name)]),
    ),
    priorities: Object.fromEntries(
      trackedProperties.map(name => [name, style.getPropertyPriority(name)]),
    ),
  };
}

function createSheetOMStyle(): DeclarationLike {
  const sheet = new SheetOMStyleSheet();
  sheet.insertRule(".fuzz {}");
  const rule = sheet.cssRules[0];
  if (!(rule instanceof SheetOMStyleRule)) throw new TypeError("Expected style rule");
  return rule.style;
}

function applyMutation(style: DeclarationLike, candidate: Mutation): void {
  if (candidate.operation === "remove") {
    style.removeProperty(candidate.name);
    return;
  }
  style.setProperty(candidate.name, candidate.value, candidate.priority);
}

test("grammar-oriented declaration sequences match the native engine consensus", () => {
  fc.assert(fc.property(
    fc.array(mutation, { minLength: 1, maxLength: 20 }),
    mutations => {
      const sheetOM = createSheetOMStyle();
      const native = document.createElement("div").style;

      for (const candidate of mutations) {
        applyMutation(sheetOM, candidate);
        applyMutation(native, candidate);
        expect(observe(sheetOM)).toEqual(observe(native));
      }
    },
  ), {
    seed: 0x5e37_0b,
    numRuns: 100,
  });
});
