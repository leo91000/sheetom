import { readFile, readdir } from "node:fs/promises";
import { createHash } from "node:crypto";
import assert from "node:assert/strict";
import path from "node:path";
import { fileURLToPath } from "node:url";

import Ajv2020 from "ajv/dist/2020.js";

import {
  chromiumShorthandLonghands,
  chromiumSupportedProperties,
} from "../src/chromium-properties.ts";

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
const relativeColorCorpusSchema = await readJson(path.join(compatibilityRoot, "schemas/relative-color-corpus.schema.json"));
const numberResultMathCorpusSchema = await readJson(path.join(compatibilityRoot, "schemas/number-result-math-corpus.schema.json"));
const propertyValueProbesSchema = await readJson(path.join(compatibilityRoot, "schemas/property-value-probes.schema.json"));
const propertyValueObservationsSchema = await readJson(path.join(compatibilityRoot, "schemas/property-value-observations.schema.json"));
const numericPropertyContractsSchema = await readJson(path.join(compatibilityRoot, "schemas/numeric-property-contracts.schema.json"));
const browserLonghandKeywordContractsSchema = await readJson(path.join(compatibilityRoot, "schemas/browser-longhand-keyword-contracts.schema.json"));
const browserLonghandCompositeContractsSchema = await readJson(path.join(compatibilityRoot, "schemas/browser-longhand-composite-contracts.schema.json"));
const browserGeometricContractsSchema = await readJson(path.join(compatibilityRoot, "schemas/browser-geometric-contracts.schema.json"));
const webrefSemanticTerminalsSchema = await readJson(path.join(compatibilityRoot, "schemas/webref-semantic-terminals.schema.json"));
const webrefPropertyBranchesSchema = await readJson(path.join(compatibilityRoot, "schemas/webref-property-branches.schema.json"));
const webrefBranchRatchetSchema = await readJson(path.join(compatibilityRoot, "schemas/webref-branch-ratchet.schema.json"));
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
const validateRelativeColorCorpus = ajv.compile(relativeColorCorpusSchema);
const validateNumberResultMathCorpus = ajv.compile(numberResultMathCorpusSchema);
const validatePropertyValueProbes = ajv.compile(propertyValueProbesSchema);
const validatePropertyValueObservations = ajv.compile(propertyValueObservationsSchema);
const validateNumericPropertyContracts = ajv.compile(numericPropertyContractsSchema);
const validateBrowserLonghandKeywordContracts = ajv.compile(browserLonghandKeywordContractsSchema);
const validateBrowserLonghandCompositeContracts = ajv.compile(browserLonghandCompositeContractsSchema);
const validateBrowserGeometricContracts = ajv.compile(browserGeometricContractsSchema);
const validateWebrefSemanticTerminals = ajv.compile(webrefSemanticTerminalsSchema);
const validateWebrefPropertyBranches = ajv.compile(webrefPropertyBranchesSchema);
const validateWebrefBranchRatchet = ajv.compile(webrefBranchRatchetSchema);

