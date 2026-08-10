import { readFile, readdir } from "node:fs/promises";
import { createHash } from "node:crypto";
import assert from "node:assert/strict";
import path from "node:path";
import { fileURLToPath } from "node:url";

import Ajv2020 from "ajv/dist/2020.js";

import { chromiumShorthandLonghands } from "../src/chromium-properties.ts";

const repositoryRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const compatibilityRoot = path.join(repositoryRoot, "compatibility");

async function readJson(file) {
  return JSON.parse(await readFile(file, "utf8"));
}

async function jsonFiles(directory) {
  const entries = await readdir(directory, { withFileTypes: true });
  const files = [];

  for (const entry of entries) {
    const entryPath = path.join(directory, entry.name);
    if (entry.isDirectory()) {
      files.push(...await jsonFiles(entryPath));
      continue;
    }
    if (entry.name.endsWith(".json")) files.push(entryPath);
  }

  return files;
}

function validateOrThrow(validate, value, file) {
  if (validate(value)) return;
  const details = validate.errors
    ?.map(error => `${error.instancePath || "/"} ${error.message}`)
    .join("\n");
  throw new Error(`${path.relative(repositoryRoot, file)} is invalid:\n${details}`);
}

const ajv = new Ajv2020({ allErrors: true, strict: true });
const operationSchema = await readJson(path.join(compatibilityRoot, "schemas/operation-fixture.schema.json"));
const mappingsSchema = await readJson(path.join(compatibilityRoot, "schemas/wpt-mappings.schema.json"));
const reportSchema = await readJson(path.join(compatibilityRoot, "schemas/compatibility-report.schema.json"));
const resolutionsSchema = await readJson(path.join(compatibilityRoot, "schemas/compatibility-resolutions.schema.json"));
const valueCapabilitiesSchema = await readJson(path.join(compatibilityRoot, "schemas/value-capability-corpus.schema.json"));
const shorthandCapabilitiesSchema = await readJson(path.join(compatibilityRoot, "schemas/shorthand-capability-corpus.schema.json"));
const shorthandGrammarContractsSchema = await readJson(path.join(compatibilityRoot, "schemas/shorthand-grammar-contracts.schema.json"));
const shorthandGrammarObservationsSchema = await readJson(path.join(compatibilityRoot, "schemas/shorthand-grammar-observations.schema.json"));
const nativeGrammarInventorySchema = await readJson(path.join(compatibilityRoot, "schemas/native-grammar-inventory.schema.json"));
const functionRuleCasesSchema = await readJson(path.join(compatibilityRoot, "schemas/function-rule-cases.schema.json"));
const propertyGrammarExtensionsSchema = await readJson(path.join(compatibilityRoot, "schemas/property-grammar-extensions.schema.json"));
const validateOperation = ajv.compile(operationSchema);
const validateMappings = ajv.compile(mappingsSchema);
const validateReport = ajv.compile(reportSchema);
const validateResolutions = ajv.compile(resolutionsSchema);
const validateValueCapabilities = ajv.compile(valueCapabilitiesSchema);
const validateShorthandCapabilities = ajv.compile(shorthandCapabilitiesSchema);
const validateShorthandGrammarContracts = ajv.compile(shorthandGrammarContractsSchema);
const validateShorthandGrammarObservations = ajv.compile(shorthandGrammarObservationsSchema);
const validateNativeGrammarInventory = ajv.compile(nativeGrammarInventorySchema);
const validateFunctionRuleCases = ajv.compile(functionRuleCasesSchema);
const validatePropertyGrammarExtensions = ajv.compile(propertyGrammarExtensionsSchema);

const propertyGrammarExtensionsFile = path.join(
  compatibilityRoot,
  "property-grammar-extensions.json",
);
const propertyGrammarExtensions = await readJson(propertyGrammarExtensionsFile);
validateOrThrow(
  validatePropertyGrammarExtensions,
  propertyGrammarExtensions,
  propertyGrammarExtensionsFile,
);
assert.equal(
  new Set(propertyGrammarExtensions.families.map(family => family.id)).size,
  propertyGrammarExtensions.families.length,
  "Property Grammar Extension family IDs must be unique",
);

const valueCapabilitiesFile = path.join(compatibilityRoot, "value-capabilities.json");
validateOrThrow(
  validateValueCapabilities,
  await readJson(valueCapabilitiesFile),
  valueCapabilitiesFile,
);

const shorthandCapabilitiesFile = path.join(compatibilityRoot, "shorthand-capabilities.json");
const shorthandCapabilities = await readJson(shorthandCapabilitiesFile);
validateOrThrow(
  validateShorthandCapabilities,
  shorthandCapabilities,
  shorthandCapabilitiesFile,
);

