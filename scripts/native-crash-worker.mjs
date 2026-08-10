import assert from "node:assert/strict";
import { createRequire } from "node:module";

const [artifactPath, encodedCase] = process.argv.slice(2);
assert.ok(artifactPath, "native artifact path is required");
assert.ok(encodedCase, "encoded crash case is required");

const crashCase = JSON.parse(Buffer.from(encodedCase, "base64url").toString("utf8"));
const require = createRequire(import.meta.url);
const binding = require(artifactPath);
let source = crashCase.source;
if (crashCase.oversized) {
    source = "x".repeat((1024 * 1024) + 1);
} else if (crashCase.nestingDepth) {
    source = `--x: ${"fn(".repeat(crashCase.nestingDepth)}value`;
} else if (crashCase.declarationCount) {
    source = "x:;".repeat(crashCase.declarationCount);
}

let execute = () => binding.canonicalizeDeclarationBlock(source);
if (crashCase.mode === "declaration-state") {
    execute = () => {
        const state = new binding.NativeDeclarationState("style");
        return state.setProperty("--x", source, "");
    };
}
if (crashCase.mode === "rule") execute = () => binding.parseStylesheetTreeJson(source, true);
if (crashCase.mode === "recovered-rule") {
    execute = () => binding.parseRecoveredRuleTreeJson(source);
}
if (crashCase.mode === "recovered-single-rule") {
    execute = () => binding.parseRecoveredSingleRuleTreeJson(source);
}
if (crashCase.mode === "selector") execute = () => binding.normalizeSelector(source);
if (crashCase.mode === "media") execute = () => binding.normalizeMedia(source);
if (crashCase.mode === "supports") execute = () => binding.normalizeSupports(source);
if (crashCase.mode === "container") {
    execute = () => binding.parseContainerPreludeJson(source);
}
if (crashCase.mode === "scope") execute = () => binding.parseScopePreludeJson(source);
if (crashCase.mode === "counter-descriptor") {
    execute = () => binding.parseCounterStyleDescriptorValue(crashCase.name, source) ?? "";
}
if (crashCase.mode === "counter-descriptors") {
    execute = () => binding.parseCounterStyleDescriptorsJson(source);
}
if (crashCase.mode === "counter-name") {
    execute = () => binding.parseCounterStyleNameJson(source) ?? "";
}
if (crashCase.mode === "identifier") execute = () => binding.serializeIdentifierValue(source);
if (crashCase.mode === "font-family") execute = () => binding.serializeFontFamilyValue(source);

if (crashCase.expectError) {
    assert.throws(
        execute,
        error => error instanceof Error && error.message.includes(crashCase.expectError),
    );
} else {
    assert.equal(typeof execute(), "string");
}

assert.equal(binding.nativeEngineRevision(), "lightningcss-1.33.0-c6a0c3ce-sheetom.11");
assert.match(binding.canonicalizeDeclarationBlock("color: red"), /color:/u);