const propertyValueProbesFile = path.join(
  compatibilityRoot,
  "property-value-probes.json",
);
const propertyValueObservationsFile = path.join(
  compatibilityRoot,
  "property-value-observations.json",
);
const browserLonghandKeywordContractsFile = path.join(
  compatibilityRoot,
  "browser-longhand-keyword-contracts.json",
);
const browserLonghandCompositeContractsFile = path.join(
  compatibilityRoot,
  "browser-longhand-composite-contracts.json",
);
const browserGeometricContractsFile = path.join(
  compatibilityRoot,
  "browser-geometric-contracts.json",
);
const propertyValueProbesBytes = await readFile(propertyValueProbesFile);
const propertyValueProbes = JSON.parse(propertyValueProbesBytes.toString("utf8"));
const propertyValueObservations = await readJson(propertyValueObservationsFile);
const browserLonghandKeywordContracts = await readJson(
  browserLonghandKeywordContractsFile,
);
const browserLonghandCompositeContracts = await readJson(
  browserLonghandCompositeContractsFile,
);
const browserGeometricContracts = await readJson(browserGeometricContractsFile);
validateOrThrow(
  validateBrowserGeometricContracts,
  browserGeometricContracts,
  browserGeometricContractsFile,
);
assert.equal(
  browserGeometricContracts.baseline,
  propertyValueObservations.baseline.userAgent,
  "Browser Geometric contracts must use the Property Value Chromium baseline",
);
assert.deepEqual(
  browserGeometricContracts.properties.map(entry => entry.property).sort(),
  ["border-shape", "d", "object-view-box", "shape-outside"],
  "Browser Geometric contracts must cover every SheetOM-owned geometric property exactly once",
);
for (const entry of browserGeometricContracts.properties) {
  assert.equal(
    new Set(entry.branches.map(branch => branch.id)).size,
    entry.branches.length,
    `${entry.property} geometric branch IDs must be unique`,
  );
}
validateOrThrow(
  validatePropertyValueProbes,
  propertyValueProbes,
  propertyValueProbesFile,
);
validateOrThrow(
  validateBrowserLonghandCompositeContracts,
  browserLonghandCompositeContracts,
  browserLonghandCompositeContractsFile,
);
assert.equal(
  browserLonghandCompositeContracts.baseline,
  propertyValueObservations.baseline.userAgent,
  "Browser Longhand Composite contracts must use the Property Value Chromium baseline",
);
assert.equal(
  new Set(browserLonghandCompositeContracts.properties.map(entry => entry.property)).size,
  browserLonghandCompositeContracts.properties.length,
  "Browser Longhand Composite properties must be unique",
);
for (const entry of browserLonghandCompositeContracts.properties) {
  assert.ok(
    chromiumSupportedProperties.has(entry.property),
    `Browser Longhand Composite contract references an unknown property: ${entry.property}`,
  );
  assert.equal(
    new Set(entry.branches.map(branch => branch.id)).size,
    entry.branches.length,
    `${entry.property} composite branch IDs must be unique`,
  );
}
validateOrThrow(
  validatePropertyValueObservations,
  propertyValueObservations,
  propertyValueObservationsFile,
);
validateOrThrow(
  validateBrowserLonghandKeywordContracts,
  browserLonghandKeywordContracts,
  browserLonghandKeywordContractsFile,
);
assert.equal(
  browserLonghandKeywordContracts.baseline,
  propertyValueObservations.baseline.userAgent,
  "Browser Longhand Keyword contracts must use the Property Value Chromium baseline",
);
assert.equal(
  new Set(browserLonghandKeywordContracts.groups.map(group => group.id)).size,
  browserLonghandKeywordContracts.groups.length,
  "Browser Longhand Keyword group IDs must be unique",
);
const contractedBrowserLonghands = new Set();
for (const group of browserLonghandKeywordContracts.groups) {
  assert.equal(
    new Set(group.properties).size,
    group.properties.length,
    `${group.id} properties must be unique`,
  );
  assert.equal(
    new Set(group.values).size,
    group.values.length,
    `${group.id} values must be unique`,
  );
  for (const property of group.properties) {
    assert.ok(
      chromiumSupportedProperties.has(property),
      `${group.id} references property missing from the Chromium manifest: ${property}`,
    );
    assert.ok(
      !contractedBrowserLonghands.has(property),
      `Browser Longhand Keyword property ${property} is contracted more than once`,
    );
    contractedBrowserLonghands.add(property);
  }
}
const contractedAliases = new Set();
for (const alias of browserLonghandKeywordContracts.aliases) {
  assert.ok(
    chromiumSupportedProperties.has(alias.property),
    `Browser Longhand alias is missing from the Chromium manifest: ${alias.property}`,
  );
  assert.ok(
    contractedBrowserLonghands.has(alias.canonical),
    `Browser Longhand alias ${alias.property} has no canonical keyword contract`,
  );
  assert.ok(
    !contractedAliases.has(alias.property),
    `Browser Longhand alias ${alias.property} is contracted more than once`,
  );
  contractedAliases.add(alias.property);
}
assert.equal(
  new Set(propertyValueProbes.values.map(probe => probe.id)).size,
  propertyValueProbes.values.length,
  "Property Value Probe IDs must be unique",
);
assert.equal(
  propertyValueObservations.baseline.propertyCount,
  chromiumSupportedProperties.size,
  "Property Value observations must cover the complete Chromium manifest",
);
assert.equal(
  propertyValueObservations.baseline.probeCount,
  propertyValueProbes.values.length,
  "Property Value observation probe count drifted",
);
assert.equal(
  propertyValueObservations.baseline.acceptedCount,
  propertyValueObservations.accepted.length,
  "Property Value accepted count drifted",
);
assert.equal(
  propertyValueObservations.baseline.acceptedCount +
    propertyValueObservations.baseline.rejectedCount,
  chromiumSupportedProperties.size * propertyValueProbes.values.length,
  "Property Value observations must account for the full cross product",
);
assert.equal(
  propertyValueObservations.baseline.atomicNoOpCount,
  propertyValueObservations.baseline.rejectedCount,
  "Every Chromium-rejected Property Value probe must be an atomic no-op",
);
assert.equal(
  propertyValueObservations.baseline.probesSha256,
  createHash("sha256").update(propertyValueProbesBytes).digest("hex"),
  "Property Value observations are stale for property-value-probes.json",
);
const propertyValueProbeIds = new Set(
  propertyValueProbes.values.map(probe => probe.id),
);
const observedPropertyValueKeys = new Set();
const observedPropertyNames = new Set();
for (const [property, probe] of propertyValueObservations.accepted) {
  assert.ok(
    chromiumSupportedProperties.has(property),
    `Property Value observations contain unknown property ${property}`,
  );
  assert.ok(
    propertyValueProbeIds.has(probe),
    `Property Value observations contain unknown probe ${probe}`,
  );
  const key = `${property}\0${probe}`;
  assert.ok(
    !observedPropertyValueKeys.has(key),
    `Property Value observation ${property}/${probe} is duplicated`,
  );
  observedPropertyValueKeys.add(key);
  observedPropertyNames.add(property);
}
assert.deepEqual(
  [...observedPropertyNames].sort(),
  [...chromiumSupportedProperties].sort(),
  "Every Chromium property must have at least one accepted Property Value probe",
);

