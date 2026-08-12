import { createHash } from "node:crypto";
import { readFile, writeFile } from "node:fs/promises";

import css from "@webref/css";
import { chromium } from "playwright";

import observations from "../compatibility/property-value-observations.json" with { type: "json" };
import probes from "../compatibility/property-value-probes.json" with { type: "json" };
import terminalDocument from "../compatibility/webref-semantic-terminals.json" with { type: "json" };
import {
  chromiumPropertyAliases,
  chromiumPropertyBaseline,
  chromiumSupportedProperties,
} from "../src/chromium-properties.ts";
import { generateWebrefSyntaxSamples } from "./lib/webref-syntax-samples.ts";

const outputUrl = new URL("../compatibility/webref-property-branches.json", import.meta.url);
const manifestUrl = new URL("../src/chromium-properties.ts", import.meta.url);
const terminalUrl = new URL("../compatibility/webref-semantic-terminals.json", import.meta.url);
const webrefDataUrl = new URL("../node_modules/@webref/css/css.json", import.meta.url);
const webrefPackageUrl = new URL("../node_modules/@webref/css/package.json", import.meta.url);
const mode = process.argv[2] ?? "--check";

if (!["--check", "--record"].includes(mode)) {
  throw new Error("Usage: generate-webref-property-branches.ts [--check|--record]");
}

const sha256 = value => createHash("sha256").update(value).digest("hex");
const stableId = (prefix, value) => `${prefix}.${sha256(value).slice(0, 16)}`;

const [definitions, manifestBytes, terminalBytes, webrefBytes, webrefPackage] = await Promise.all([
  css.index(),
  readFile(manifestUrl),
  readFile(terminalUrl),
  readFile(webrefDataUrl),
  readFile(webrefPackageUrl, "utf8").then(JSON.parse),
]);

if (terminalDocument.schemaVersion !== 1) {
  throw new Error("Unsupported Webref semantic terminal schema");
}
const terminalTypes = new Set();
const terminalValues = {};
for (const terminal of terminalDocument.terminals) {
  if (terminalTypes.has(terminal.type)) {
    throw new Error(`Duplicate Webref semantic terminal ${terminal.type}`);
  }
  terminalTypes.add(terminal.type);
  terminalValues[terminal.type] = terminal.values;
}

const inputByProbe = new Map(probes.values.map(probe => [probe.id, probe.input]));
const fallbackValues = new Map();
for (const [property, probe] of observations.accepted) {
  if (probe.startsWith("css-wide.") || fallbackValues.has(property)) continue;
  const input = inputByProbe.get(probe);
  if (input) fallbackValues.set(property, input);
}
for (const property of chromiumSupportedProperties) {
  if (!fallbackValues.has(property)) {
    throw new Error(`Missing a non-CSS-wide atomicity seed for ${property}`);
  }
}

const profileBySyntax = new Map();
const missingProperties = [];
for (const property of [...chromiumSupportedProperties].sort()) {
  const sourceProperty = chromiumPropertyAliases[property] ?? property;
  const definition = definitions.properties[property] ?? definitions.properties[sourceProperty];
  if (!definition?.syntax) {
    missingProperties.push(property);
    continue;
  }
  let profile = profileBySyntax.get(definition.syntax);
  if (!profile) {
    profile = {
      id: stableId("webref-profile", definition.syntax),
      syntax: definition.syntax,
      href: definition.href,
      representativeProperty: sourceProperty,
      properties: [],
    };
    profileBySyntax.set(definition.syntax, profile);
  }
  profile.properties.push(property);
}

const generationIssues = [];
const profiles = [...profileBySyntax.values()]
  .sort((left, right) => left.id.localeCompare(right.id))
  .map(profile => {
    const generated = generateWebrefSyntaxSamples({
      definitions,
      property: profile.representativeProperty,
      syntax: profile.syntax,
      fallbackValue: property => fallbackValues.get(property),
      terminalValues,
    });
    generationIssues.push(...generated.issues.map(issue => ({ profile: profile.id, ...issue })));
    const branchesByValue = new Map();
    for (const sample of generated.samples) {
      const branch = sample.branch.replace(
        `property:${profile.representativeProperty}`,
        profile.id,
      );
      const branches = branchesByValue.get(sample.value) ?? [];
      branches.push(branch);
      branchesByValue.set(sample.value, branches);
    }
    const samples = [...branchesByValue]
      .map(([input, branches]) => ({
        id: stableId("webref-sample", `${profile.id}\0${input}`),
        input,
        branches: [...new Set(branches)].sort(),
      }))
      .sort((left, right) => left.id.localeCompare(right.id));
    return {
      ...profile,
      properties: profile.properties.sort(),
      samples,
    };
  });

if (generationIssues.length > 0) {
  const summary = Object.groupBy(generationIssues, issue => issue.kind);
  throw new Error(
    `Webref sampling has unresolved branches: ${JSON.stringify(
      Object.fromEntries(Object.entries(summary).map(([kind, issues]) => [kind, issues.length])),
    )}`,
  );
}

