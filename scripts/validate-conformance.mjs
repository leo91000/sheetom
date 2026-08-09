import { readFile, readdir } from "node:fs/promises";
import { createHash } from "node:crypto";
import assert from "node:assert/strict";
import path from "node:path";
import { fileURLToPath } from "node:url";

import Ajv2020 from "ajv/dist/2020.js";
import * as csstree from "css-tree";
import { transformStyleAttribute } from "lightningcss";

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
const validateOperation = ajv.compile(operationSchema);
const validateMappings = ajv.compile(mappingsSchema);
const validateReport = ajv.compile(reportSchema);
const validateResolutions = ajv.compile(resolutionsSchema);
const validateValueCapabilities = ajv.compile(valueCapabilitiesSchema);
const validateShorthandCapabilities = ajv.compile(shorthandCapabilitiesSchema);
const validateShorthandGrammarContracts = ajv.compile(shorthandGrammarContractsSchema);
const validateShorthandGrammarObservations = ajv.compile(shorthandGrammarObservationsSchema);

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
const propertyManifestFile = path.join(repositoryRoot, "src/chromium-properties.ts");
const propertyManifestSha256 = createHash("sha256")
  .update(await readFile(propertyManifestFile))
  .digest("hex");
if (shorthandCapabilities.baseline.propertyManifestSha256 !== propertyManifestSha256) {
  throw new Error("Shorthand Capability Corpus is stale for chromium-properties.ts");
}

const runtimeOverridesFile = path.join(
  repositoryRoot,
  "src/internal/shorthand-runtime-overrides.json",
);
const runtimeOverrides = await readJson(runtimeOverridesFile);
if (runtimeOverrides.schemaVersion !== 1) {
  throw new Error("Unsupported shorthand runtime override schema");
}

function sortedPairs(values) {
  return [...values].sort((left, right) =>
    left[0].localeCompare(right[0]) || left[1].localeCompare(right[1])
  );
}

function usesGeneralValueParser(property, input) {
  let declaration;
  let declarationCount = 0;
  try {
    transformStyleAttribute({
      code: new TextEncoder().encode(`${property}: ${input}`),
      visitor: {
        Declaration(candidate) {
          declaration = candidate;
          declarationCount += 1;
        },
      },
    });
  } catch {
    return false;
  }
  const typed = declarationCount === 1 &&
    declaration &&
    declaration.property !== "unparsed" &&
    declaration.property !== "custom";
  return typed || csstree.lexer.matchProperty(property, input).matched !== null;
}

const requiredLiteralOverrides = shorthandCapabilities.cases
  .filter(capability => !usesGeneralValueParser(capability.property, capability.input))
  .map(capability => [capability.property, capability.input]);
assert.deepEqual(
  sortedPairs(runtimeOverrides.literal),
  sortedPairs(requiredLiteralOverrides),
  "Measured Literal Overrides must equal the shorthand seeds not covered by general parsers",
);

const requiredLonghandOrders = {};
for (const capability of shorthandCapabilities.cases) {
  const manifestOrder = chromiumShorthandLonghands[capability.property];
  if (JSON.stringify(manifestOrder) === JSON.stringify(capability.chromium.items)) continue;
  requiredLonghandOrders[capability.property] = capability.chromium.items;
}
assert.deepEqual(
  runtimeOverrides.longhandOrders,
  requiredLonghandOrders,
  "Measured shorthand longhand orders must equal Chromium corpus divergences",
);

const mappingsFile = path.join(compatibilityRoot, "wpt-mappings.json");
const mappings = await readJson(mappingsFile);
validateOrThrow(validateMappings, mappings, mappingsFile);

const lock = await readJson(path.join(compatibilityRoot, "wpt.lock.json"));
if (mappings.wptCommit !== lock.commit) {
  throw new Error("WPT mappings and lock file must reference the same commit");
}

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