const webrefSemanticTerminalsFile = path.join(
  compatibilityRoot,
  "webref-semantic-terminals.json",
);
const webrefPropertyBranchesFile = path.join(
  compatibilityRoot,
  "webref-property-branches.json",
);
const webrefBranchRatchetFile = path.join(
  compatibilityRoot,
  "webref-branch-ratchet.json",
);
const webrefSemanticTerminalsBytes = await readFile(webrefSemanticTerminalsFile);
const webrefPropertyBranchesBytes = await readFile(webrefPropertyBranchesFile);
const webrefSemanticTerminals = JSON.parse(webrefSemanticTerminalsBytes.toString("utf8"));
const webrefPropertyBranches = JSON.parse(webrefPropertyBranchesBytes.toString("utf8"));
const webrefBranchRatchet = await readJson(webrefBranchRatchetFile);
validateOrThrow(
  validateWebrefSemanticTerminals,
  webrefSemanticTerminals,
  webrefSemanticTerminalsFile,
);
validateOrThrow(
  validateWebrefPropertyBranches,
  webrefPropertyBranches,
  webrefPropertyBranchesFile,
);
validateOrThrow(
  validateWebrefBranchRatchet,
  webrefBranchRatchet,
  webrefBranchRatchetFile,
);
assert.equal(
  new Set(webrefSemanticTerminals.terminals.map(terminal => terminal.type)).size,
  webrefSemanticTerminals.terminals.length,
  "Webref semantic terminal types must be unique",
);
assert.equal(
  webrefPropertyBranches.baseline.userAgent,
  propertyValueObservations.baseline.userAgent,
  "Webref branches and Property Value observations must use the same Chromium baseline",
);
assert.equal(
  webrefPropertyBranches.baseline.propertyManifestSha256,
  createHash("sha256")
    .update(await readFile(path.join(repositoryRoot, "src/chromium-properties.ts")))
    .digest("hex"),
  "Webref branches are stale for chromium-properties.ts",
);
assert.equal(
  webrefPropertyBranches.baseline.semanticTerminalsSha256,
  createHash("sha256").update(webrefSemanticTerminalsBytes).digest("hex"),
  "Webref branches are stale for webref-semantic-terminals.json",
);
const webrefPackage = await readJson(path.join(
  repositoryRoot,
  "node_modules/@webref/css/package.json",
));
assert.equal(
  webrefPropertyBranches.baseline.webrefVersion,
  webrefPackage.version,
  "Webref branch evidence must use the installed pinned @webref/css version",
);
assert.equal(
  webrefPropertyBranches.baseline.webrefCssSha256,
  createHash("sha256")
    .update(await readFile(path.join(repositoryRoot, "node_modules/@webref/css/css.json")))
    .digest("hex"),
  "Webref branch evidence is stale for the installed @webref/css data",
);

