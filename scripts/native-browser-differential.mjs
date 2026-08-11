import assert from "node:assert/strict";
import { createRequire } from "node:module";
import { readFile, readdir } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

import { chromium } from "playwright";

const repositoryRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const nativeDirectory = path.join(repositoryRoot, "native");
const [nativeArtifact] = (await readdir(nativeDirectory)).filter(
    entry => entry.startsWith("sheetom-native.") && entry.endsWith(".node"),
);
assert.ok(nativeArtifact, "build the native addon before running the differential");
const require = createRequire(import.meta.url);
const binding = require(path.join(nativeDirectory, nativeArtifact));
const relativeColorCorpus = JSON.parse(await readFile(
    path.join(repositoryRoot, "compatibility/relative-color-capabilities.json"),
    "utf8",
));
const valueCapabilityCorpus = JSON.parse(await readFile(
    path.join(repositoryRoot, "compatibility/value-capabilities.json"),
    "utf8",
));
const numberResultMathCorpus = JSON.parse(await readFile(
    path.join(repositoryRoot, "compatibility/number-result-math-capabilities.json"),
    "utf8",
));

const cases = [
    {
        id: "replacement-winners",
        operations: [["replace", "width: 1px !important; color: red; width: 2px; height: 3px !important;"]],
        probes: ["color", "width", "height"],
    },
    {
        id: "pending-shorthand",
        operations: [["set", "padding", "72px var(--space, var(--space,", ""]],
        probes: ["padding", "padding-top", "padding-left"],
    },
    {
        id: "broken-pending-group",
        operations: [
            ["set", "padding", "var(--space)", "important"],
            ["set", "padding-left", "3px", ""],
        ],
        probes: ["padding", "padding-top", "padding-left"],
    },
    {
        id: "shorthand-remove",
        operations: [
            ["set", "overflow", "hidden auto", ""],
            ["set", "overflow-x", "scroll", ""],
            ["remove", "overflow-x"],
        ],
        probes: ["overflow", "overflow-x", "overflow-y"],
    },
    {
        id: "intrinsic-flex-basis-calc-size",
        operations: [[
            "set",
            "flex-basis",
            "calc-size(auto, size / 2 + 1px)",
            "important",
        ]],
        probes: ["flex-basis"],
    },
    {
        id: "intrinsic-flex-shorthand-order",
        operations: [["set", "flex", "content 2 3", "important"]],
        probes: ["flex", "flex-grow", "flex-shrink", "flex-basis"],
    },
    {
        id: "intrinsic-webkit-flex-shorthand",
        operations: [["set", "-webkit-flex", "2 stretch", ""]],
        probes: ["-webkit-flex", "flex", "flex-grow", "flex-shrink", "flex-basis"],
    },
    {
        id: "intrinsic-flex-shorthand-remove",
        operations: [
            ["set", "flex", "2 3 max-content", "important"],
            ["set", "flex-basis", "content", "important"],
            ["remove", "flex-basis"],
        ],
        probes: ["flex", "flex-grow", "flex-shrink", "flex-basis"],
    },
    {
        id: "invalid-flex-calc-size-is-atomic",
        operations: [
            ["set", "flex", "0 0 auto", "important"],
            ["set", "flex", "1 1 calc-size(auto, size)", ""],
        ],
        probes: ["flex", "flex-grow", "flex-shrink", "flex-basis"],
    },
    {
        id: "invalid-flex-basis-calc-size-is-atomic",
        operations: [
            ["set", "flex-basis", "content", "important"],
            ["set", "flex-basis", "calc-size(any, size)", ""],
        ],
        probes: ["flex-basis"],
    },
    {
        id: "rule-inset-canonicalization",
        operations: [[
            "set",
            "rule-inset",
            "calc(10px + 5%) -2px / overlap-join 4%",
            "important",
        ]],
        probes: [
            "rule-inset",
            "row-rule-inset-cap-start",
            "column-rule-inset-junction-end",
        ],
    },
    {
        id: "rule-inset-reduced-math-canonicalization",
        operations: [["set", "rule-inset", "min(1px, 2px)", ""]],
        probes: ["rule-inset", "column-rule-inset", "row-rule-inset"],
    },
    {
        id: "invalid-rule-inset-is-atomic",
        operations: [
            ["set", "rule-inset", "1px 2px / 3px 4px", "important"],
            ["set", "rule-inset", "1px / 2px / 3px", "important"],
        ],
        probes: ["rule-inset", "row-rule-inset-cap-start"],
    },
    {
        id: "pending-rule-inset",
        operations: [["set", "rule-inset", "var(--inset)", "important"]],
        probes: ["rule-inset", "row-rule-inset-cap-start"],
    },
    {
        id: "anchor-inset-canonicalization",
        operations: [[
            "set",
            "top",
            "anchor(inside --sheetom, calc(anchor-size(width) + 1px))",
            "important",
        ]],
        probes: ["top"],
    },
    {
        id: "recursive-anchor-inset-fallback",
        operations: [[
            "set",
            "top",
            "anchor(inside, anchor(outside, calc(anchor-size(width) + 1px)))",
            "",
        ]],
        probes: ["top"],
    },
    {
        id: "anchor-inset-shorthand-remove",
        operations: [
            [
                "set",
                "inset",
                "anchor(inside) anchor(--sheetom outside, 1px) calc(anchor(start) + 1px) anchor(20%)",
                "important",
            ],
            ["set", "top", "2px", ""],
            ["remove", "top"],
        ],
        probes: ["inset", "top", "right", "bottom", "left"],
    },
    {
        id: "invalid-anchor-inset-is-atomic",
        operations: [
            ["set", "top", "anchor(inside)", "important"],
            ["set", "top", "anchor(inside, calc(anchor-size(width) + 1s))", ""],
        ],
        probes: ["top"],
    },
    {
        id: "pending-anchor-inset-shorthand",
        operations: [[
            "set",
            "inset",
            "anchor(inside, var(--fallback))",
            "important",
        ]],
        probes: ["inset", "top", "right", "bottom", "left"],
    },
    {
        id: "escaped-custom-name",
        operations: [["set", "--foo:bar", "red", ""]],
        probes: ["--foo:bar"],
    },
    {
        id: "css-whitespace-does-not-strip-nbsp",
        operations: [[
            "replace",
            "--x: red ; width: 10px; width: 1px ; font-family: A ;",
        ]],
        probes: ["--x", "width", "font-family"],
    },
    {
        id: "custom-function-substitution",
        operations: [["set", "width", "calc(--double(1px) + 1px)", ""]],
        probes: ["width"],
    },
    {
        id: "custom-function-pending-shorthand",
        operations: [["set", "padding", "--spacing(1px, 2px)", "important"]],
        probes: ["padding", "padding-top", "padding-left"],
    },
    {
        id: "invalid-custom-function-is-atomic",
        operations: [
            ["set", "width", "10px", ""],
            ["set", "width", "--double(1px; 2px)", ""],
        ],
        probes: ["width"],
    },
];