const browser = await chromium.launch({ headless: true });
let browserResult;
try {
  const page = await browser.newPage();
  browserResult = await page.evaluate(({ profiles, seedInputs }) => {
    const accepted = [];
    const seeds = [];
    let checkCount = 0;
    let branchCount = 0;
    let sampleCount = 0;
    let missingInvalidNeighborCount = 0;
    let rejectedAtomicNoOpCount = 0;
    let invalidNeighborAtomicNoOpCount = 0;
    const invalidSuffixes = [
      " __sheetom_invalid__",
      " /",
      ",",
      " !",
    ];
    const state = style => ({
      cssText: style.cssText,
      items: Array.from(style, name => [
        name,
        style.getPropertyValue(name),
        style.getPropertyPriority(name),
      ]),
    });
    for (const profile of profiles) {
      sampleCount += profile.samples.length;
      for (const sample of profile.samples) branchCount += sample.branches.length;
      for (const property of profile.properties) {
        const seedInput = seedInputs[property];
        const seedStyle = document.createElement("div").style;
        seedStyle.setProperty(property, seedInput);
        if (seedStyle.length === 0) {
          throw new Error(`Chromium rejected the Property Value seed for ${property}`);
        }
        const seedState = state(seedStyle);
        seeds.push([
          property,
          seedInput,
          seedStyle.getPropertyValue(property),
          seedState.cssText,
          seedState.items,
        ]);
        for (const sample of profile.samples) {
          checkCount += 1;
          const style = document.createElement("div").style;
          style.setProperty(property, sample.input);
          const items = Array.from(style);
          if (items.length === 0) {
            const atomicStyle = document.createElement("div").style;
            atomicStyle.setProperty(property, seedInput);
            const before = JSON.stringify(state(atomicStyle));
            atomicStyle.setProperty(property, sample.input);
            if (JSON.stringify(state(atomicStyle)) === before) {
              rejectedAtomicNoOpCount += 1;
            }
            continue;
          }
          let invalidNeighbor = null;
          for (const suffix of invalidSuffixes) {
            const candidate = `${sample.input}${suffix}`;
            const neighbor = document.createElement("div").style;
            neighbor.setProperty(property, candidate);
            if (neighbor.length === 0) {
              invalidNeighbor = candidate;
              break;
            }
          }
          if (invalidNeighbor === null) missingInvalidNeighborCount += 1;
          if (invalidNeighbor !== null) {
            const before = JSON.stringify(state(style));
            style.setProperty(property, invalidNeighbor);
            if (JSON.stringify(state(style)) === before) {
              invalidNeighborAtomicNoOpCount += 1;
            }
          }
          accepted.push([
            profile.id,
            property,
            sample.id,
            style.getPropertyValue(property),
            style.cssText,
            items.map(name => [
              name,
              style.getPropertyValue(name),
              style.getPropertyPriority(name),
            ]),
            invalidNeighbor,
          ]);
        }
      }
    }
    return {
      userAgent: navigator.userAgent,
      checkCount,
      branchCount,
      sampleCount,
      missingInvalidNeighborCount,
      rejectedAtomicNoOpCount,
      invalidNeighborAtomicNoOpCount,
      seeds,
      accepted,
    };
  }, {
    profiles,
    seedInputs: Object.fromEntries(fallbackValues),
  });
} finally {
  await browser.close();
}

if (browserResult.userAgent !== chromiumPropertyBaseline) {
  throw new Error("Webref grammar and Property Value corpora must use the same Chromium build");
}

const representativeTerminals = terminalDocument.terminals
  .filter(terminal => terminal.coverage !== "branch-complete")
  .map(terminal => terminal.type);
const report = {
  "$schema": "./schemas/webref-property-branches.schema.json",
  schemaVersion: 1,
  baseline: {
    browser: "chromium",
    userAgent: browserResult.userAgent,
    webrefVersion: webrefPackage.version,
    webrefCssSha256: sha256(webrefBytes),
    propertyManifestSha256: sha256(manifestBytes),
    semanticTerminalsSha256: sha256(terminalBytes),
    generator: "webref-value-definition-sampler@1",
  },
  coverage: {
    manifestedProperties: chromiumSupportedProperties.size,
    webrefProperties: chromiumSupportedProperties.size - missingProperties.length,
    profiles: profiles.length,
    samples: browserResult.sampleCount,
    branches: browserResult.branchCount,
    checks: browserResult.checkCount,
    accepted: browserResult.accepted.length,
    rejected: browserResult.checkCount - browserResult.accepted.length,
    missingInvalidNeighborCount: browserResult.missingInvalidNeighborCount,
    rejectedAtomicNoOpCount: browserResult.rejectedAtomicNoOpCount,
    invalidNeighborAtomicNoOpCount: browserResult.invalidNeighborAtomicNoOpCount,
    missingProperties,
    representativeTerminals,
  },
  profiles,
  seeds: browserResult.seeds,
  accepted: browserResult.accepted,
};
const serialized = `${JSON.stringify(report, null, 2)}\n`;

if (mode === "--record") {
  await writeFile(outputUrl, serialized);
  console.log(JSON.stringify(report.coverage, null, 2));
} else {
  const existing = await readFile(outputUrl, "utf8");
  if (existing !== serialized) {
    throw new Error(
      "Webref Property Branch corpus drifted; review and run "
      + "npm run record:webref-property-branches",
    );
  }
  console.log(JSON.stringify(report.coverage, null, 2));
}