const webrefProfileIds = new Set();
const webrefProfileSyntaxes = new Set();
const webrefCoveredProperties = new Set();
const webrefCandidateKeys = new Set();
let webrefSampleCount = 0;
let webrefBranchCount = 0;
let webrefCheckCount = 0;
for (const profile of webrefPropertyBranches.profiles) {
  assert.ok(!webrefProfileIds.has(profile.id), `Duplicate Webref profile ${profile.id}`);
  assert.ok(
    !webrefProfileSyntaxes.has(profile.syntax),
    `Duplicate Webref syntax profile ${profile.syntax}`,
  );
  webrefProfileIds.add(profile.id);
  webrefProfileSyntaxes.add(profile.syntax);
  const sampleIds = new Set();
  for (const sample of profile.samples) {
    assert.ok(!sampleIds.has(sample.id), `Duplicate Webref sample ${sample.id}`);
    sampleIds.add(sample.id);
    webrefBranchCount += sample.branches.length;
  }
  webrefSampleCount += profile.samples.length;
  webrefCheckCount += profile.properties.length * profile.samples.length;
  for (const property of profile.properties) {
    assert.ok(
      chromiumSupportedProperties.has(property),
      `Webref profile contains an unknown Chromium property: ${property}`,
    );
    assert.ok(
      !webrefCoveredProperties.has(property),
      `Webref property is assigned to multiple syntax profiles: ${property}`,
    );
    webrefCoveredProperties.add(property);
    for (const sample of profile.samples) {
      webrefCandidateKeys.add(`${property}\0${sample.id}`);
    }
  }
}
const expectedMissingWebrefProperties = [...chromiumSupportedProperties]
  .filter(property => !webrefCoveredProperties.has(property))
  .sort();
assert.deepEqual(
  webrefPropertyBranches.coverage.missingProperties,
  expectedMissingWebrefProperties,
  "Every manifested property absent from Webref must remain explicitly listed",
);
assert.equal(
  webrefPropertyBranches.coverage.manifestedProperties,
  chromiumSupportedProperties.size,
  "Webref branch evidence must account for the complete Chromium property manifest",
);
assert.equal(webrefPropertyBranches.coverage.webrefProperties, webrefCoveredProperties.size);
assert.equal(webrefPropertyBranches.coverage.profiles, webrefPropertyBranches.profiles.length);
assert.equal(webrefPropertyBranches.coverage.samples, webrefSampleCount);
assert.equal(webrefPropertyBranches.coverage.branches, webrefBranchCount);
assert.equal(webrefPropertyBranches.coverage.checks, webrefCheckCount);
assert.equal(
  webrefPropertyBranches.coverage.accepted + webrefPropertyBranches.coverage.rejected,
  webrefCheckCount,
  "Webref Chromium evidence must classify every generated candidate",
);
assert.equal(
  webrefPropertyBranches.coverage.accepted,
  webrefPropertyBranches.accepted.length,
  "Webref accepted count drifted",
);
assert.equal(
  webrefPropertyBranches.coverage.rejectedAtomicNoOpCount,
  webrefPropertyBranches.coverage.rejected,
  "Every Chromium-rejected Webref candidate must preserve previous state",
);
assert.equal(
  webrefPropertyBranches.coverage.invalidNeighborAtomicNoOpCount,
  webrefPropertyBranches.coverage.accepted,
  "Every accepted Webref candidate must have an atomic invalid neighbor",
);
const expectedRepresentativeTerminals = webrefSemanticTerminals.terminals
  .filter(terminal => terminal.coverage === "representative")
  .map(terminal => terminal.type);
