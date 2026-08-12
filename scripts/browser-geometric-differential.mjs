import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { readFile, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

import { chromium } from "playwright";

import contracts from "../compatibility/browser-geometric-contracts.json" with { type: "json" };
import {
  CSSStyleRule,
  CSSStyleSheet,
  parseStyleSheet,
} from "../dist/index.js";

const scriptPath = fileURLToPath(import.meta.url);
const contractPath = fileURLToPath(
  new URL("../compatibility/browser-geometric-contracts.json", import.meta.url),
);
const sha256 = bytes => createHash("sha256").update(bytes).digest("hex");

const reviewedCases = contracts.properties.flatMap(({ property, branches }) =>
  branches.map(branch => ({
    id: `${property}.${branch.id}`,
    property,
    input: branch.input,
    invalidNeighbor: branch.invalidNeighbor,
  })),
);

function permutations(values) {
  if (values.length <= 1) return [values];
  const result = [];
  for (let index = 0; index < values.length; index += 1) {
    const value = values[index];
    const remaining = [...values.slice(0, index), ...values.slice(index + 1)];
    for (const suffix of permutations(remaining)) result.push([value, ...suffix]);
  }
  return result;
}

const arcPermutationCases = permutations(["of 5px 6px", "cw", "large", "rotate 30deg"])
  .map((options, index) => ({
    id: `shape-outside.generated-arc-options-${index + 1}`,
    property: "shape-outside",
    input: `shape(from 0 0, arc to 10px 20px ${options.join(" ")})`,
    invalidNeighbor: "shape(from 0 0, arc to 10px 20px cw ccw of 5px)",
  }));

const axisCases = [
  ["hline-to-left", "hline to left", "hline to top"],
  ["hline-to-center", "hline to center", "hline to left 1px"],
  ["hline-to-right", "hline to right", "hline to y-start"],
  ["hline-to-x-start", "hline to x-start", "hline to x-start 1px"],
  ["hline-to-x-end", "hline to x-end", "hline to x-end 1px"],
  ["vline-to-top", "vline to top", "vline to left"],
  ["vline-to-center", "vline to center", "vline to top 1px"],
  ["vline-to-bottom", "vline to bottom", "vline to x-end"],
  ["vline-to-y-start", "vline to y-start", "vline to y-start 1px"],
  ["vline-to-y-end", "vline to y-end", "vline to y-end 1px"],
  ["hline-by", "hline by calc(1px + 2%)", "hline by left"],
  ["vline-by", "vline by min(3px, 10%)", "vline by top"],
].map(([id, command, invalid]) => ({
  id: `shape-outside.generated-${id}`,
  property: "shape-outside",
  input: `shape(from 0 0, ${command})`,
  invalidNeighbor: `shape(from 0 0, ${invalid})`,
}));

const svgCommandCases = [
  ["moveto-relative", "m.5-.5", "m"],
  ["lineto-absolute", "M0 0 L1e2 -2E-2", "M0 0 L1"],
  ["lineto-relative", "M0 0 l+1.0-.5", "M0 0 l1"],
  ["horizontal-absolute", "M0 0 H1e-7", "M0 0 H"],
  ["horizontal-relative", "M0 0 h-0", "M0 0 h"],
  ["vertical-absolute", "M0 0 V.25", "M0 0 V"],
  ["vertical-relative", "M0 0 v-.25", "M0 0 v"],
  ["cubic-absolute", "M0 0 C1 2 3 4 5 6", "M0 0 C1 2 3"],
  ["cubic-relative", "M0 0 c1 2 3 4 5 6", "M0 0 c1 2 3"],
  ["smooth-cubic", "M0 0 S1 2 3 4 s5 6 7 8", "M0 0 S1 2 3"],
  ["quadratic", "M0 0 Q1 2 3 4 q5 6 7 8", "M0 0 Q1 2 3"],
  ["smooth-quadratic", "M0 0 T1 2 t3 4", "M0 0 T1"],
  ["arc-absolute", "M0 0 A1 2 30 1 0 3 4", "M0 0 A1 2 30 3 0 3 4"],
  ["arc-relative", "M0 0 a1 2 30 0 1 3 4", "M0 0 a1 2 30 0 2 3 4"],
  ["close-uppercase", "M0 0 Z", "Z"],
  ["close-lowercase", "M0 0 z", "M0"],
].map(([id, path, invalidPath]) => ({
  id: `d.generated-${id}`,
  property: "d",
  input: `path("${path}")`,
  invalidNeighbor: `path("${invalidPath}")`,
}));

const geometryBoxCases = [
  "margin-box",
  "border-box",
  "padding-box",
  "content-box",
].flatMap(box => [
  ["inset", "inset(1px)"],
  ["circle", "circle()"],
  ["ellipse", "ellipse()"],
  ["polygon", "polygon(0 0)"],
  ["path", 'path("M0 0")'],
  ["shape", "shape(from 0 0, close)"],
].flatMap(([shapeId, shape]) => [
  {
    id: `shape-outside.generated-${box}-${shapeId}-after`,
    property: "shape-outside",
    input: `${shape} ${box}`,
    invalidNeighbor: `${shape} ${box} content-box`,
  },
  {
    id: `shape-outside.generated-${box}-${shapeId}-before`,
    property: "shape-outside",
    input: `${box} ${shape}`,
    invalidNeighbor: `${box} url(x)`,
  },
]));

const clipPathGeometryBoxCases = [
  "margin-box",
  "border-box",
  "padding-box",
  "content-box",
  "fill-box",
  "stroke-box",
  "view-box",
].flatMap(box => [
  ["inset", "inset(1px)"],
  ["circle", "circle()"],
  ["ellipse", "ellipse()"],
  ["polygon", "polygon(0 0, 1px 2px)"],
  ["path", 'path("M0 0")'],
  ["rect", "rect(auto 1px 2px 3px)"],
  ["xywh", "xywh(0 0 1px 2px)"],
  ["shape", "shape(from 0 0, line to 1px 2px)"],
].flatMap(([shapeId, shape]) => [
  {
    id: `clip-path.generated-${box}-${shapeId}-after`,
    property: "clip-path",
    input: `${shape} ${box}`,
    invalidNeighbor: `${shape} ${box} content-box`,
  },
  {
    id: `clip-path.generated-${box}-${shapeId}-before`,
    property: "clip-path",
    input: `${box} ${shape}`,
    invalidNeighbor: `${box} ${shape} content-box`,
  },
]));

const rectangularShapeCases = [
  "inset(0)",
  "inset(1px 2px)",
  "inset(1px 2px 3px)",
  "inset(1px 2px 3px 4px)",
  "inset(calc(-1px) 2%)",
  "inset(0 round 1px)",
  "inset(0 round 1px 2px)",
  "inset(0 round 1px 2px 3px)",
  "inset(0 round 1px 2px 3px 4px / 5px 6px 7px 8px)",
  "rect(auto 1px 2px 3px)",
  "rect(auto 1px 2px 3px round 4px / 5px)",
  "xywh(0 0 0 0)",
  "xywh(-1px -2px calc(-1px) calc(2px))",
  "xywh(0 0 1px 2px round 3px 4px / 5px 6px)",
].map((input, index) => ({
  id: `object-view-box.generated-rectangular-${index + 1}`,
  property: "object-view-box",
  input,
  invalidNeighbor: "circle()",
}));

const basicShapeCases = [
  "circle()",
  "circle(at left top)",
  "circle(closest-side)",
  "circle(farthest-side at left 1px top 2px)",
  "circle(0)",
  "circle(calc(-1px))",
  "ellipse()",
  "ellipse(closest-side farthest-side)",
  "ellipse(1px 2% at right bottom)",
  "ellipse(calc(-1px) calc(2px))",
  "polygon(0 0)",
  "polygon(nonzero, 0 0, 100% 100%)",
  "polygon(evenodd, calc(1px + 2%) 0, 1px 2px)",
  'path("M0 0")',
  'path(nonzero, "M0 0 L1 1")',
  'path(evenodd, "M0 0Z")',
].map((input, index) => ({
  id: `shape-outside.generated-basic-shape-${index + 1}`,
  property: "shape-outside",
  input,
  invalidNeighbor: "auto",
}));

const gradientAndImageCases = [
  "url(x)",
  'url("a b")',
  'image-set("a.png" 1x, url(b.png) 2x type("image/png"))',
  "linear-gradient(to right, red, blue)",
  "repeating-linear-gradient(0deg, red 0 10px, blue 10px 20px)",
  "radial-gradient(ellipse farthest-corner at left top, red, blue)",
  "conic-gradient(from 10deg at 20% 30%, red 0, blue .5turn)",
  "-webkit-gradient(radial, center center, 0, center center, 100, from(red), color-stop(.5, green), to(blue))",
].map((input, index) => ({
  id: `shape-outside.generated-image-${index + 1}`,
  property: "shape-outside",
  input,
  invalidNeighbor: "url(x) circle()",
}));

const borderShapeCases = [
  "circle()",
  "circle() circle()",
  "circle() border-box",
  "circle() border-box ellipse() padding-box",
  'inset(1px) margin-box path("M0 0") content-box',
  "shape(from 0 0, close) fill-box polygon(0 0) stroke-box",
].map((input, index) => ({
  id: `border-shape.generated-pair-${index + 1}`,
  property: "border-shape",
  input,
  invalidNeighbor: "circle() ellipse() inset(1px)",
}));

const cases = [
  ...reviewedCases,
  ...arcPermutationCases,
  ...axisCases,
  ...svgCommandCases,
  ...geometryBoxCases,
  ...clipPathGeometryBoxCases,
  ...rectangularShapeCases,
  ...basicShapeCases,
  ...gradientAndImageCases,
  ...borderShapeCases,
];
assert.equal(new Set(cases.map(candidate => candidate.id)).size, cases.length);

function observe(style, property) {
  const entries = [];
  for (let index = 0; index < style.length; index += 1) {
    const name = style.item(index);
    entries.push({
      name,
      value: style.getPropertyValue(name),
      priority: style.getPropertyPriority(name),
    });
  }
  return {
    cssText: style.cssText,
    length: style.length,
    entries,
    value: style.getPropertyValue(property),
    priority: style.getPropertyPriority(property),
  };
}

function runCases(style, candidates) {
  const results = [];
  for (const candidate of candidates) {
    style.cssText = "";
    style.setProperty(candidate.property, candidate.invalidNeighbor, "important");
    const invalidInitial = observe(style, candidate.property);

    style.setProperty(candidate.property, candidate.input, "important");
    const valid = observe(style, candidate.property);
    style.setProperty(candidate.property, candidate.invalidNeighbor, "important");
    const afterInvalid = observe(style, candidate.property);
    const removed = style.removeProperty(candidate.property);
    results.push({
      id: candidate.id,
      invalidInitial,
      valid,
      afterInvalid,
      removed,
      afterRemove: observe(style, candidate.property),
    });
  }
  return results;
}

const browser = await chromium.launch({ headless: true });
let chromiumRun;
try {
  const page = await browser.newPage();
  await page.setContent("<!doctype html><title>SheetOM geometric differential</title>");
  chromiumRun = await page.evaluate(candidates => {
    const style = document.createElement("div").style;
    const observeState = property => {
      const entries = [];
      for (let index = 0; index < style.length; index += 1) {
        const name = style.item(index);
        entries.push({
          name,
          value: style.getPropertyValue(name),
          priority: style.getPropertyPriority(name),
        });
      }
      return {
        cssText: style.cssText,
        length: style.length,
        entries,
        value: style.getPropertyValue(property),
        priority: style.getPropertyPriority(property),
      };
    };
    const results = [];
    for (const candidate of candidates) {
      style.cssText = "";
      style.setProperty(candidate.property, candidate.invalidNeighbor, "important");
      const invalidInitial = observeState(candidate.property);
      style.setProperty(candidate.property, candidate.input, "important");
      const valid = observeState(candidate.property);
      style.setProperty(candidate.property, candidate.invalidNeighbor, "important");
      const afterInvalid = observeState(candidate.property);
      const removed = style.removeProperty(candidate.property);
      results.push({
        id: candidate.id,
        invalidInitial,
        valid,
        afterInvalid,
        removed,
        afterRemove: observeState(candidate.property),
      });
    }
    return { userAgent: navigator.userAgent, results };
  }, cases);

  const expectedMajor = contracts.baseline.match(/Chrome\/(\d+)/u)?.[1];
  const actualMajor = chromiumRun.userAgent.match(/Chrome\/(\d+)/u)?.[1];
  assert.equal(actualMajor, expectedMajor, `Unexpected Chromium oracle ${chromiumRun.userAgent}`);

  const sheet = new CSSStyleSheet();
  sheet.insertRule(".probe {}", 0);
  const rule = sheet.cssRules[0];
  assert.ok(rule instanceof CSSStyleRule);
  const sheetomResults = runCases(rule.style, cases);
  const sheetomById = new Map(sheetomResults.map(result => [result.id, result]));
  const mismatches = [];

  for (const chromiumResult of chromiumRun.results) {
    const sheetomResult = sheetomById.get(chromiumResult.id);
    try {
      assert.equal(
        chromiumResult.invalidInitial.length,
        0,
        `${chromiumResult.id} invalid neighbor is accepted by Chromium`,
      );
      assert.notEqual(
        chromiumResult.valid.length,
        0,
        `${chromiumResult.id} valid branch is rejected by Chromium`,
      );
      assert.deepEqual(
        chromiumResult.afterInvalid,
        chromiumResult.valid,
        `${chromiumResult.id} invalid replacement is not an atomic Chromium no-op`,
      );
      assert.deepEqual(sheetomResult, chromiumResult);
    } catch (error) {
      mismatches.push({
        id: chromiumResult.id,
        message: error.message,
        chromium: chromiumResult,
        sheetom: sheetomResult,
      });
    }
  }

  if (mismatches.length > 0) {
    console.error(JSON.stringify(mismatches.slice(0, 20), null, 2));
    throw new Error(`${mismatches.length} geometric grammar branches diverged`);
  }

  const serializedCases = [];
  for (const candidate of cases) {
    const candidateSheet = new CSSStyleSheet();
    candidateSheet.insertRule(".probe {}", 0);
    const candidateRule = candidateSheet.cssRules[0];
    assert.ok(candidateRule instanceof CSSStyleRule);
    candidateRule.style.setProperty(candidate.property, candidate.input, "important");
    const expected = observe(candidateRule.style, candidate.property);
    const serialized = candidateSheet.serialize();

    const reparsed = parseStyleSheet(serialized);
    const reparsedRule = reparsed.cssRules[0];
    assert.ok(reparsedRule instanceof CSSStyleRule);
    assert.deepEqual(
      observe(reparsedRule.style, candidate.property),
      expected,
      `${candidate.id} changed across SheetOM serialize and reparse`,
    );
    assert.equal(
      reparsed.serialize(),
      serialized,
      `${candidate.id} serialization is not idempotent`,
    );
    serializedCases.push({
      id: candidate.id,
      property: candidate.property,
      serialized,
      expected,
    });
  }

  const chromiumReparsed = await page.evaluate(serializedCases =>
    serializedCases.map(candidate => {
      const sheet = new CSSStyleSheet();
      sheet.replaceSync(candidate.serialized);
      const style = sheet.cssRules[0].style;
      const entries = [];
      for (let index = 0; index < style.length; index += 1) {
        const name = style.item(index);
        entries.push({
          name,
          value: style.getPropertyValue(name),
          priority: style.getPropertyPriority(name),
        });
      }
      return {
        id: candidate.id,
        state: {
          cssText: style.cssText,
          length: style.length,
          entries,
          value: style.getPropertyValue(candidate.property),
          priority: style.getPropertyPriority(candidate.property),
        },
      };
    }),
  serializedCases);

  for (const result of chromiumReparsed) {
    const expected = serializedCases.find(candidate => candidate.id === result.id)?.expected;
    assert.deepEqual(
      result.state,
      expected,
      `${result.id} safe serialization changes when Chromium reparses it`,
    );
  }
} finally {
  await browser.close();
}

const reportArgument = process.argv.find(argument => argument.startsWith("--report="));
if (reportArgument) {
  const output = path.resolve(reportArgument.slice("--report=".length));
  await writeFile(output, `${JSON.stringify({
    schemaVersion: 1,
    userAgent: chromiumRun.userAgent,
    passed: cases.length,
    total: cases.length,
    reviewed: reviewedCases.length,
    generated: cases.length - reviewedCases.length,
    contractsSha256: sha256(await readFile(contractPath)),
    generatorSha256: sha256(await readFile(scriptPath)),
  }, null, 2)}\n`);
}

console.log(
  `Verified ${cases.length} geometric branches, invalid neighbors, atomicity, and two-engine round trips against Chromium.`,
);
