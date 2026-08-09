import { readFile, readdir } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

import Ajv2020 from "ajv/dist/2020.js";

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
const validateOperation = ajv.compile(operationSchema);
const validateMappings = ajv.compile(mappingsSchema);
const validateReport = ajv.compile(reportSchema);
const validateResolutions = ajv.compile(resolutionsSchema);

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
