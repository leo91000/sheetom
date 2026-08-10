import assert from "node:assert/strict";
import { test } from "vitest";

import {
  CSSKeyframesRule,
  CSSMediaRule,
  CSSStyleRule,
  CSSStyleSheet,
  parseStyleSheet,
} from "../src/index.js";

function firstStyleRule(sheet: CSSStyleSheet): CSSStyleRule {
  const rule = sheet.cssRules[0];
  assert.ok(rule instanceof CSSStyleRule);
  return rule;
}

test("resource budget options reject values that cannot cross the native boundary", () => {
  assert.throws(
    () => new CSSStyleSheet({ resourceBudget: "small" as never }),
    error => error instanceof TypeError && error.message.includes("resourceBudget"),
  );
  assert.throws(
    () => new CSSStyleSheet({ resourceBudget: [] as never }),
    error => error instanceof TypeError && error.message.includes("resourceBudget"),
  );
  for (const maxStylesheetBytes of [-1, 1.5, Number.NaN, 2 ** 32, "64"]) {
    assert.throws(
      () => new CSSStyleSheet({
        resourceBudget: { maxStylesheetBytes: maxStylesheetBytes as never },
      }),
      RangeError,
    );
  }
  assert.throws(
    () => new CSSStyleSheet({ resourceBudget: { maxSyntaxDepth: 16_385 } }),
    RangeError,
  );
});

test("stylesheet source and rule-count limits reject atomically", () => {
  const sourceLimited = new CSSStyleSheet({
    resourceBudget: { maxStylesheetBytes: 64 },
  });
  sourceLimited.replaceSync(".old { color: red; }");
  const previousRule = sourceLimited.cssRules[0];
  assert.throws(
    () => sourceLimited.replaceSync(`.new { --x: ${"x".repeat(64)}; }`),
    error => error instanceof RangeError && error.message.includes("SHEETOM_INPUT_LIMIT"),
  );
  assert.equal(sourceLimited.cssRules.length, 1);
  assert.equal(sourceLimited.cssRules[0], previousRule);
  assert.equal(sourceLimited.serialize(), ".old {\n  color: red;\n}\n");

  const ruleLimited = new CSSStyleSheet({
    resourceBudget: { maxRuleCount: 1 },
  });
  ruleLimited.replaceSync(".old { color: red; }");
  assert.throws(
    () => ruleLimited.replaceSync(".a {} .b {}"),
    error => error instanceof RangeError && error.message.includes("SHEETOM_RULE_LIMIT"),
  );
  assert.equal(ruleLimited.cssRules.length, 1);
  assert.equal(firstStyleRule(ruleLimited).selectorText, ".old");
});

test("declaration-value, nesting and block-cardinality limits preserve prior state", () => {
  const sheet = new CSSStyleSheet({
    resourceBudget: {
      maxDeclarationValueBytes: 16,
      maxSyntaxDepth: 2,
      maxDeclarationsPerBlock: 1,
    },
  });
  sheet.replaceSync(".card { width: 1px; }");
  const style = firstStyleRule(sheet).style;

  assert.throws(
    () => style.setProperty("width", "12345678901234567"),
    error => error instanceof RangeError && error.message.includes("SHEETOM_INPUT_LIMIT"),
  );
  assert.equal(style.getPropertyValue("width"), "1px");

  assert.throws(
    () => style.setProperty("width", "fn(fn(fn(1", "important"),
    error => error instanceof RangeError && error.message.includes("SHEETOM_NESTING_LIMIT"),
  );
  assert.equal(style.getPropertyValue("width"), "1px");
  assert.equal(style.getPropertyPriority("width"), "");

  assert.throws(
    () => {
      style.cssText = "width: 2px; height: 3px";
    },
    error => error instanceof RangeError && error.message.includes("SHEETOM_DECLARATION_LIMIT"),
  );
  assert.equal(style.cssText, "width: 1px;");
});

test("resource budgets propagate to parsed, nested and detached declaration objects", () => {
  const sheet = parseStyleSheet("@media all { .card { width: 1px; } }", {
    resourceBudget: { maxDeclarationValueBytes: 4 },
  });
  const media = sheet.cssRules[0];
  assert.ok(media && "cssRules" in media);
  const nested = (media as { cssRules: { [index: number]: unknown } }).cssRules[0];
  assert.ok(nested instanceof CSSStyleRule);
  sheet.deleteRule(0);

  assert.throws(
    () => nested.style.setProperty("width", "12345"),
    RangeError,
  );
  assert.equal(nested.style.getPropertyValue("width"), "1px");
});

