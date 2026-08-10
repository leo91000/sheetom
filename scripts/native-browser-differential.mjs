import assert from "node:assert/strict";
import { createRequire } from "node:module";
import { readdir } from "node:fs/promises";
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
        id: "escaped-custom-name",
        operations: [["set", "--foo:bar", "red", ""]],
        probes: ["--foo:bar"],
    },
];

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
} finally {
    await browser.close();
}

console.log(`${cases.length} native declaration sequences match Chromium.`);
