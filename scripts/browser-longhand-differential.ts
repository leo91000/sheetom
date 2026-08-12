import assert from "node:assert/strict";

import { chromium } from "playwright";

import contracts from "../compatibility/browser-longhand-keyword-contracts.json" with { type: "json" };
import { CSSStyleRule, CSSStyleSheet } from "../dist/index.js";

const cases = [];
const groupByProperty = new Map();
for (const group of contracts.groups) {
  for (const property of group.properties) {
    groupByProperty.set(property, group);
    for (const value of group.values) {
      cases.push({
        id: `${group.id}.${property}.${value}`,
        queries: [property],
        operations: [{ op: "set", property, value }],
      });
      cases.push({
        id: `${group.id}.${property}.${value}.ascii-case-insensitive`,
        queries: [property],
        operations: [{ op: "set", property, value: value.toUpperCase() }],
      });
    }
    if (group.separator === "comma" && group.values.length > 1) {
      cases.push({
        id: `${group.id}.${property}.multiple-items`,
        queries: [property],
        operations: [{
          op: "set",
          property,
          value: `${group.values[0]},${group.values[1]}`,
        }],
      });
    }
    cases.push({
      id: `${group.id}.${property}.invalid-atomicity`,
      queries: [property],
      operations: [
        { op: "set", property, value: group.values[0], priority: "important" },
        { op: "set", property, value: "--not-a-keyword" },
      ],
      atomicAfter: 1,
    });
  }
}

for (const alias of contracts.aliases) {
  const group = groupByProperty.get(alias.canonical);
  assert.ok(group, `Missing canonical keyword contract for ${alias.property}`);
  for (const value of [group.values[0], "initial", "var(--value)"]) {
    cases.push({
      id: `alias.${alias.property}.${value}`,
      queries: [alias.property, alias.canonical],
      operations: [{ op: "set", property: alias.property, value, priority: "important" }],
    });
    cases.push({
      id: `alias.${alias.property}.${value}.remove`,
      queries: [alias.property, alias.canonical],
      operations: [
        { op: "set", property: alias.property, value, priority: "important" },
        { op: "remove", property: alias.property },
      ],
    });
  }
}

function observe(style, queries) {
  const entries = [];
  for (let index = 0; index < style.length; index += 1) {
    const property = style.item(index);
    entries.push({
      property,
      value: style.getPropertyValue(property),
      priority: style.getPropertyPriority(property),
    });
  }
  return {
    cssText: style.cssText,
    length: style.length,
    entries,
    queries: Object.fromEntries(queries.map(property => [property, {
      value: style.getPropertyValue(property),
      priority: style.getPropertyPriority(property),
    }])),
  };
}

function runCases(style, candidates) {
  const results = [];
  for (const candidate of candidates) {
    style.cssText = "";
    const snapshots = [];
    for (const operation of candidate.operations) {
      const returnValue = operation.op === "remove"
        ? style.removeProperty(operation.property)
        : (style.setProperty(operation.property, operation.value, operation.priority ?? ""), null);
      snapshots.push({ returnValue, state: observe(style, candidate.queries) });
    }
    results.push({ id: candidate.id, snapshots });
  }
  return results;
}

const browser = await chromium.launch({ headless: true });
let browserResult;
try {
  const page = await browser.newPage();
  await page.setContent("<!doctype html><title>SheetOM browser longhand differential</title>");
  browserResult = await page.evaluate(candidates => {
    const style = document.createElement("div").style;
    const observeState = (queries) => {
      const entries = [];
      for (let index = 0; index < style.length; index += 1) {
        const property = style.item(index);
        entries.push({
          property,
          value: style.getPropertyValue(property),
          priority: style.getPropertyPriority(property),
        });
      }
      return {
        cssText: style.cssText,
        length: style.length,
        entries,
        queries: Object.fromEntries(queries.map(property => [property, {
          value: style.getPropertyValue(property),
          priority: style.getPropertyPriority(property),
        }])),
      };
    };
    const results = [];
    for (const candidate of candidates) {
      style.cssText = "";
      const snapshots = [];
      for (const operation of candidate.operations) {
        const returnValue = operation.op === "remove"
          ? style.removeProperty(operation.property)
          : (style.setProperty(operation.property, operation.value, operation.priority ?? ""), null);
        snapshots.push({ returnValue, state: observeState(candidate.queries) });
      }
      results.push({ id: candidate.id, snapshots });
    }
    return { userAgent: navigator.userAgent, results };
  }, cases);
} finally {
  await browser.close();
}

const expectedMajor = contracts.baseline.match(/Chrome\/(\d+)/u)?.[1];
const actualMajor = browserResult.userAgent.match(/Chrome\/(\d+)/u)?.[1];
assert.equal(actualMajor, expectedMajor, `Unexpected Chromium oracle ${browserResult.userAgent}`);

const sheet = new CSSStyleSheet();
sheet.insertRule(".probe {}", 0);
const rule = sheet.cssRules[0];
assert.ok(rule instanceof CSSStyleRule);
const sheetomResult = runCases(rule.style, cases);

const browserById = new Map(browserResult.results.map(result => [result.id, result]));
const mismatches = [];
for (const candidate of cases) {
  const browserCase = browserById.get(candidate.id);
  const sheetomCase = sheetomResult.find(result => result.id === candidate.id);
  try {
    assert.deepEqual(sheetomCase, browserCase);
    if (candidate.atomicAfter !== undefined) {
      assert.deepEqual(
        sheetomCase.snapshots[candidate.atomicAfter].state,
        sheetomCase.snapshots[candidate.atomicAfter - 1].state,
      );
    }
  } catch (error) {
    mismatches.push({
      id: candidate.id,
      message: error.message,
      chromium: browserCase,
      sheetom: sheetomCase,
    });
  }
}

if (mismatches.length > 0) {
  console.error(JSON.stringify(mismatches.slice(0, 20), null, 2));
  throw new Error(`${mismatches.length} browser longhand keyword cases diverged`);
}

console.log(
  `Verified ${cases.length} browser longhand keyword and alias sequences against Chromium.`,
);