test("callers can raise a lowered budget without changing global behavior", () => {
  const source = `.card { --value: ${"x".repeat(32)}; }`;
  const lower = new CSSStyleSheet({
    resourceBudget: { maxDeclarationValueBytes: 16 },
  });
  assert.throws(() => lower.replaceSync(source), RangeError);

  const raised = new CSSStyleSheet({
    resourceBudget: { maxDeclarationValueBytes: 64 },
  });
  raised.replaceSync(source);
  assert.equal(firstStyleRule(raised).style.getPropertyValue("--value"), "x".repeat(32));

  const defaults = new CSSStyleSheet();
  defaults.replaceSync(source);
  assert.equal(firstStyleRule(defaults).style.getPropertyValue("--value"), "x".repeat(32));
});

test("exact UTF-8 byte, syntax, rule and declaration boundaries remain usable", () => {
  const source = ".é{}";
  const sheet = new CSSStyleSheet({
    resourceBudget: {
      maxStylesheetBytes: Buffer.byteLength(source),
      maxDeclarationValueBytes: 4,
      maxSyntaxDepth: 2,
      maxRuleCount: 1,
      maxDeclarationsPerBlock: 1,
    },
  });
  sheet.replaceSync(source);
  const rule = firstStyleRule(sheet);
  rule.style.setProperty("--value", "éé");
  assert.equal(rule.style.getPropertyValue("--value"), "éé");

  const syntaxSheet = new CSSStyleSheet({
    resourceBudget: {
      maxDeclarationValueBytes: 8,
      maxSyntaxDepth: 2,
    },
  });
  syntaxSheet.replaceSync(".a{}");
  const syntaxRule = firstStyleRule(syntaxSheet);
  syntaxRule.style.setProperty("--value", "fn(fn())");
  assert.equal(syntaxRule.style.getPropertyValue("--value"), "fn(fn())");
});

test("internal parser wrappers do not consume the caller's source budget", () => {
  const styleSource = ".a{}";
  const styleSheet = new CSSStyleSheet({
    resourceBudget: { maxStylesheetBytes: Buffer.byteLength(styleSource) },
  });
  styleSheet.replaceSync(styleSource);
  const style = firstStyleRule(styleSheet);
  style.selectorText = ".é";
  assert.equal(style.selectorText, ".é");

  const mediaSource = "@media all{}";
  const mediaSheet = new CSSStyleSheet({
    resourceBudget: { maxStylesheetBytes: Buffer.byteLength(mediaSource) },
  });
  mediaSheet.replaceSync(mediaSource);
  const media = mediaSheet.cssRules[0];
  assert.ok(media instanceof CSSMediaRule);
  media.media.mediaText = "print";
  assert.equal(media.conditionText, "print");
});

test("sequential declaration mutations cannot exceed the block budget", () => {
  const sheet = new CSSStyleSheet({
    resourceBudget: { maxDeclarationsPerBlock: 1 },
  });
  sheet.replaceSync(".card { width: 1px; }");
  const style = firstStyleRule(sheet).style;
  assert.throws(
    () => style.setProperty("height", "2px"),
    error => error instanceof RangeError && error.message.includes("SHEETOM_DECLARATION_LIMIT"),
  );
  assert.equal(style.cssText, "width: 1px;");

  style.removeProperty("width");
  assert.throws(
    () => style.setProperty("padding", "1px"),
    error => error instanceof RangeError && error.message.includes("SHEETOM_DECLARATION_LIMIT"),
  );
  assert.equal(style.cssText, "");
});

test("sequential sheet, grouping and keyframe insertions cannot exceed the rule budget", () => {
  const sheet = new CSSStyleSheet({ resourceBudget: { maxRuleCount: 1 } });
  sheet.replaceSync(".old {}");
  const previousRule = sheet.cssRules[0];
  assert.throws(
    () => sheet.insertRule(".new {}", 1),
    error => error instanceof RangeError && error.message.includes("SHEETOM_RULE_LIMIT"),
  );
  assert.equal(sheet.cssRules.length, 1);
  assert.equal(sheet.cssRules[0], previousRule);

  const groupingSheet = new CSSStyleSheet({ resourceBudget: { maxRuleCount: 2 } });
  groupingSheet.replaceSync("@media all { .old {} }");
  const media = groupingSheet.cssRules[0];
  assert.ok(media instanceof CSSMediaRule);
  const previousChild = media.cssRules[0];
  assert.throws(() => media.insertRule(".new {}", 1), RangeError);
  assert.equal(media.cssRules.length, 1);
  assert.equal(media.cssRules[0], previousChild);

  groupingSheet.deleteRule(0);
  assert.throws(() => media.insertRule(".detached {}", 1), RangeError);
  assert.equal(media.cssRules.length, 1);

  const keyframesSheet = new CSSStyleSheet({ resourceBudget: { maxRuleCount: 2 } });
  keyframesSheet.replaceSync("@keyframes fade { from {} }");
  const keyframes = keyframesSheet.cssRules[0];
  assert.ok(keyframes instanceof CSSKeyframesRule);
  assert.throws(() => keyframes.appendRule("to {}"), RangeError);
  assert.equal(keyframes.cssRules.length, 1);
});
