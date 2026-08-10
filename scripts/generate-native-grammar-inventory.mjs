import { createHash } from "node:crypto";
import { readFile, writeFile } from "node:fs/promises";

import { chromium } from "playwright";
import { createServer } from "vite";

const capabilitiesUrl = new URL("../compatibility/shorthand-capabilities.json", import.meta.url);
const contractsUrl = new URL("../compatibility/shorthand-grammar-contracts.json", import.meta.url);
const manifestUrl = new URL("../src/chromium-properties.ts", import.meta.url);
const outputUrl = new URL("../compatibility/native-grammar-inventory.json", import.meta.url);
const mode = process.argv[2] ?? "--check";

if (!["--check", "--record"].includes(mode)) {
  throw new Error("Usage: generate-native-grammar-inventory.mjs [--check|--record]");
}

const reviewedPropertyBranches = [
  {
    id: "native.animation.auto-duration",
    property: "animation",
    branch: "auto duration",
    input: "auto ease 1s foo",
    accepted: true,
  },
  {
    id: "native.animation.duplicate-auto-invalid",
    property: "animation",
    branch: "duplicate auto",
    input: "auto auto foo",
    accepted: false,
    preserves: "native.animation.auto-duration",
  },
  {
    id: "native.font.system-font",
    property: "font",
    branch: "system font",
    input: "caption",
    accepted: true,
  },
  {
    id: "native.font.system-font-tail-invalid",
    property: "font",
    branch: "system font with trailing family",
    input: "caption serif",
    accepted: false,
    preserves: "native.font.system-font",
  },
  {
    id: "native.background-position.four-components",
    property: "background-position",
    branch: "four edge-offset components",
    input: "left 10px top 20px",
    accepted: true,
  },
  {
    id: "native.background-position.five-components-invalid",
    property: "background-position",
    branch: "extra position component",
    input: "left 10px top 20px center",
    accepted: false,
    preserves: "native.background-position.four-components",
  },
  {
    id: "native.row-rule.full",
    property: "row-rule",
    branch: "width style and color",
    input: "2px dashed red",
    accepted: true,
  },
  {
    id: "native.row-rule.duplicate-style-invalid",
    property: "row-rule",
    branch: "duplicate style",
    input: "2px dashed solid red",
    accepted: false,
    preserves: "native.row-rule.full",
  },
  {
    id: "native.rule.full",
    property: "rule",
    branch: "shared column and row rule",
    input: "2px dashed red",
    accepted: true,
  },
  {
    id: "native.rule.duplicate-style-invalid",
    property: "rule",
    branch: "duplicate style",
    input: "2px dashed solid red",
    accepted: false,
    preserves: "native.rule.full",
  },
];

function sha256(bytes) {
  return createHash("sha256").update(bytes).digest("hex");
}

const [capabilitiesBytes, contractsBytes, manifestBytes] = await Promise.all([
  readFile(capabilitiesUrl),
  readFile(contractsUrl),
  readFile(manifestUrl),
]);
const capabilities = JSON.parse(capabilitiesBytes.toString("utf8"));
const contracts = JSON.parse(contractsBytes.toString("utf8"));

const vite = await createServer({
  appType: "custom",
  configFile: false,
  logLevel: "error",
  optimizeDeps: { noDiscovery: true },
  server: { middlewareMode: true },
});

let definitions;
try {
  const module = await vite.ssrLoadModule("/src/internal/shorthand-registry.ts");
  definitions = module.getStaticShorthandDefinitions();
} finally {
  await vite.close();
}

const propertiesByCodec = new Map();
for (const definition of definitions) {
  const properties = propertiesByCodec.get(definition.codec) ?? [];
  properties.push(definition.name);
  propertiesByCodec.set(definition.codec, properties);
}

const profileContracts = new Map(
  contracts.profiles.map(profile => [
    profile.codec,
    profile.cases.map(grammarCase => grammarCase.id),
  ]),
);
const breadthCases = new Map(
  capabilities.cases.map(capability => [capability.property, capability.id]),
);

const profiles = [...propertiesByCodec]
  .sort(([left], [right]) => left.localeCompare(right))
  .map(([codec, properties]) => {
    const contractCaseIds = profileContracts.get(codec);
    if (!contractCaseIds) throw new Error(`Missing Grammar Branch Contract for ${codec}`);
    return { codec, properties: [...properties].sort(), contractCaseIds };
  });
const properties = definitions
  .map(definition => {
    const breadthCaseId = breadthCases.get(definition.name);
    if (!breadthCaseId) throw new Error(`Missing breadth case for ${definition.name}`);
    return {
      property: definition.name,
      codec: definition.codec,
      breadthCaseId,
      propertyBranchIds: reviewedPropertyBranches
        .filter(branch => branch.property === definition.name)
        .map(branch => branch.id),
    };
  })
  .sort((left, right) => left.property.localeCompare(right.property));

const browser = await chromium.launch({ headless: true });
let browserResult;
try {
  const page = await browser.newPage();
  browserResult = await page.evaluate(inputs => {
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
    return { userAgent: navigator.userAgent, observations };
  }, reviewedPropertyBranches);
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

const inventory = {
  $schema: "./schemas/native-grammar-inventory.schema.json",
  schemaVersion: 1,
  baseline: {
    browser: "chromium",
    userAgent: browserResult.userAgent,
    propertyManifestSha256: sha256(manifestBytes),
    shorthandCapabilitiesSha256: sha256(capabilitiesBytes),
    shorthandGrammarContractsSha256: sha256(contractsBytes),
    derivation: "manifest-profile-property-branches@1",
  },
  profiles,
  properties,
  propertyBranches,
};
const serialized = `${JSON.stringify(inventory, null, 2)}\n`;

if (mode === "--record") {
  await writeFile(outputUrl, serialized);
  console.log(
    `Recorded ${properties.length} shorthand properties, ${profiles.length} profiles, and ` +
    `${propertyBranches.length} property-specific branches.`,
  );
} else {
  const current = await readFile(outputUrl, "utf8");
  if (current !== serialized) {
    throw new Error(
      "Native Grammar Inventory drifted; review the diff and run " +
      "npm run record:native-grammar to accept it",
    );
  }
  console.log(
    `Verified ${properties.length} shorthand properties and ` +
    `${propertyBranches.length} property-specific branches.`,
  );
}