for (const longhand of [
    "column-rule-inset-cap-start",
    "column-rule-inset-cap-end",
    "column-rule-inset-junction-start",
    "column-rule-inset-junction-end",
    "row-rule-inset-cap-start",
    "row-rule-inset-cap-end",
    "row-rule-inset-junction-start",
    "row-rule-inset-junction-end",
]) {
    cases.push({
        id: `rule-inset-remove:${longhand}`,
        operations: [
            ["set", "rule-inset", "1px 2px / 3px 4px", "important"],
            ["set", longhand, "5px", "important"],
            ["remove", longhand],
        ],
        probes: ["rule-inset", "column-rule-inset", "row-rule-inset"],
    });
}

const customFunctionNames = ["--f", "---f", "--\\66", "--"];
const customFunctionArguments = [
    "",
    "a",
    ",a",
    "a,b",
    "{a,b}",
    "[a,b]",
    "foo(a;b)",
    "foo(!)",
    "a,,b",
    "a,",
    "a;",
    "!",
    '"a;b"',
    "a/*x*/,b",
];
const customFunctionContexts = [
    value => value,
    value => `calc(${value} + 1px)`,
    value => `min(${value}, 1px)`,
];
for (const name of customFunctionNames) {
    for (const argument of customFunctionArguments) {
        for (const context of customFunctionContexts) {
            const value = context(`${name}(${argument})`);
            cases.push({
                id: `custom-function-call:${value}`,
                operations: [
                    ["set", "width", "10px", ""],
                    ["set", "width", value, ""],
                ],
                probes: ["width"],
            });
        }
    }
}

for (const value of [
    "a/*c*/b",
    "/*c*/a",
    "a/*c*/",
    "var(--x/*c*/)",
    "foo(a/*c*/b)",
    "a/*c",
    " red ",
    " ",
]) {
    cases.push({
        id: `custom-property-comment:${value}`,
        operations: [["set", "--x", value, ""]],
        probes: ["--x"],
    });
}

