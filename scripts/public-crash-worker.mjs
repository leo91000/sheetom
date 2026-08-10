import assert from "node:assert/strict";

import { CSSStyleRule, CSSStyleSheet } from "../dist/index.js";

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
    sheet.insertRule(".x {}");
    const rule = sheet.cssRules[0];
    assert.ok(rule instanceof CSSStyleRule);

    const separator = crashCase.source.indexOf(":");
    assert.notEqual(separator, -1, "crash case must contain a property delimiter");
    const property = crashCase.source.slice(0, separator).trim();
    const value = crashCase.source.slice(separator + 1).trim();
    if (crashCase.expectRejected) {
        rule.style.setProperty(property, "initial");
        const before = rule.style.cssText;
        rule.style.setProperty(property, value);
        assert.equal(rule.style.cssText, before, `${property} must reject atomically`);
        const serialized = sheet.serialize();
        const reparsed = new CSSStyleSheet();
        reparsed.replaceSync(serialized);
        assert.equal(reparsed.serialize(), serialized);
        process.exit(0);
    }
    rule.style.setProperty(property, value);

    assert.notEqual(rule.style.getPropertyValue(property), "", `${property} must survive the public API`);
    const serialized = sheet.serialize();
    const reparsed = new CSSStyleSheet();
    reparsed.replaceSync(serialized);
    assert.equal(reparsed.serialize(), serialized);
}
