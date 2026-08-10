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
    source = `--x: ${"x".repeat(1024 * 1024)}`;
} else if (crashCase.nestingDepth) {
    source = `--x: ${"fn(".repeat(crashCase.nestingDepth)}value`;
} else if (crashCase.declarationCount) {
    source = "x:;".repeat(crashCase.declarationCount);
}

const execute = crashCase.mode === "rule"
    ? () => binding.parseStylesheetTreeJson(source, true)
    : () => binding.canonicalizeDeclarationBlock(source);

if (crashCase.expectError) {
    assert.throws(
        execute,
        error => error instanceof Error && error.message.includes(crashCase.expectError),
    );
} else {
    assert.equal(typeof execute(), "string");
}

assert.equal(binding.nativeEngineRevision(), "lightningcss-1.33.0-c6a0c3ce-sheetom.5");
assert.match(binding.canonicalizeDeclarationBlock("color: red"), /color:/u);
