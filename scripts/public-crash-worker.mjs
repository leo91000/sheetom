import assert from "node:assert/strict";

import { CSSStyleRule, CSSStyleSheet } from "../dist/index.js";

const [encodedCase] = process.argv.slice(2);
assert.ok(encodedCase, "encoded crash case is required");

const crashCase = JSON.parse(Buffer.from(encodedCase, "base64url").toString("utf8"));
const sheet = new CSSStyleSheet();
sheet.insertRule(".x {}");
const rule = sheet.cssRules[0];
assert.ok(rule instanceof CSSStyleRule);

const separator = crashCase.source.indexOf(":");
assert.notEqual(separator, -1, "crash case must contain a property delimiter");
const property = crashCase.source.slice(0, separator).trim();
const value = crashCase.source.slice(separator + 1).trim();
rule.style.setProperty(property, value);

assert.notEqual(rule.style.getPropertyValue(property), "", `${property} must survive the public API`);
const serialized = sheet.serialize();
const reparsed = new CSSStyleSheet();
reparsed.replaceSync(serialized);
assert.equal(reparsed.serialize(), serialized);
