import { createHash } from "node:crypto";
import { readFile, writeFile } from "node:fs/promises";

import { chromium } from "playwright";

import { chromiumShorthandLonghands } from "../src/chromium-properties.ts";

const capabilitiesUrl = new URL("../compatibility/shorthand-capabilities.json", import.meta.url);
const contractsUrl = new URL("../compatibility/shorthand-grammar-contracts.json", import.meta.url);
const manifestUrl = new URL("../src/chromium-properties.ts", import.meta.url);
const valueCapabilitiesUrl = new URL("../compatibility/value-capabilities.json", import.meta.url);
const outputUrl = new URL("../compatibility/native-grammar-inventory.json", import.meta.url);
const mode = process.argv[2] ?? "--check";

if (!["--check", "--record"].includes(mode)) {
  throw new Error("Usage: generate-native-grammar-inventory.mjs [--check|--record]");
}

function sha256(bytes) {
  return createHash("sha256").update(bytes).digest("hex");
}

const [capabilitiesBytes, contractsBytes, manifestBytes, valueCapabilitiesBytes, inventoryBytes] = await Promise.all([
  readFile(capabilitiesUrl),
  readFile(contractsUrl),
  readFile(manifestUrl),
  readFile(valueCapabilitiesUrl),
  readFile(outputUrl),
]);
const capabilities = JSON.parse(capabilitiesBytes.toString("utf8"));
const contracts = JSON.parse(contractsBytes.toString("utf8"));
const valueCapabilities = JSON.parse(valueCapabilitiesBytes.toString("utf8"));
const inventory = JSON.parse(inventoryBytes.toString("utf8"));

const manifestedProperties = Object.entries(chromiumShorthandLonghands)
  .filter(([, longhands]) => longhands.length > 1)
  .map(([property]) => property)
  .sort();
const inventoryProperties = inventory.properties
  .map(property => property.property)
  .sort();
if (JSON.stringify(inventoryProperties) !== JSON.stringify(manifestedProperties)) {
  throw new Error("Native Grammar Inventory does not cover the Chromium shorthand manifest");
}

const breadthCases = new Set(capabilities.cases.map(capability => capability.id));
const profileContracts = new Map(
  contracts.profiles.map(profile => [
    profile.codec,
    profile.cases.map(grammarCase => grammarCase.id),
  ]),
);
const propertiesByProfile = new Map();
for (const property of inventory.properties) {
  if (!breadthCases.has(property.breadthCaseId)) {
    throw new Error(`Missing shorthand breadth case ${property.breadthCaseId}`);
  }
  const properties = propertiesByProfile.get(property.codec) ?? [];
  properties.push(property.property);
  propertiesByProfile.set(property.codec, properties);
}
for (const profile of inventory.profiles) {
  const contractCaseIds = profileContracts.get(profile.codec);
  if (
    mode === "--check"
    && JSON.stringify(profile.contractCaseIds) !== JSON.stringify(contractCaseIds)
  ) {
    throw new Error(`Native grammar contract drifted for ${profile.codec}`);
  }
  const properties = (propertiesByProfile.get(profile.codec) ?? []).sort();
  if (JSON.stringify(profile.properties) !== JSON.stringify(properties)) {
    throw new Error(`Native grammar property coverage drifted for ${profile.codec}`);
  }
}

const reviewedPropertyBranches = inventory.propertyBranches.map(({ chromium: _, ...branch }) => branch);
const browser = await chromium.launch({ headless: true });
let browserResult;
try {
  const page = await browser.newPage();
  browserResult = await page.evaluate(({ inputs, valueCases }) => {
    const observations = [];
    for (const input of inputs) {
      const style = document.createElement("div").style;
      style.setProperty(input.property, input.input);
      const items = Array.from(style);
      observations.push({
        id: input.id,
        accepted: items.length > 0,
        items,
        longhands: items.map(name => ({
          name,
          value: style.getPropertyValue(name),
          priority: style.getPropertyPriority(name),
        })),
        shorthandValue: style.getPropertyValue(input.property),
        priority: style.getPropertyPriority(input.property),
        cssText: style.cssText,
      });
    }
    const valueObservations = valueCases.map(input => {
      const style = document.createElement("div").style;
      style.setProperty(input.property, input.input);
      return {
        id: input.id,
        accepted: style.length > 0,
        observable: style.getPropertyValue(input.property),
      };
    });
    return { userAgent: navigator.userAgent, observations, valueObservations };
  }, { inputs: reviewedPropertyBranches, valueCases: valueCapabilities.cases });
} finally {
  await browser.close();
}

const observationById = new Map(
  browserResult.observations.map(observation => [observation.id, observation]),
);
const propertyBranches = reviewedPropertyBranches.map(branch => {
  const chromiumObservation = observationById.get(branch.id);
  if (!chromiumObservation || chromiumObservation.accepted !== branch.accepted) {
    throw new Error(`Chromium acceptance drifted for ${branch.id}`);
  }
  return { ...branch, chromium: chromiumObservation };
});
for (let index = 0; index < valueCapabilities.cases.length; index += 1) {
  const expected = valueCapabilities.cases[index];
  const actual = browserResult.valueObservations[index];
  if (
    actual?.id !== expected?.id
    || actual.accepted !== expected.accepted
    || (expected.accepted && actual.observable !== expected.observable)
  ) {
    throw new Error(`Chromium value capability drifted for ${expected?.id}`);
  }
}

const updatedInventory = {
  ...inventory,
  baseline: {
    ...inventory.baseline,
    userAgent: browserResult.userAgent,
    propertyManifestSha256: sha256(manifestBytes),
    shorthandCapabilitiesSha256: sha256(capabilitiesBytes),
    shorthandGrammarContractsSha256: sha256(contractsBytes),
    valueCapabilitiesSha256: sha256(valueCapabilitiesBytes),
  },
  profiles: inventory.profiles.map(profile => ({
    ...profile,
    contractCaseIds: profileContracts.get(profile.codec),
  })),
  propertyBranches,
};
const serialized = `${JSON.stringify(updatedInventory, null, 2)}\n`;

if (mode === "--record") {
  await writeFile(outputUrl, serialized);
  console.log(
    `Recorded ${inventory.properties.length} shorthand properties, ` +
    `${inventory.profiles.length} profiles, and ${propertyBranches.length} ` +
    "property-specific branches.",
  );
} else {
  if (inventoryBytes.toString("utf8") !== serialized) {
    throw new Error(
      "Native Grammar Inventory drifted; review the diff and run " +
      "npm run record:native-grammar to accept it",
    );
  }
  console.log(
    `Verified ${inventory.properties.length} shorthand properties and ` +
    `${propertyBranches.length} property-specific branches.`,
  );
}
