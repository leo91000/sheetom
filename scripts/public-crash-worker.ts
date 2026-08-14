import assert from "node:assert/strict";

import { CSSStyleRule, CSSStyleSheet } from "../dist/index.js";
import { materializeCrashSource } from "./crash-case-source.ts";

function createStyleRule(): { sheet: CSSStyleSheet; rule: CSSStyleRule } {
    const sheet = new CSSStyleSheet();
    sheet.insertRule(".x {}");
    const rule = sheet.cssRules[0];
    assert.ok(rule instanceof CSSStyleRule);
    return { sheet, rule };
}

function assertRoundTrip(sheet: CSSStyleSheet): void {
    const serialized = sheet.serialize();
    const reparsed = new CSSStyleSheet();
    reparsed.replaceSync(serialized);
    assert.equal(reparsed.serialize(), serialized);
}

const [encodedCase] = process.argv.slice(2);
assert.ok(encodedCase, "encoded crash case is required");

const crashCase = JSON.parse(Buffer.from(encodedCase, "base64url").toString("utf8"));
const sheet = new CSSStyleSheet();
if (crashCase.mode === "stylesheet-resource") {
    sheet.replaceSync(".old { color: red; }");
    const previousRule = sheet.cssRules[0];
    const nestingDepth = crashCase.nestingDepth;
    const source = `${"@media all{".repeat(nestingDepth)}.x{color:red}${"}".repeat(nestingDepth)}`;
    let thrown;
    try {
        sheet.replaceSync(source);
    } catch (error) {
        thrown = error;
    }

    if (crashCase.expectError) {
        assert.ok(thrown instanceof RangeError);
        assert.match(thrown.message, new RegExp(crashCase.expectError, "u"));
    } else {
        assert.equal(thrown, undefined, "input at the advertised boundary must remain usable");
        assert.equal(sheet.cssRules.length, 1);
        assert.notEqual(sheet.cssRules[0], previousRule);
        const ruleCssText = sheet.cssRules[0]?.cssText;
        assert.match(ruleCssText, /color: red/u);
        const serialized = sheet.serialize();
        assert.match(serialized, /color: red/u);
    }
    if (thrown !== undefined) {
        assert.equal(sheet.cssRules.length, 1);
        assert.equal(sheet.cssRules[0], previousRule);
    }
} else {
    const { sheet: declarationSheet, rule } = createStyleRule();

    const source = materializeCrashSource(crashCase);
    const separator = source.indexOf(":");
    assert.notEqual(separator, -1, "crash case must contain a property delimiter");
    const property = source.slice(0, separator).trim();
    const value = source.slice(separator + 1).trim();
    if (crashCase.expectPublicError) {
        rule.style.setProperty(property, "initial");
        const before = rule.style.cssText;
        assert.throws(
            () => rule.style.setProperty(property, value),
            error => error instanceof RangeError
                && error.message.includes(crashCase.expectPublicError),
        );
        assert.equal(rule.style.cssText, before, `${property} must reject atomically`);

        const { rule: batchedRule } = createStyleRule();
        batchedRule.style.setProperty(property, "initial");
        const batchedBefore = batchedRule.style.cssText;
        assert.throws(
            () => batchedRule.style.applyMutations([
                { kind: "set", property, value },
            ]),
            error => error instanceof RangeError
                && error.message.includes(crashCase.expectPublicError),
        );
        assert.equal(batchedRule.style.cssText, batchedBefore, `${property} batch must reject atomically`);
        process.exit(0);
    }
    if (crashCase.expectRejected) {
        rule.style.setProperty(property, "initial");
        const before = rule.style.cssText;
        rule.style.setProperty(property, value);
        assert.equal(rule.style.cssText, before, `${property} must reject atomically`);
        assertRoundTrip(declarationSheet);

        const { sheet: batchedSheet, rule: batchedRule } = createStyleRule();
        batchedRule.style.setProperty(property, "initial");
        const batchedBefore = batchedRule.style.cssText;
        const [result] = batchedRule.style.applyMutations([
            { kind: "set", property, value },
        ]);
        assert.equal(result?.kind, "set");
        assert.equal(result.accepted, false);
        assert.equal(result.diagnostic?.code, "INVALID_PROPERTY_VALUE");
        assert.equal(result.diagnostic?.operation, "setProperty");
        assert.equal(result.diagnostic?.property, property);
        assert.equal(result.diagnostic?.input, value);
        assert.equal(batchedRule.style.cssText, batchedBefore, `${property} batch must reject atomically`);
        assertRoundTrip(batchedSheet);
        process.exit(0);
    }
    rule.style.setProperty(property, value);

    if (crashCase.expectedEmptyGetter) {
        assert.equal(rule.style.getPropertyValue(property), "");
        assert.ok(rule.style.length > 0, `${property} must expand through the public API`);
    } else {
        assert.notEqual(rule.style.getPropertyValue(property), "", `${property} must survive the public API`);
    }
    assertRoundTrip(declarationSheet);

    const { sheet: batchedSheet, rule: batchedRule } = createStyleRule();
    const [result] = batchedRule.style.applyMutations([
        { kind: "set", property, value },
    ]);
    assert.deepEqual(result, { kind: "set", accepted: true, diagnostic: null });
    if (crashCase.expectedEmptyGetter) {
        assert.equal(batchedRule.style.getPropertyValue(property), "");
        assert.ok(batchedRule.style.length > 0, `${property} must expand through the batched API`);
    } else {
        assert.notEqual(
            batchedRule.style.getPropertyValue(property),
            "",
            `${property} must survive the batched API`,
        );
    }
    assertRoundTrip(batchedSheet);
}
