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
const publicWorkerPath = path.join(repositoryRoot, "scripts/public-crash-worker.mjs");
const crashCases = [
    {
        name: "background image-set",
        source: "background: image-set(url(a.png) 1x, url(b.png) 2x) center/cover no-repeat red",
        public: true,
    },
    {
        name: "mask image-set",
        source: "mask: image-set(url(a.png) 1x, url(b.png) 2x) center/cover no-repeat",
        public: true,
    },
    {
        name: "webkit mask image-set",
        source: "-webkit-mask: image-set(url(a.png) 1x, url(b.png) 2x) center/cover no-repeat",
        public: true,
    },
    {
        name: "multiple image-set layers",
        source: "background: image-set(url(a.png) 1x), image-set(url(b.png) 2x) center/contain no-repeat",
        public: true,
    },
    {
        name: "malformed pending substitution",
        source: "padding: 72px var(--space, var(--space,",
        public: true,
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
        mode: "declaration-state",
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
        name: "public stylesheet at the resource boundary",
        mode: "stylesheet-resource",
        nestingDepth: 4095,
        public: true,
        publicOnly: true,
    },
    {
        name: "public stylesheet above the resource boundary",
        mode: "stylesheet-resource",
        nestingDepth: 4096,
        expectError: "SHEETOM_NESTING_LIMIT",
        public: true,
        publicOnly: true,
    },
    {
        name: "declaration count above the supported limit",
        declarationCount: 100_001,
        expectError: "SHEETOM_DECLARATION_LIMIT",
    },
    {
        name: "rule parser image-set",
        mode: "rule",
        source: '.x { background: image-set(url("a.png") 1x, url("b.png") 2x) center/cover no-repeat red; }',
    },
    {
        name: "rule parser recovered stylesheet",
        mode: "recovered-rule",
        source: "@media screen { .x { padding: 72px var(--space, var(--space,; } }",
    },
    {
        name: "custom function image-set parameter and result",
        mode: "recovered-rule",
        source: "@function --image(--value <image>: image-set(url(a.png) 1x, url(b.png) 2x)) returns <image> { --local: foo(a;b); result: var(--value); }",
    },
    {
        name: "custom function nested component blocks",
        mode: "recovered-rule",
        source: "@function --blocks() { --fn: foo(a;b); --square: [a;b]; --curly: {a;b} tail; --choice: if(style(--x: 1): red; else: blue); result: ok; }",
    },
    {
        name: "custom function conditional nesting below the rule limit",
        mode: "recovered-rule",
        source: `@function --nested() { ${"@supports (display:grid) {".repeat(64)}result: 1;${"}".repeat(64)} }`,
    },
    {
        name: "single recovered function group keeps native values process-safe",
        mode: "recovered-single-rule",
        source: "@media(width:1px){result:image-set(url(a.png) 1x, url(b.png) 2x);}",
    },
    {
        name: "single recovered function group rejects trailing tokens safely",
        mode: "recovered-single-rule",
        source: "@media(width:1px){result:1px;} color:red",
        expectError: "SHEETOM_PARSE_ERROR",
    },
    {
        name: "recovered rule parser nesting formerly below the RC5 implementation cap",
        mode: "recovered-rule",
        source: `${"@media all{".repeat(257)}.x{color:red}${"}".repeat(257)}`,
    },
    {
        name: "recovered rule parser at the resource boundary",
        mode: "recovered-rule",
        source: `${"@media all{".repeat(4095)}.x{color:red}${"}".repeat(4095)}`,
    },
    {
        name: "selector normalization",
        mode: "selector",
        source: ':is(.a, [data-value="a;b"], :not(.c)) > .child',
    },
    {
        name: "media normalization",
        mode: "media",
        source: "screen/**/and (max-width:767px),print",
    },
    {
        name: "supports normalization",
        mode: "supports",
        source: "(display:grid) and (not (color:contrast-color(red)))",
    },
    {
        name: "container prelude parsing",
        mode: "container",
        source: "card style(--theme:dark)",
    },
    {
        name: "scope prelude parsing",
        mode: "scope",
        source: "(.a, :is(.b, .c)) to (.d)",
    },
    {
        name: "symbols",
        mode: "counter-descriptor",
        source: `"a" ${"fn(".repeat(64)}value`,
    },
    {
        name: "counter descriptor declaration recovery",
        mode: "counter-descriptors",
        source: 'system: fixed; symbols: "a;b"; range: 10 1; suffix: "}";',
    },
    {
        name: "counter name escapes",
        mode: "counter-name",
        source: "\\78",
    },
    {
        name: "symbols",
        mode: "counter-descriptor",
        source: `${"fn(".repeat(4097)}value`,
        expectError: "SHEETOM_NESTING_LIMIT",
    },
    {
        name: "arbitrary identifier serialization",
        mode: "identifier",
        source: "123 bad\u0000name",
    },
    {
        name: "font family setter serialization",
        mode: "font-family",
        source: '"A,B", var(--family), serif, é',
    },
    {
        name: "rule parser nesting above the supported limit",
        mode: "rule",
        source: `${"@media all{".repeat(4097)}.x{color:red}${"}".repeat(4097)}`,
        expectError: "SHEETOM_NESTING_LIMIT",
    },
];

const nativeCrashCases = crashCases.filter(candidate => !candidate.publicOnly);
for (const crashCase of nativeCrashCases) {
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

const publicCrashCases = crashCases.filter(candidate => candidate.public);
for (const crashCase of publicCrashCases) {
    const encodedCase = Buffer.from(JSON.stringify(crashCase)).toString("base64url");
    const child = spawnSync(process.execPath, [publicWorkerPath, encodedCase], {
        encoding: "utf8",
        timeout: 15_000,
    });

    assert.equal(child.signal, null, `public ${crashCase.name} terminated with ${child.signal ?? "no signal"}`);
    assert.equal(
        child.status,
        0,
        `public ${crashCase.name} exited ${child.status}\nstdout:\n${child.stdout}\nstderr:\n${child.stderr}`,
    );
}

console.log(
    `${nativeCrashCases.length} native and ${publicCrashCases.length} public crash-safety subprocesses passed.`,
);