assert.deepEqual(
  webrefPropertyBranches.coverage.representativeTerminals,
  expectedRepresentativeTerminals,
  "Webref representative terminal dispositions drifted",
);
const webrefAcceptedKeys = new Set();
const webrefSeedProperties = new Set();
for (const seed of webrefPropertyBranches.seeds) {
  const [property] = seed;
  assert.ok(webrefCoveredProperties.has(property), `Unknown Webref seed property ${property}`);
  assert.ok(!webrefSeedProperties.has(property), `Duplicate Webref seed for ${property}`);
  webrefSeedProperties.add(property);
}
assert.deepEqual(
  [...webrefSeedProperties].sort(),
  [...webrefCoveredProperties].sort(),
  "Every Webref-covered property must have exactly one atomicity seed",
);
for (const observation of webrefPropertyBranches.accepted) {
  const [profileId, property, sampleId] = observation;
  assert.ok(webrefProfileIds.has(profileId), `Unknown Webref profile ${profileId}`);
  const key = `${property}\0${sampleId}`;
  assert.ok(webrefCandidateKeys.has(key), `Unknown Webref candidate ${property}/${sampleId}`);
  assert.ok(!webrefAcceptedKeys.has(key), `Duplicate Webref observation ${property}/${sampleId}`);
  webrefAcceptedKeys.add(key);
}
assert.equal(
  webrefBranchRatchet.corpusSha256,
  createHash("sha256").update(webrefPropertyBranchesBytes).digest("hex"),
  "Webref mismatch ratchet is stale for the checked-in branch corpus",
);
assert.equal(
  webrefBranchRatchet.mismatchCases,
  webrefBranchRatchet.unresolved.length,
  "Webref mismatch case count drifted",
);
const webrefMismatchKeys = new Set();
const webrefMismatchSummary = Object.fromEntries(
  Object.keys(webrefBranchRatchet.summary).map(kind => [kind, 0]),
);
for (const unresolved of webrefBranchRatchet.unresolved) {
  const key = `${unresolved.property}\0${unresolved.sample}`;
  assert.ok(webrefCandidateKeys.has(key), `Unknown Webref mismatch ${unresolved.property}/${unresolved.sample}`);
  assert.ok(!webrefMismatchKeys.has(key), `Duplicate Webref mismatch ${unresolved.property}/${unresolved.sample}`);
  webrefMismatchKeys.add(key);
  for (const kind of unresolved.kinds) webrefMismatchSummary[kind] += 1;
}
assert.deepEqual(
  webrefBranchRatchet.summary,
  webrefMismatchSummary,
  "Webref mismatch kind totals drifted",
);

const numberResultMathCorpusFile = path.join(
  compatibilityRoot,
  "number-result-math-capabilities.json",
);
const numberResultMathCorpus = await readJson(numberResultMathCorpusFile);
validateOrThrow(
  validateNumberResultMathCorpus,
  numberResultMathCorpus,
  numberResultMathCorpusFile,
);
assert.equal(
  new Set(numberResultMathCorpus.cases.map(candidate => candidate.id)).size,
  numberResultMathCorpus.cases.length,
  "Number Result Math case IDs must be unique",
);
const numberResultProbeByBranch = new Map(
  numberResultMathCorpus.probes.map(probe => [probe.branch, probe.input]),
);
assert.equal(
  numberResultProbeByBranch.size,
  numberResultMathCorpus.probes.length,
  "Number Result Math probe branches must be unique",
);
for (const candidate of numberResultMathCorpus.cases) {
  assert.equal(
    candidate.input,
    numberResultProbeByBranch.get(candidate.branch),
    `${candidate.id} drifted from its declared probe`,
  );
  if (candidate.accepted) {
    assert.equal(typeof candidate.observable, "string", `${candidate.id} needs an observable`);
    assert.ok(Array.isArray(candidate.items), `${candidate.id} needs expanded items`);
    assert.equal(typeof candidate.cssText, "string", `${candidate.id} needs cssText`);
    continue;
  }
  assert.equal(candidate.observable, undefined, `${candidate.id} must omit its observable`);
  assert.equal(candidate.items, undefined, `${candidate.id} must omit expanded items`);
  assert.equal(candidate.cssText, undefined, `${candidate.id} must omit cssText`);
}