const shorthandGrammarContractsFile = path.join(
  compatibilityRoot,
  "shorthand-grammar-contracts.json",
);
const shorthandGrammarContracts = await readJson(shorthandGrammarContractsFile);
validateOrThrow(
  validateShorthandGrammarContracts,
  shorthandGrammarContracts,
  shorthandGrammarContractsFile,
);
const shorthandGrammarObservationsFile = path.join(
  compatibilityRoot,
  "shorthand-grammar-observations.json",
);
const shorthandGrammarObservations = await readJson(shorthandGrammarObservationsFile);
validateOrThrow(
  validateShorthandGrammarObservations,
  shorthandGrammarObservations,
  shorthandGrammarObservationsFile,
);
const grammarCases = shorthandGrammarContracts.profiles.flatMap(profile => profile.cases);
assert.equal(new Set(shorthandGrammarContracts.profiles.map(profile => profile.codec)).size, 24);
assert.equal(new Set(grammarCases.map(grammarCase => grammarCase.id)).size, grammarCases.length);
assert.deepEqual(
  shorthandGrammarObservations.cases.map(observation => observation.id),
  grammarCases.map(grammarCase => grammarCase.id),
  "Grammar Branch observations must exactly follow the reviewed contracts",
);
for (let index = 0; index < grammarCases.length; index += 1) {
  assert.equal(
    shorthandGrammarObservations.cases[index]?.accepted,
    grammarCases[index]?.accepted,
    `${grammarCases[index]?.id} Chromium acceptance drifted`,
  );
}
const nativeGrammarInventoryFile = path.join(
  compatibilityRoot,
  "native-grammar-inventory.json",
);
const nativeGrammarInventory = await readJson(nativeGrammarInventoryFile);
validateOrThrow(
  validateNativeGrammarInventory,
  nativeGrammarInventory,
  nativeGrammarInventoryFile,
);
const propertyManifestFile = path.join(repositoryRoot, "src/chromium-properties.ts");
const propertyManifestSha256 = createHash("sha256")
  .update(await readFile(propertyManifestFile))
  .digest("hex");
if (shorthandCapabilities.baseline.propertyManifestSha256 !== propertyManifestSha256) {
  throw new Error("Shorthand Capability Corpus is stale for chromium-properties.ts");
}
const fileSha256 = async file => createHash("sha256")
  .update(await readFile(file))
  .digest("hex");
assert.equal(
  nativeGrammarInventory.baseline.propertyManifestSha256,
  propertyManifestSha256,
  "Native Grammar Inventory is stale for chromium-properties.ts",
);
assert.equal(
  nativeGrammarInventory.baseline.shorthandCapabilitiesSha256,
  await fileSha256(shorthandCapabilitiesFile),
  "Native Grammar Inventory is stale for shorthand-capabilities.json",
);
assert.equal(
  nativeGrammarInventory.baseline.shorthandGrammarContractsSha256,
  await fileSha256(shorthandGrammarContractsFile),
  "Native Grammar Inventory is stale for shorthand-grammar-contracts.json",
);
assert.equal(
  nativeGrammarInventory.baseline.valueCapabilitiesSha256,
  await fileSha256(valueCapabilitiesFile),
  "Native Grammar Inventory is stale for value-capabilities.json",
);

const inventoryProperties = nativeGrammarInventory.properties.map(entry => entry.property);
const manifestedShorthands = Object.entries(chromiumShorthandLonghands)
  .filter(([, longhands]) => longhands.length > 1)
  .map(([property]) => property)
  .sort();
assert.deepEqual(inventoryProperties, manifestedShorthands);
assert.equal(new Set(inventoryProperties).size, 129);
const inventoryProfileByCodec = new Map(
  nativeGrammarInventory.profiles.map(profile => [profile.codec, profile]),
);
assert.equal(inventoryProfileByCodec.size, 24);
for (const profile of shorthandGrammarContracts.profiles) {
  const inventoryProfile = inventoryProfileByCodec.get(profile.codec);
  assert.ok(inventoryProfile, `Missing native grammar profile ${profile.codec}`);
  assert.deepEqual(
    inventoryProfile.contractCaseIds,
    profile.cases.map(grammarCase => grammarCase.id),
    `Native grammar profile ${profile.codec} is stale`,
  );
}
const breadthCaseIds = new Set(shorthandCapabilities.cases.map(capability => capability.id));
const propertyBranchIds = new Set(
  nativeGrammarInventory.propertyBranches.map(branch => branch.id),
);
assert.equal(propertyBranchIds.size, nativeGrammarInventory.propertyBranches.length);
for (const property of nativeGrammarInventory.properties) {
  assert.ok(breadthCaseIds.has(property.breadthCaseId), property.property);
  const profile = inventoryProfileByCodec.get(property.codec);
  assert.ok(profile?.properties.includes(property.property), property.property);
  for (const branchId of property.propertyBranchIds) {
    assert.ok(propertyBranchIds.has(branchId), branchId);
  }
}
for (const branch of nativeGrammarInventory.propertyBranches) {
  assert.equal(branch.chromium.id, branch.id, branch.id);
  assert.equal(branch.chromium.accepted, branch.accepted, branch.id);
  if (!branch.accepted) {
    const preserved = nativeGrammarInventory.propertyBranches.find(
      candidate => candidate.id === branch.preserves,
    );
    assert.equal(preserved?.property, branch.property, branch.id);
    assert.equal(preserved?.accepted, true, branch.id);
  }
}

