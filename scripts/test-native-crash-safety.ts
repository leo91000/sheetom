import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import { readFile, readdir, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const repositoryRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const contractSha256 = createHash("sha256")
    .update(await readFile(fileURLToPath(import.meta.url)))
    .digest("hex");
const nativeDirectory = path.join(repositoryRoot, "native");
const nativeArtifacts = (await readdir(nativeDirectory)).filter(
    entry => entry.startsWith("sheetom-native.") && entry.endsWith(".node"),
);

assert.equal(nativeArtifacts.length, 1, "expected exactly one local native artifact");

const artifactPath = path.join(nativeDirectory, nativeArtifacts[0]);
const workerPath = path.join(repositoryRoot, "scripts/native-crash-worker.ts");
const publicWorkerPath = path.join(repositoryRoot, "scripts/public-crash-worker.ts");
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
        name: "cursor image-set with hotspot",
        source: "cursor: image-set(url(a.png) 1x, url(b.png) 2x type(\"image/png\")) 1 1, auto",
        public: true,
    },
    {
        name: "modern clip path SVG path and geometry box",
        source: "clip-path: content-box path(evenodd, \"M0 0 L10 10\")",
        public: true,
    },
    {
        name: "multiple image-set layers",
        source: "background: image-set(url(a.png) 1x), image-set(url(b.png) 2x) center/contain no-repeat",
        public: true,
    },
    {
        name: "webkit mask box image-set shorthand",
        source: "-webkit-mask-box-image: image-set(url(a.png) 1x, url(b.png) 2x) repeat 1 fill / auto / 2px",
        expectedEmptyGetter: true,
        public: true,
    },
    {
        name: "webkit mask box image-set source longhand",
        source: "-webkit-mask-box-image-source: image-set(url(a.png) 1x, url(b.png) 2x)",
        public: true,
    },
    {
        name: "transform origin depth calculation",
        source: "transform-origin: calc(1px + 2%) center calc(3px + 4px)",
        public: true,
    },
    {
        name: "columns calculated height",
        source: "columns: min(10em, 50vw) 3 / max(10px, calc(20px + 5vh))",
        public: true,
    },
    {
        name: "background level four compound clip",
        source: "background: image-set(url(a.png) 1x, url(b.png) 2x) center/cover border-area text",
        public: true,
    },
    {
        name: "border image fill with context dependent math",
        source: "border-image: image-set(url(a.png) 1x, url(b.png) 2x) sign(1em) fill / 2 / 3 round",
        public: true,
    },
    {
        name: "corner shape non finite exponents",
        source: "corner-shape: superellipse(infinity) superellipse(-infinity) superellipse(calc(NaN))",
        public: true,
    },
    {
        name: "nested calc size with anchor math and recovered tail",
        source: "width: calc-size(calc-size(anchor-size(width), size / 2 + 1px), min(size, 10px), ignored tokens)",
        public: true,
    },
    {
        name: "text fit deferred percentage math",
        source: "text-fit: grow per-line-all min(calc(-1%), 25%)",
        public: true,
    },
    {
        name: "initial letter context dependent sink math",
        source: "initial-letter: calc(sign(1em) + 2) sign(1rem)",
        public: true,
    },
    {
        name: "white space unordered level four components",
        source: "white-space: nowrap preserve-breaks",
        public: true,
    },
    {
        name: "contain intrinsic compound calculated axes",
        source: "contain-intrinsic-size: auto calc(sign(1em) * 1px) auto calc(-1px)",
        public: true,
    },
    {
        name: "math depth nested integer calculation",
        source: "math-depth: add(calc(round(down, pow(2, 8) / 3, 1)))",
        public: true,
    },
    {
        name: "text box unordered paired edge",
        source: "text-box: cap alphabetic trim-end",
        public: true,
    },
    {
        name: "text decoration error with typed components",
        source: "text-decoration: spelling-error calc(1px + 1em) wavy contrast-color(red)",
        public: true,
    },
    {
        name: "compound container type in shorthand",
        source: "container: card / scroll-state inline-size",
        public: true,
    },
    {
        name: "recursive font palette mixing",
        source: "font-palette: palette-mix(in oklch longer hue, --brand 10%, palette-mix(in srgb-linear, normal, dark calc(-1%)))",
        public: true,
    },
    {
        name: "rule partition intersection",
        source: "rule-break: intersection",
        public: true,
    },
    {
        name: "rule visibility around",
        source: "rule-visibility-items: around",
        public: true,
    },
    {
        name: "overscroll chaining pair",
        source: "overscroll-behavior: chain contain",
        public: true,
    },
    {
        name: "scroll timeline mixed none and name list",
        source: "scroll-timeline: none inline, --x x",
        public: true,
    },
    {
        name: "view timeline mixed none and name list",
        source: "view-timeline: none 10% 20%, --x inline auto",
        public: true,
    },
    {
        name: "anchored self alignment",
        source: "place-self: safe anchor-center unsafe anchor-center",
        public: true,
    },
    {
        name: "bare legacy item alignment",
        source: "place-items: normal legacy",
        public: true,
    },
    {
        name: "hyphenate limit mixed calculation triple",
        source: "hyphenate-limit-chars: calc(1 + 1) auto calc(.5)",
        public: true,
    },
    {
        name: "font math size shorthand",
        source: "font: normal math / normal \"sheetom\"",
        public: true,
    },
    {
        name: "large SVG path command sequence",
        mode: "geometric-svg-path",
        repeatCount: 50_000,
        public: true,
    },
    {
        name: "large CSS shape command sequence",
        mode: "geometric-shape-commands",
        repeatCount: 10_000,
        public: true,
    },
    {
        name: "large polygon point sequence",
        mode: "geometric-polygon-points",
        repeatCount: 20_000,
        public: true,
    },
    {
        name: "nested gradient color functions",
        mode: "geometric-nested-gradient",
        nestingDepth: 128,
        public: true,
    },
    {
        name: "malformed SVG path recovery",
        source: 'd: path("M0 0 L1")',
        expectRejected: true,
        public: true,
    },
    {
        name: "geometric value above the declaration input budget",
        mode: "geometric-oversized",
        expectError: "SHEETOM_INPUT_LIMIT",
        expectPublicError: "SHEETOM_INPUT_LIMIT",
        public: true,
    },
    {
        name: "font stretch incompatible percentage calc",
        source: "font-stretch: calc(1 + 1)",
        public: true,
        expectRejected: true,
    },
    {
        name: "text size adjust incompatible percentage calc",
        source: "text-size-adjust: calc(1 + 1)",
        public: true,
        expectRejected: true,
    },
    {
        name: "webkit text size adjust incompatible percentage calc",
        source: "-webkit-text-size-adjust: calc(1 + 1)",
        public: true,
        expectRejected: true,
    },
    {
        name: "dimensionless length calculation",
        source: "width: calc(1px + 1)",
        public: true,
        expectRejected: true,
    },
    {
        name: "non-finite dimension calculation",
        source: "width: calc(infinity * 1px)",
        public: true,
    },
    {
        name: "cross-dimension static sign calculation",
        source: "opacity: sign(1px)",
        public: true,
    },
    {
        name: "negative math result remains reparsable",
        source: "width: rem(-5px, 2px)",
        public: true,
    },
    {
        name: "anchor-size shorthand expansion",
        source: "margin: anchor-size(width)",
        public: true,
    },
    {
        name: "timeline range expansion",
        source: "timeline-trigger: scroll",
        public: true,
    },
    {
        name: "position try area expansion",
        source: "position-try: most-width left top",
        public: true,
    },
    {
        name: "balanced flex flow expansion",
        source: "flex-flow: wrap-reverse balance",
        public: true,
    },
    {
        name: "webkit mask percentage shorthand expansion",
        source: "-webkit-mask-box-image: 10%",
    },
    {
        name: "context-dependent sign calculation",
        source: "opacity: sign(calc(1px - 2em))",
        public: true,
    },
    {
        name: "context-dependent dynamic product",
        source: "opacity: calc(sign(1em) * sign(1rem))",
        public: true,
    },
    {
        name: "context-dependent dynamic quotient",
        source: "opacity: calc(sign(1em) / sign(1rem))",
        public: true,
    },
    {
        name: "invalid dimension product",
        source: "opacity: calc(1px * 1em)",
        public: true,
        expectRejected: true,
    },
    {
        name: "mixed word spacing calculation",
        source: "word-spacing: min(1px, 2%)",
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
        name: "custom property recovery at the resource boundary",
        mode: "declaration-state",
        source: `${"fn(".repeat(4096)}value`,
    },
    {
        name: "public custom property recovery at the resource boundary",
        source: `--x: ${"fn(".repeat(4095)}value`,
        public: true,
        publicOnly: true,
    },
    {
        name: "dynamic range mix at the resource limit remains process-safe",
        mode: "dynamic-range-depth",
        nestingDepth: 4096,
        expectError: "SHEETOM_PARSE_ERROR",
        expectPublicError: "SHEETOM_NESTING_LIMIT",
        public: true,
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
        name: "custom function conditional nesting at the resource boundary",
        mode: "recovered-rule",
        source: `@function --nested() { ${"@media all{".repeat(4094)}result: 1;${"}".repeat(4094)} }`,
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
        name: "nested style rule parser at the resource boundary",
        mode: "recovered-rule",
        source: `${".x{".repeat(4095)}color:red${"}".repeat(4095)}`,
    },
    {
        name: "stylesheet rule parser at the resource boundary",
        mode: "rule",
        source: `${"@media all{".repeat(4095)}.x{color:red}${"}".repeat(4095)}`,
    },
    {
        name: "selector normalization",
        mode: "selector",
        source: ':is(.a, [data-value="a;b"], :not(.c)) > .child',
    },
    {
        name: "selector normalization at the resource boundary",
        mode: "selector",
        source: `${":is(".repeat(4096)}.x${")".repeat(4096)}`,
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

    assert.equal(
        child.error,
        undefined,
        `${crashCase.name} could not start or complete: ${child.error?.message ?? "unknown spawn error"}`,
    );
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

    assert.equal(
        child.error,
        undefined,
        `public ${crashCase.name} could not start or complete: ${child.error?.message ?? "unknown spawn error"}`,
    );
    assert.equal(child.signal, null, `public ${crashCase.name} terminated with ${child.signal ?? "no signal"}`);
    assert.equal(
        child.status,
        0,
        `public ${crashCase.name} exited ${child.status}\nstdout:\n${child.stdout}\nstderr:\n${child.stderr}`,
    );
}

const reportArgument = process.argv.find(argument => argument.startsWith("--report="));
if (reportArgument) {
    const reportPath = path.resolve(reportArgument.slice("--report=".length));
    await writeFile(reportPath, `${JSON.stringify({
        schemaVersion: 1,
        contractSha256,
        native: { passed: nativeCrashCases.length, total: nativeCrashCases.length },
        public: { passed: publicCrashCases.length, total: publicCrashCases.length },
    }, null, 2)}\n`);
}

console.log(
    `${nativeCrashCases.length} native and ${publicCrashCases.length} public crash-safety subprocesses passed.`,
);
