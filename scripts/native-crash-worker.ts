import assert from "node:assert/strict";
import { createRequire } from "node:module";
import path from "node:path";
import { fileURLToPath } from "node:url";

import { readNativeEngineRevision } from "./native-engine-revision.ts";
import { materializeCrashSource } from "./crash-case-source.ts";

const repositoryRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const expectedEngineRevision = await readNativeEngineRevision(repositoryRoot);

const [artifactPath, encodedCase] = process.argv.slice(2);
assert.ok(artifactPath, "native artifact path is required");
assert.ok(encodedCase, "encoded crash case is required");

const crashCase = JSON.parse(Buffer.from(encodedCase, "base64url").toString("utf8"));
const require = createRequire(import.meta.url);
const binding = require(artifactPath);
const source = materializeCrashSource(crashCase);

let execute = () => binding.canonicalizeDeclarationBlock(source);
if (crashCase.mode === "declaration-state" || crashCase.mode === "geometric-oversized") {
    execute = () => {
        const state = new binding.NativeDeclarationState("style");
        if (crashCase.mode === "geometric-oversized") {
            const separator = source.indexOf(":");
            return state.setProperty(source.slice(0, separator), source.slice(separator + 1), "");
        }
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

assert.equal(binding.nativeEngineRevision(), expectedEngineRevision);
assert.match(binding.canonicalizeDeclarationBlock("color: red"), /color:/u);