for (const recoveryCase of [
    { id: "font-string", property: "font-family", initial: "serif", input: '"Gotham' },
    { id: "typed-comment", property: "color", initial: "blue", input: "red/*comment" },
    {
        id: "calc-internal-comment",
        property: "width",
        initial: "2px",
        input: "calc(1px/*comment*/",
    },
    {
        id: "color-internal-comment",
        property: "color",
        initial: "blue",
        input: "rgb(1/*comment*/ 2 3",
    },
    {
        id: "gradient-internal-comment",
        property: "background-image",
        initial: "none",
        input: "linear-gradient(red/*comment*/, blue",
    },
    { id: "calc-function", property: "width", initial: "2px", input: "calc(1px" },
    { id: "min-function", property: "width", initial: "2px", input: "min(1px, 2px" },
    { id: "color-function", property: "color", initial: "blue", input: "rgb(1 2 3" },
    {
        id: "gradient-function",
        property: "background-image",
        initial: "none",
        input: "linear-gradient(red, blue",
    },
    {
        id: "transform-function",
        property: "transform",
        initial: "none",
        input: "translateX(1px",
    },
    { id: "url-token", property: "background-image", initial: "none", input: "url(foo" },
    { id: "pending-function", property: "content", initial: '"before"', input: "var(--x" },
    {
        id: "nested-pending-functions",
        property: "padding",
        initial: "1px",
        input: "72px var(--space, var(--space,",
    },
    { id: "custom-string", property: "--x", initial: "before", input: '"hello' },
    { id: "custom-function", property: "--x", initial: "before", input: "fn(value" },
    { id: "custom-square-block", property: "--x", initial: "before", input: "[value" },
    { id: "custom-curly-block", property: "--x", initial: "before", input: "{value" },
    { id: "custom-comment", property: "--x", initial: "before", input: "red/*comment" },
    {
        id: "custom-mixed-comments",
        property: "--x",
        initial: "before",
        input: "a/* internal */b/* unfinished",
    },
    { id: "custom-terminal-escape", property: "--x", initial: "before", input: "foo\\" },
    { id: "custom-url-escape", property: "--x", initial: "before", input: "url(foo\\" },
    { id: "invalid-string", property: "font-family", initial: "serif", input: '"bad\nnext' },
    { id: "invalid-url", property: "background-image", initial: "none", input: 'url(bad"' },
    { id: "unmatched-close", property: "color", initial: "blue", input: "red)" },
]) {
    cases.push({
        id: `recovered-eof:${recoveryCase.id}`,
        operations: [
            ["set", recoveryCase.property, recoveryCase.initial, ""],
            ["set", recoveryCase.property, recoveryCase.input, ""],
        ],
        probes: [recoveryCase.property],
    });
}

function applyNative(state, operation) {
    const [kind, ...args] = operation;
    if (kind === "replace") state.replaceCssText(args[0]);
    if (kind === "set") state.setProperty(args[0], args[1], args[2]);
    if (kind === "remove") state.removeProperty(args[0]);
}

function nativeSnapshot(testCase) {
    const state = new binding.NativeDeclarationState();
    for (const operation of testCase.operations) applyNative(state, operation);
    return {
        cssText: state.cssText,
        items: Array.from({ length: state.length }, (_, index) => state.item(index)),
        values: Object.fromEntries(testCase.probes.map(name => [name, {
            value: state.getPropertyValue(name),
            priority: state.getPropertyPriority(name),
        }])),
    };
}

