import assert from "node:assert/strict";

import { chromium } from "playwright";

import contracts from "../compatibility/browser-longhand-composite-contracts.json" with { type: "json" };
import { CSSStyleRule, CSSStyleSheet } from "../dist/index.js";

const cases = contracts.properties.flatMap(({ property, branches }) =>
  branches.map(branch => ({
    id: `${property}.${branch.id}`,
    property,
    input: branch.input,
    invalidNeighbor: branch.invalidNeighbor,
  })),
);

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
    style.setProperty(candidate.property, candidate.input, "important");
    const valid = observe(style, candidate.property);
    style.setProperty(candidate.property, candidate.invalidNeighbor, "");
    const afterInvalid = observe(style, candidate.property);
    const removed = style.removeProperty(candidate.property);
    results.push({
      id: candidate.id,
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
  await page.setContent("<!doctype html><title>SheetOM composite longhand differential</title>");
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
      style.setProperty(candidate.property, candidate.input, "important");
      const valid = observeState(candidate.property);
      style.setProperty(candidate.property, candidate.invalidNeighbor, "");
      const afterInvalid = observeState(candidate.property);
      const removed = style.removeProperty(candidate.property);
      results.push({
        id: candidate.id,
        valid,
        afterInvalid,
        removed,
        afterRemove: observeState(candidate.property),
      });
    }
    return { userAgent: navigator.userAgent, results };
  }, cases);
} finally {
  await browser.close();
}

const expectedMajor = contracts.baseline.match(/Chrome\/(\d+)/u)?.[1];
const actualMajor = chromiumRun.userAgent.match(/Chrome\/(\d+)/u)?.[1];
assert.equal(actualMajor, expectedMajor, `Unexpected Chromium oracle ${chromiumRun.userAgent}`);

const sheet = new CSSStyleSheet();
sheet.insertRule(".probe {}", 0);
const rule = sheet.cssRules[0];
assert.ok(rule instanceof CSSStyleRule);
const sheetomById = new Map(runCases(rule.style, cases).map(result => [result.id, result]));

const mismatches = [];
for (const chromiumResult of chromiumRun.results) {
  const sheetomResult = sheetomById.get(chromiumResult.id);
  try {
    assert.notEqual(
      chromiumResult.valid.length,
      0,
      `${chromiumResult.id} valid branch is not accepted by Chromium`,
    );
    assert.deepEqual(
      chromiumResult.afterInvalid,
      chromiumResult.valid,
      `${chromiumResult.id} invalid neighbor is not an atomic Chromium no-op`,
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
  throw new Error(`${mismatches.length} composite browser longhand branches diverged`);
}

console.log(
  `Verified ${cases.length} composite browser longhand branches and invalid neighbors against Chromium.`,
);
