import assert from "node:assert/strict";
import type { CSSRule, CSSStyleRule, CSSGroupingRule, CSSStyleSheet } from "../src/index.js";

type RuleState = CSSRule & Partial<Pick<CSSStyleRule, "style"> & Pick<CSSGroupingRule, "cssRules">>;
interface RuleSnapshot {
  type: string;
  children: RuleSnapshot[];
  [field: string]: unknown;
}
type AuthoringAPI = Pick<typeof import("../src/index.js"), "CSSStyleSheet" | "CSSGroupingRule">;

// Keep rule topology, descriptor state, declaration names/order, and priorities
// exact. Canonicalize each longhand independently: a whole-sheet serializer
// dropping a rule/declaration or collapsing a shorthand cannot bless its own
// loss. Equivalent scalar spellings (blue/#00f, calc(1px)/1px) may normalize.
export function assertAuthoringRoundTrip(api: AuthoringAPI, before: CSSStyleSheet, after: CSSStyleSheet, snapshotRule = snapshotAuthoringRule) {
  const scalarCache = new Map();
  function snapshot(rule: RuleState): RuleSnapshot {
    const state = snapshotRule(rule);
    if (rule.style) {
      const style = rule.style;
      state.style = {
        parent: style.parentRule === rule,
        items: Array.from(style, name => {
          assert.ok(name);
          const value = style.getPropertyValue(name);
          const priority = style.getPropertyPriority(name);
          // Pending shorthand members can expose an empty longhand value.
          if (value === "") return [name, value, priority];
          const context = rule.constructor.name;
          const key = JSON.stringify([context, name, value, priority]);
          if (!scalarCache.has(key)) {
            const source = context === "CSSFontFaceRule" ? "@font-face {}"
              : context === "CSSPageRule" ? "@page {}"
                : context === "CSSMarginRule" ? "@page { @top-left {} }" : ".scalar {}";
            const scalar = new api.CSSStyleSheet();
            scalar.replaceSync(source);
            const root = scalar.cssRules[0];
            const target: RuleState | undefined = root instanceof api.CSSGroupingRule && context === "CSSMarginRule" ? root.cssRules[0] : root;
            assert.ok(target?.style);
            target.style.setProperty(name, value, priority);
            assert.equal(target.style.length, 1, `Scalar normalization must retain ${name}`);
            assert.equal(target.style.item(0), name);
            scalarCache.set(key, scalar.serializeStrict());
          }
          return [name, scalarCache.get(key), priority];
        }),
      };
    }
    if (rule.cssRules) state.children = Array.from(rule.cssRules, child => { assert.ok(child); return snapshot(child); });
    return state;
  }
  assert.deepEqual(Array.from(after.cssRules, rule => { assert.ok(rule); return snapshot(rule); }), Array.from(before.cssRules, rule => { assert.ok(rule); return snapshot(rule); }));
}

export function snapshotAuthoringRule(rule: RuleState): RuleSnapshot {
  const result: RuleSnapshot = { type: rule.constructor.name, children: [] };
  for (const field of ["selectorText", "conditionText", "name", "keyText", "syntax", "inherits", "initialValue", "fontFamily", "basePalette", "overrideColors", "start", "end", "namespaceURI", "prefix", "returnType", "contents"])
    if (field in rule) result[field] = Reflect.get(rule, field);
  if (rule.style) {
    const style = rule.style;
    result.style = { cssText: style.cssText, items: Array.from({ length: style.length }, (_, index) => {
      const name = style.item(index);
      return [name, style.getPropertyValue(name), style.getPropertyPriority(name)];
    }), parent: style.parentRule === rule };
  }
  for (const method of ["getParameters", "getArguments"])
    if (typeof Reflect.get(rule, method) === "function") result[method] = Reflect.apply(Reflect.get(rule, method), rule, []);
  for (const field of ["nameList", "types"])
    if (field in rule) result[field] = Array.from(Reflect.get(rule, field));
  if (rule.cssRules) result.children = Array.from({ length: rule.cssRules.length }, (_, index) => snapshotAuthoringRule(rule.cssRules![index]!));
  return result;
}