const browser = await chromium.launch({ headless: true });
try {
    const page = await browser.newPage();
    const browserSnapshots = await page.evaluate(testCases => testCases.map(testCase => {
        const style = document.createElement("div").style;
        for (const [kind, ...args] of testCase.operations) {
            if (kind === "replace") style.cssText = args[0];
            if (kind === "set") style.setProperty(args[0], args[1], args[2]);
            if (kind === "remove") style.removeProperty(args[0]);
        }
        return {
            cssText: style.cssText,
            items: Array.from(style),
            values: Object.fromEntries(testCase.probes.map(name => [name, {
                value: style.getPropertyValue(name),
                priority: style.getPropertyPriority(name),
            }])),
        };
    }), cases);

    for (let index = 0; index < cases.length; index += 1) {
        assert.deepEqual(nativeSnapshot(cases[index]), browserSnapshots[index], cases[index].id);
    }

    const valueCapabilityBrowserSnapshots = await page.evaluate(testCases => testCases.map(testCase => {
        const style = document.createElement("div").style;
        style.setProperty(testCase.property, testCase.input);
        return {
            accepted: style.length > 0,
            observable: style.getPropertyValue(testCase.property),
        };
    }), valueCapabilityCorpus.cases);

    for (let index = 0; index < valueCapabilityCorpus.cases.length; index += 1) {
        const testCase = valueCapabilityCorpus.cases[index];
        const browserSnapshot = valueCapabilityBrowserSnapshots[index];
        assert.equal(
            browserSnapshot.accepted,
            testCase.accepted,
            `${testCase.id}: Chromium acceptance drifted`,
        );
        assert.equal(
            browserSnapshot.observable,
            testCase.observable ?? "",
            `${testCase.id}: Chromium serialization drifted`,
        );

        const state = new binding.NativeDeclarationState();
        state.setProperty(testCase.property, testCase.input, "");
        assert.deepEqual(
            {
                accepted: state.length > 0,
                observable: state.getPropertyValue(testCase.property),
            },
            browserSnapshot,
            testCase.id,
        );
    }

    const numberResultBrowserSnapshots = await page.evaluate(testCases => testCases.map(testCase => {
        const style = document.createElement("div").style;
        style.setProperty(testCase.property, testCase.input);
        return {
            accepted: style.length > 0,
            observable: style.getPropertyValue(testCase.property),
            items: Array.from(style),
            cssText: style.cssText,
        };
    }), numberResultMathCorpus.cases);

    for (let index = 0; index < numberResultMathCorpus.cases.length; index += 1) {
        const testCase = numberResultMathCorpus.cases[index];
        const browserSnapshot = numberResultBrowserSnapshots[index];
        assert.equal(
            browserSnapshot.accepted,
            testCase.accepted,
            `${testCase.id}: Chromium acceptance drifted`,
        );
        assert.equal(
            browserSnapshot.observable,
            testCase.observable ?? "",
            `${testCase.id}: Chromium serialization drifted`,
        );
        assert.deepEqual(
            browserSnapshot.items,
            testCase.items ?? [],
            `${testCase.id}: Chromium expansion drifted`,
        );
        assert.equal(
            browserSnapshot.cssText,
            testCase.cssText ?? "",
            `${testCase.id}: Chromium declaration serialization drifted`,
        );
        const state = new binding.NativeDeclarationState();
        state.setProperty(testCase.property, testCase.input, "");
        assert.deepEqual(
            {
                accepted: state.length > 0,
                observable: state.getPropertyValue(testCase.property),
                items: Array.from({ length: state.length }, (_, itemIndex) => state.item(itemIndex)),
                cssText: state.cssText,
            },
            browserSnapshot,
            testCase.id,
        );
    }

    const relativeColorBrowserSnapshots = await page.evaluate(testCases => testCases.map(testCase => {
        const style = document.createElement("div").style;
        style.setProperty(testCase.property, testCase.input);
        return {
            accepted: style.length === 1,
            observable: style.getPropertyValue(testCase.property),
        };
    }), relativeColorCorpus.cases);

    for (let index = 0; index < relativeColorCorpus.cases.length; index += 1) {
        const testCase = relativeColorCorpus.cases[index];
        const browserSnapshot = relativeColorBrowserSnapshots[index];
        assert.equal(
            browserSnapshot.accepted,
            testCase.chromiumAccepted,
            `${testCase.id}: Chromium acceptance drifted`,
        );
        assert.equal(
            browserSnapshot.observable,
            testCase.chromiumObservable ?? "",
            `${testCase.id}: Chromium serialization drifted`,
        );

        const state = new binding.NativeDeclarationState();
        state.setProperty(testCase.property, testCase.input, "");
        assert.deepEqual(
            {
                accepted: state.length === 1,
                observable: state.getPropertyValue(testCase.property),
            },
            browserSnapshot,
            testCase.id,
        );
    }
} finally {
    await browser.close();
}

console.log(
    `${cases.length} native declaration sequences and ` +
    `${valueCapabilityCorpus.cases.length} value-capability cases and ` +
    `${numberResultMathCorpus.cases.length} number-result math cases and ` +
    `${relativeColorCorpus.cases.length} relative-color cases match Chromium.`,
);
