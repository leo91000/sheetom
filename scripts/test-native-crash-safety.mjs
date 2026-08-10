import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { readdir } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const repositoryRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const nativeDirectory = path.join(repositoryRoot, "native");
const nativeArtifacts = (await readdir(nativeDirectory)).filter(
    entry => entry.startsWith("sheetom-native.") && entry.endsWith(".node"),
);

assert.equal(nativeArtifacts.length, 1, "expected exactly one local native artifact");

const artifactPath = path.join(nativeDirectory, nativeArtifacts[0]);
const workerPath = path.join(repositoryRoot, "scripts/native-crash-worker.mjs");
const crashCases = [
    {
        name: "background image-set",
        source: "background: image-set(url(a.png) 1x, url(b.png) 2x) center/cover no-repeat red",
    },
    {
        name: "mask image-set",
        source: "mask: image-set(url(a.png) 1x, url(b.png) 2x) center/cover no-repeat",
    },
    {
        name: "webkit mask image-set",
        source: "-webkit-mask: image-set(url(a.png) 1x, url(b.png) 2x) center/cover no-repeat",
    },
    {
        name: "multiple image-set layers",
        source: "background: image-set(url(a.png) 1x), image-set(url(b.png) 2x) center/contain no-repeat",
    },
    {
        name: "malformed pending substitution",
        source: "padding: 72px var(--space, var(--space,",
    },
    {
        name: "deep recovered functions",
        source: `--x: ${"func(".repeat(512)}value`,
        expectError: "SHEETOM_PARSE_ERROR",
    },
    {
        name: "nested functions below the parser limit",
        nestingDepth: 64,
    },
    {
        name: "top-level declaration delimiters",
        source: "width: 20px; color: red; background: url(data:image/svg+xml;utf8,<svg></svg>)",
    },
    {
        name: "declaration input budget",
        oversized: true,
        expectError: "SHEETOM_INPUT_LIMIT",
    },
    {
        name: "nesting at the resource limit remains process-safe",
        nestingDepth: 4096,
        expectError: "SHEETOM_PARSE_ERROR",
    },
    {
        name: "nesting above the supported limit",
        nestingDepth: 4097,
        expectError: "SHEETOM_NESTING_LIMIT",
    },
    {
        name: "declaration count above the supported limit",
        declarationCount: 100_001,
        expectError: "SHEETOM_DECLARATION_LIMIT",
    },
];

for (const crashCase of crashCases) {
    const encodedCase = Buffer.from(JSON.stringify(crashCase)).toString("base64url");
    const child = spawnSync(process.execPath, [workerPath, artifactPath, encodedCase], {
        encoding: "utf8",
        timeout: 15_000,
    });

    assert.equal(child.signal, null, `${crashCase.name} terminated with ${child.signal ?? "no signal"}`);
    assert.equal(
        child.status,
        0,
        `${crashCase.name} exited ${child.status}\nstdout:\n${child.stdout}\nstderr:\n${child.stderr}`,
    );
}

console.log(`${crashCases.length} native crash-safety subprocesses passed.`);
