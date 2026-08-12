import assert from "node:assert/strict";
import { createRequire } from "node:module";
import { readdir, readFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const repositoryRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const nativeDirectory = path.join(repositoryRoot, "native");
const [nativeArtifact] = (await readdir(nativeDirectory)).filter(
    entry => entry.startsWith("sheetom-native.") && entry.endsWith(".node"),
);
assert.ok(nativeArtifact, "build the native addon before running Operation Fixtures");
const require = createRequire(import.meta.url);
const binding = require(path.join(nativeDirectory, nativeArtifact));

const fixturesDirectory = path.join(repositoryRoot, "compatibility/fixtures/declarations");
const fixtureFiles = (await readdir(fixturesDirectory))
    .filter(file => file.endsWith(".json"))
    .sort();
const resolutions = JSON.parse(await readFile(
    path.join(repositoryRoot, "compatibility/resolutions/declarations.json"),
    "utf8",
)).resolutions;
const expectedByFixture = new Map(
    resolutions.map(resolution => [resolution.fixtureId, resolution.expected]),
);

function decodeBoundaryValue(value) {
    if (!value || typeof value !== "object" || !("$type" in value)) return value;
    if (value.$type === "undefined") return undefined;
    if (value.$type === "nan") return Number.NaN;
    if (value.$type === "positive-infinity") return Number.POSITIVE_INFINITY;
    if (value.$type === "negative-infinity") return Number.NEGATIVE_INFINITY;
    if (value.$type === "bigint") return BigInt(value.value ?? "0");
    if (value.$type === "symbol") return Symbol(value.value);
    if (value.$type === "throwing-string-coercion") {
        return { toString() { throw new Error(value.value ?? "string coercion failed"); } };
    }
    throw new Error(`Unknown Boundary Value: ${value.$type}`);
}

function encodeBoundaryValue(value) {
    if (value === undefined) return { $type: "undefined" };
    return value;
}

function cssValue(value) {
    return value === null ? "" : `${value}`;
}

function invoke(operation, target, args) {
    if (operation.op === "constructStyleRule") {
        return { state: new binding.NativeDeclarationState() };
    }
    if (operation.op === "getStyle") return target.state;
    if (operation.op === "setProperty") {
        if (args.length < 2) throw new TypeError("setProperty requires 2 arguments");
        const name = `${args[0]}`;
        const value = cssValue(args[1]);
        const priority = args.length < 3 || args[2] == null ? "" : `${args[2]}`;
        target.setProperty(name, value, priority);
        return undefined;
    }
    if (operation.op === "removeProperty") {
        if (args.length < 1) throw new TypeError("removeProperty requires 1 argument");
        return target.removeProperty(`${args[0]}`);
    }
    if (operation.op === "getPropertyValue") {
        if (args.length < 1) throw new TypeError("getPropertyValue requires 1 argument");
        return target.getPropertyValue(`${args[0]}`);
    }
    if (operation.op === "getPropertyPriority") {
        if (args.length < 1) throw new TypeError("getPropertyPriority requires 1 argument");
        return target.getPropertyPriority(`${args[0]}`);
    }
    if (operation.op === "setCssText") {
        target.replaceCssText(`${args[0]}`);
        return undefined;
    }
    throw new Error(`Unsupported native fixture operation: ${operation.op}`);
}

async function runFixture(fixture) {
    const handles = new Map([["$root", null]]);
    const observations = [];
    for (const operation of fixture.operations) {
        const target = handles.get(operation.target);
        const args = operation.args.map(decodeBoundaryValue);
        const observation = {};
        let result;
        try {
            result = invoke(operation, target, args);
            observation.exception = null;
        } catch (error) {
            observation.exception = { name: error instanceof Error ? error.name : "UnknownError" };
        }
        if (operation.handle && observation.exception === null) handles.set(operation.handle, result);
        const requested = operation.observe ?? [];
        if (requested.includes("return") && observation.exception === null) {
            observation.return = encodeBoundaryValue(result);
        }
        if (!requested.includes("exception")) delete observation.exception;
        if (requested.includes("cssText")) observation.cssText = target.cssText;
        if (requested.includes("length")) observation.length = target.length;
        if (requested.includes("items")) {
            observation.items = Array.from({ length: target.length }, (_, index) => target.item(index));
        }
        observations.push(observation);
    }
    return observations;
}

const failures = [];
for (const fixtureFile of fixtureFiles) {
    const fixture = JSON.parse(await readFile(path.join(fixturesDirectory, fixtureFile), "utf8"));
    const expected = expectedByFixture.get(fixture.id);
    if (!expected) {
        failures.push(`${fixture.id}: missing compatibility resolution`);
        continue;
    }
    const actual = await runFixture(fixture);
    try {
        assert.deepEqual(actual, expected);
    } catch (error) {
        failures.push(`${fixture.id}: ${error.message}`);
    }
}

assert.deepEqual(failures, [], `Native Operation Fixture failures:\n${failures.join("\n")}`);
console.log(`${fixtureFiles.length} native declaration Operation Fixtures passed.`);