const relativeColorCorpusFile = path.join(
  compatibilityRoot,
  "relative-color-capabilities.json",
);
const relativeColorCorpus = await readJson(relativeColorCorpusFile);
validateOrThrow(
  validateRelativeColorCorpus,
  relativeColorCorpus,
  relativeColorCorpusFile,
);
assert.equal(
  relativeColorCorpus.provenance.wptCommit,
  (await readJson(path.join(compatibilityRoot, "wpt.lock.json"))).commit,
  "Relative Color Corpus is stale for wpt.lock.json",
);
assert.equal(
  new Set(relativeColorCorpus.cases.map(candidate => candidate.id)).size,
  relativeColorCorpus.cases.length,
  "Relative Color Corpus case IDs must be unique",
);
assert.equal(
  relativeColorCorpus.cases.length,
  1_306,
  "Relative Color Corpus must retain every reviewed WPT case",
);
for (const candidate of relativeColorCorpus.cases) {
  assert.equal(
    candidate.chromiumAccepted,
    candidate.wptAccepted,
    `${candidate.id} Chromium acceptance drifted from the pinned WPT expectation`,
  );
}

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
const numericPropertyContractsFile = path.join(
  compatibilityRoot,
  "numeric-property-contracts.json",
);
const numericPropertyContracts = await readJson(numericPropertyContractsFile);
validateOrThrow(
  validateNumericPropertyContracts,
  numericPropertyContracts,
  numericPropertyContractsFile,
);
const propertyGrammarFamilyIds = new Set(
  propertyGrammarExtensions.families.map(family => family.id),
);
for (const family of numericPropertyContracts.extensionFamilies) {
  assert.ok(
    propertyGrammarFamilyIds.has(family),
    `Numeric Property contract references unknown extension family ${family}`,
  );
}
for (const property of numericPropertyContracts.additionalProperties) {
  assert.ok(
    chromiumSupportedProperties.has(property),
    `Numeric Property contract references unknown property ${property}`,
  );
}
for (const probe of numericPropertyContracts.probes) {
  assert.ok(
    propertyValueProbeIds.has(probe),
    `Numeric Property contract references unknown probe ${probe}`,
  );
}
assert.equal(
  new Set(propertyGrammarExtensions.families.map(family => family.id)).size,
  propertyGrammarExtensions.families.length,
  "Property Grammar Extension family IDs must be unique",
);
assert.equal(
  numberResultMathCorpus.baseline.userAgent,
  propertyGrammarExtensions.baseline,
  "Number Result Math and Property Grammar Extension baselines must match",
);
assert.equal(
  numberResultMathCorpus.baseline.propertyManifestSha256,
  createHash("sha256")
    .update(await readFile(path.join(repositoryRoot, "src/chromium-properties.ts")))
    .digest("hex"),
  "Number Result Math Corpus is stale for chromium-properties.ts",
);
const numberResultProperties = branch => numberResultMathCorpus.cases
  .filter(candidate =>
    candidate.branch === branch &&
    candidate.accepted &&
    candidate.integration === "direct-number")
  .map(candidate => candidate.property)
  .sort();
const relativeLengthProperties = numberResultProperties("relative-length");
const percentageProperties = numberResultProperties("percentage");
const lengthPercentageOrNumberProperties = numberResultProperties(
  "dimension-result-neighbor",
);
const lengthPercentageOrNumberPropertySet = new Set(
  lengthPercentageOrNumberProperties,
);
const percentagePropertySet = new Set(percentageProperties);
const lengthOnlyProperties = relativeLengthProperties
  .filter(property => !percentagePropertySet.has(property));
const numberOnlyLengthPercentageProperties = percentageProperties
  .filter(property => !lengthPercentageOrNumberPropertySet.has(property));
const extensionFamilyProperties = id => propertyGrammarExtensions.families
  .find(family => family.id === id)?.properties?.slice().sort() ?? [];
assert.deepEqual(
  extensionFamilyProperties("length-number-calculation"),
  lengthOnlyProperties,
  "Length-only number-result runtime grammar drifted from Chromium evidence",
);
assert.deepEqual(
  extensionFamilyProperties("length-percentage-number-calculation"),
  numberOnlyLengthPercentageProperties,
  "Length-percentage number-result runtime grammar drifted from Chromium evidence",
);
assert.deepEqual(
  extensionFamilyProperties("length-percentage-or-number-calculation"),
  lengthPercentageOrNumberProperties,
  "Length-percentage-or-number runtime grammar drifted from Chromium evidence",
);
assert.equal(
  numberResultMathCorpus.cases.some(candidate =>
    candidate.branch === "invalid-relative-sine" && candidate.accepted),
  false,
  "The invalid neighboring sine branch must remain rejected",
);
const compositeNumberResultProperties = new Set(
  numberResultMathCorpus.cases
    .filter(candidate => candidate.integration === "composite-property")
    .map(candidate => candidate.property),
);
assert.equal(
  compositeNumberResultProperties.size,
  13,
  "Every composite number-result property must remain visible for the CSSOM-state tranche",
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
