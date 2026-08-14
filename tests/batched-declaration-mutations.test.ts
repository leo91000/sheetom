import { strict as assert } from "node:assert";
import { describe, it } from "vitest";

import { CSSStyleRule, CSSStyleSheet } from "../src/index.js";

function styleRule(options: ConstructorParameters<typeof CSSStyleSheet>[0] = {}) {
  const sheet = new CSSStyleSheet(options);
  sheet.insertRule(".target {}");
  const rule = sheet.cssRules[0];
  assert.ok(rule instanceof CSSStyleRule);
  return { rule, sheet };
}

describe("CSSStyleDeclaration.applyMutations", () => {
  it("applies ordered shorthand, longhand, invalid, and removal operations", () => {
    const { rule, sheet } = styleRule({ diagnostics: true });

    const results = rule.style.applyMutations([
      { kind: "set", property: "padding", value: "1px 2px", priority: "important" },
      { kind: "set", property: "padding-left", value: "3px", priority: "important" },
      { kind: "set", property: "width", value: "20px; color: red" },
      { kind: "remove", property: "padding-right" },
      { kind: "set", property: "--publisher-token", value: "var(--space, 4px)" },
    ]);

    assert.deepEqual(results.map(result => result.kind), [
      "set",
      "set",
      "set",
      "remove",
      "set",
    ]);
    assert.deepEqual(results.map(result => result.kind === "set" && result.accepted), [
      true,
      true,
      false,
      false,
      true,
    ]);
    assert.equal(
      results[2]?.kind === "set" && results[2].diagnostic?.code,
      "INVALID_PROPERTY_VALUE",
    );
    assert.deepEqual(results[3], { kind: "remove", value: "2px" });
    assert.equal(rule.style.getPropertyValue("padding"), "");
    assert.equal(rule.style.getPropertyValue("padding-left"), "3px");
    assert.equal(rule.style.getPropertyValue("padding-right"), "");
    assert.equal(rule.style.getPropertyValue("width"), "");
    assert.equal(rule.style.getPropertyValue("--publisher-token"), "var(--space, 4px)");
    assert.deepEqual(sheet.takeDiagnostics(), [
      results[2]?.kind === "set" ? results[2].diagnostic : null,
    ]);
  });

  it("matches the final observable state of sequential CSSOM mutations", () => {
    const batched = styleRule();
    const sequential = styleRule();
    const operations = [
      { kind: "set", property: "background", value: "red" },
      { kind: "set", property: "background-color", value: "blue" },
      { kind: "remove", property: "background-color" },
      { kind: "set", property: "padding", value: "72px var(--space, var(--space," },
      { kind: "set", property: "padding-left", value: "5px" },
      { kind: "set", property: "height", value: "10px", priority: "bogus" },
      { kind: "set", property: "height", value: null },
    ] as const;

    batched.rule.style.applyMutations(operations);
    for (const operation of operations) {
      if (operation.kind === "remove") {
        sequential.rule.style.removeProperty(operation.property);
        continue;
      }
      sequential.rule.style.setProperty(
        operation.property,
        operation.value,
        "priority" in operation ? operation.priority : "",
      );
    }

    assert.equal(batched.rule.style.cssText, sequential.rule.style.cssText);
    assert.equal(batched.sheet.serialize(), sequential.sheet.serialize());
    assert.equal(batched.rule.style.length, sequential.rule.style.length);
    for (let index = 0; index < batched.rule.style.length; index += 1) {
      assert.equal(batched.rule.style.item(index), sequential.rule.style.item(index));
    }
  });

  it("returns rejection diagnostics even when the sheet queue is disabled", () => {
    const { rule, sheet } = styleRule();

    const [result] = rule.style.applyMutations([
      { kind: "set", property: "width", value: "invalid" },
    ]);

    assert.equal(result?.kind, "set");
    assert.equal(result?.kind === "set" && result.accepted, false);
    assert.equal(
      result?.kind === "set" && result.diagnostic?.code,
      "INVALID_PROPERTY_VALUE",
    );
    assert.deepEqual(sheet.takeDiagnostics(), []);
  });

  it("reports browser-recovered values as accepted", () => {
    const { rule } = styleRule();

    const [result] = rule.style.applyMutations([
      { kind: "set", property: "padding", value: "72px var(--space, var(--space," },
    ]);

    assert.deepEqual(result, { kind: "set", accepted: true, diagnostic: null });
    assert.equal(
      rule.style.getPropertyValue("padding"),
      "72px var(--space, var(--space,",
    );
  });

  it("validates the complete typed operation array before mutation", () => {
    const { rule } = styleRule();

    assert.throws(
      () => rule.style.applyMutations([
        { kind: "set", property: "width", value: "1px" },
        { kind: "set", property: "height", value: 2 as never },
      ]),
      TypeError,
    );
    assert.equal(rule.style.cssText, "");
    assert.throws(() => rule.style.applyMutations(null as never), TypeError);
    assert.throws(() => Reflect.apply(rule.style.applyMutations, rule.style, []), TypeError);
  });

  it("retains prior commits when a later resource limit throws", () => {
    const sheet = new CSSStyleSheet({
      resourceBudget: { maxDeclarationsPerBlock: 1 },
    });
    sheet.insertRule(".target {}");
    const rule = sheet.cssRules[0];
    assert.ok(rule instanceof CSSStyleRule);

    assert.throws(
      () => rule.style.applyMutations([
        { kind: "set", property: "width", value: "1px" },
        { kind: "set", property: "height", value: "2px" },
      ]),
      /SHEETOM_DECLARATION_LIMIT/u,
    );
    assert.equal(rule.style.cssText, "width: 1px;");
  });
});