const mappingsFile = path.join(compatibilityRoot, "wpt-mappings.json");
const mappings = await readJson(mappingsFile);
validateOrThrow(validateMappings, mappings, mappingsFile);

const lock = await readJson(path.join(compatibilityRoot, "wpt.lock.json"));
if (mappings.wptCommit !== lock.commit) {
  throw new Error("WPT mappings and lock file must reference the same commit");
}

const functionRuleCasesFile = path.join(compatibilityRoot, "function-rule-cases.json");
const functionRuleCases = await readJson(functionRuleCasesFile);
validateOrThrow(validateFunctionRuleCases, functionRuleCases, functionRuleCasesFile);
assert.equal(
  functionRuleCases.baseline.wptCommit,
  lock.commit,
  "Function Rule corpus and WPT lock file must reference the same commit",
);
assert.equal(
  new Set(functionRuleCases.cases.map(testCase => testCase.id)).size,
  functionRuleCases.cases.length,
  "Function Rule case IDs must be unique",
);

const fixtureDirectory = path.join(compatibilityRoot, "fixtures");
const fixtureIds = new Set();
const fixturesById = new Map();
for (const file of await jsonFiles(fixtureDirectory)) {
  const fixture = await readJson(file);
  validateOrThrow(validateOperation, fixture, file);
  if (fixtureIds.has(fixture.id)) throw new Error(`Duplicate Operation Fixture ID: ${fixture.id}`);
  fixtureIds.add(fixture.id);
  fixturesById.set(fixture.id, fixture);
}

const mappingIds = new Set();
for (const mapping of mappings.mappings) {
  if (mappingIds.has(mapping.id)) throw new Error(`Duplicate WPT Mapping ID: ${mapping.id}`);
  mappingIds.add(mapping.id);
  for (const fixtureId of mapping.fixtureIds) {
    const fixture = fixturesById.get(fixtureId);
    if (!fixture) throw new Error(`WPT Mapping ${mapping.id} references missing fixture ${fixtureId}`);
    if (!fixture.provenanceIds?.includes(mapping.id)) {
      throw new Error(`Fixture ${fixtureId} does not retain WPT provenance ${mapping.id}`);
    }
  }
}
for (const fixture of fixturesById.values()) {
  for (const provenanceId of fixture.provenanceIds ?? []) {
    if (!mappingIds.has(provenanceId)) {
      throw new Error(`Fixture ${fixture.id} references missing WPT Mapping ${provenanceId}`);
    }
  }
}

const baselineDirectory = path.join(compatibilityRoot, "baselines");
for (const file of await jsonFiles(baselineDirectory)) {
  validateOrThrow(validateReport, await readJson(file), file);
}

const resolutionDirectory = path.join(compatibilityRoot, "resolutions");
const resolvedFixtureIds = new Set();
for (const file of await jsonFiles(resolutionDirectory)) {
  const document = await readJson(file);
  validateOrThrow(validateResolutions, document, file);
  for (const resolution of document.resolutions) {
    if (resolvedFixtureIds.has(resolution.fixtureId)) {
      throw new Error(`Duplicate Compatibility Resolution: ${resolution.fixtureId}`);
    }
    resolvedFixtureIds.add(resolution.fixtureId);
  }
}

for (const fixtureId of fixtureIds) {
  if (!resolvedFixtureIds.has(fixtureId)) {
    throw new Error(`Unexplained Operation Fixture: ${fixtureId}`);
  }
}
for (const fixtureId of resolvedFixtureIds) {
  if (!fixtureIds.has(fixtureId)) {
    throw new Error(`Compatibility Resolution references a missing fixture: ${fixtureId}`);
  }
}

console.log("Conformance schemas and checked-in documents are valid.");
