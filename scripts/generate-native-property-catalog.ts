import { createHash } from "node:crypto";
import { readFile, writeFile } from "node:fs/promises";

import {
  chromiumPropertyAliases,
  chromiumPropertyBaseline,
  chromiumShorthandLonghands,
  chromiumSupportedProperties,
} from "../src/chromium-properties.ts";

const manifestUrl = new URL("../src/chromium-properties.ts", import.meta.url);
const capabilitiesUrl = new URL(
  "../compatibility/shorthand-capabilities.json",
  import.meta.url,
);
const grammarExtensionsUrl = new URL(
  "../compatibility/property-grammar-extensions.json",
  import.meta.url,
);
const outputUrl = new URL("../crates/sheetom-core/src/generated/chromium_properties.rs", import.meta.url);
const mode = process.argv[2] ?? "--check";

if (!["--check", "--record"].includes(mode)) {
  throw new Error("Usage: generate-native-property-catalog.ts [--check|--record]");
}

function rustString(value) {
  return JSON.stringify(value);
}

const source = await readFile(manifestUrl);
const sourceSha256 = createHash("sha256").update(source).digest("hex");
const capabilitiesSource = await readFile(capabilitiesUrl);
const capabilitiesSha256 = createHash("sha256")
  .update(capabilitiesSource)
  .digest("hex");
const grammarExtensionsSource = await readFile(grammarExtensionsUrl);
const capabilities = JSON.parse(capabilitiesSource);
const grammarExtensions = JSON.parse(grammarExtensionsSource);
if (grammarExtensions.baseline !== chromiumPropertyBaseline) {
  throw new Error("Property Grammar Extensions baseline drifted from Chromium");
}
const properties = [...chromiumSupportedProperties].sort();
const aliases = Object.entries(chromiumPropertyAliases).sort(([left], [right]) =>
  left.localeCompare(right),
);
const observedLonghandOrder = new Map(
  capabilities.cases.map(shorthandCase => [
    shorthandCase.property,
    shorthandCase.chromium.items,
  ]),
);
const shorthands = Object.entries(chromiumShorthandLonghands)
  .map(([shorthand, longhands]) => {
    const observed = observedLonghandOrder.get(shorthand);
    if (!observed) return [shorthand, longhands];
    const declaredSet = [...longhands].sort().join("\0");
    const observedSet = [...observed].sort().join("\0");
    if (declaredSet !== observedSet) {
      throw new Error(
        `Chromium longhand membership drifted for ${shorthand}; ` +
        "review the property manifest before recording a new order",
      );
    }
    return [shorthand, observed];
  })
  .sort(([left], [right]) => left.localeCompare(right));
const declaredShorthands = Object.entries(chromiumShorthandLonghands)
  .sort(([left], [right]) => left.localeCompare(right));
const shorthandLonghandNames = new Set(
  declaredShorthands.flatMap(([, longhands]) => longhands),
);
const shorthandLonghandBits = Array(Math.ceil(properties.length / 8)).fill(0);
for (const longhand of shorthandLonghandNames) {
  const index = properties.indexOf(longhand);
  if (index === -1) {
    throw new Error(`Unknown shorthand longhand ${longhand}`);
  }
  shorthandLonghandBits[index >> 3] |= 1 << (index & 7);
}
const initialLonghandValues = new Map();
for (const shorthandCase of capabilities.cases) {
  for (const longhand of shorthandCase.chromium.longhands) {
    const existing = initialLonghandValues.get(longhand.name);
    if (existing !== undefined && existing !== longhand.value) {
      throw new Error(
        `Conflicting Chromium initial values for ${longhand.name}: ` +
        `${JSON.stringify(existing)} and ${JSON.stringify(longhand.value)}`,
      );
    }
    initialLonghandValues.set(longhand.name, longhand.value);
  }
}
const initialValues = [...initialLonghandValues].sort(([left], [right]) =>
  left.localeCompare(right),
);
const extensionVariants = new Map([
  ["aspect-ratio", "AspectRatio"],
  ["browser-longhand", "BrowserLonghand"],
  ["content", "Content"],
  ["gap-rule-longhand", "GapRuleLonghand"],
  ["integer-calculation", "IntegerCalculation"],
  ["length-number-calculation", "LengthNumberCalculation"],
  ["length-percentage-number-calculation", "LengthPercentageNumberCalculation"],
  ["length-percentage-or-number-calculation", "LengthPercentageOrNumberCalculation"],
  ["offset-position", "OffsetPosition"],
  ["offset-rotate", "OffsetRotate"],
  ["page-size", "PageSize"],
  ["webkit-box-reflect", "WebkitBoxReflect"],
  ["webkit-border-image", "WebkitBorderImage"],
  ["webkit-mask-box-image-component", "WebkitMaskBoxImageComponent"],
]);
const extensionsByProperty = new Map();
for (const family of grammarExtensions.families) {
  const variant = extensionVariants.get(family.id);
  if (!variant) throw new Error(`Unknown property grammar extension ${family.id}`);
  const members = new Set(family.properties ?? []);
  for (const suffix of family.propertySuffixes ?? []) {
    for (const property of properties) {
      if (property.endsWith(suffix)) members.add(property);
    }
  }
  for (const property of members) {
    if (!chromiumSupportedProperties.has(property)) {
      throw new Error(`Unknown Chromium property ${property} in ${family.id}`);
    }
    const propertyExtensions = extensionsByProperty.get(property) ?? [];
    propertyExtensions.push(variant);
    extensionsByProperty.set(property, propertyExtensions);
  }
}
const propertyExtensions = [...extensionsByProperty]
  .map(([property, extensions]) => [property, extensions.sort()])
  .sort(([left], [right]) => left.localeCompare(right));
const lines = [
  "// Generated by npm run record:native-properties. Do not edit manually.",
  `pub const SOURCE_SHA256: &str = ${rustString(sourceSha256)};`,
  `pub const INITIAL_VALUES_SOURCE_SHA256: &str = ${rustString(capabilitiesSha256)};`,
  `pub const CHROMIUM_BASELINE: &str = ${rustString(chromiumPropertyBaseline)};`,
  "",
  "pub static SUPPORTED_PROPERTIES: &[&str] = &[",
  ...properties.map(property => `    ${rustString(property)},`),
  "];",
  "",
  "pub static PROPERTY_ALIASES: &[(&str, &str)] = &[",
  ...aliases.map(([alias, canonical]) =>
    `    (${rustString(alias)}, ${rustString(canonical)}),`,
  ),
  "];",
  "",
  "pub static SHORTHAND_LONGHANDS: &[(&str, &[&str])] = &[",
  ...declaredShorthands.flatMap(([shorthand, longhands]) => [
    `    (${rustString(shorthand)}, &[`,
    ...longhands.map(longhand => `        ${rustString(longhand)},`),
    "    ]),",
  ]),
  "];",
  "",
  "pub static SHORTHAND_LONGHAND_BITS: &[u8] = &[",
  ...shorthandLonghandBits.map(bits => `    ${bits},`),
  "];",
  "",
  "pub static OBSERVED_SHORTHAND_LONGHANDS: &[(&str, &[&str])] = &[",
  ...shorthands.flatMap(([shorthand, longhands]) => [
    `    (${rustString(shorthand)}, &[`,
    ...longhands.map(longhand => `        ${rustString(longhand)},`),
    "    ]),",
  ]),
  "];",
  "",
  "pub static INITIAL_LONGHAND_VALUES: &[(&str, &str)] = &[",
  ...initialValues.map(([longhand, value]) =>
    `    (${rustString(longhand)}, ${rustString(value)}),`,
  ),
  "];",
  "",
  "pub static PROPERTY_GRAMMAR_EXTENSIONS: &[(&str, &[super::PropertyGrammarExtension])] = &[",
  ...propertyExtensions.flatMap(([property, extensions]) => [
    `    (${rustString(property)}, &[`,
    ...extensions.map(extension =>
      `        super::PropertyGrammarExtension::${extension},`,
    ),
    "    ]),",
  ]),
  "];",
  "",
];
const serialized = lines.join("\n");

if (mode === "--record") {
  await writeFile(outputUrl, serialized);
  console.log(
    `Recorded ${properties.length} properties, ${aliases.length} aliases, ` +
    `${shorthands.length} shorthand definitions, ${initialValues.length} ` +
    `initial longhand values, and ${propertyExtensions.length} extended ` +
    "property grammars for Rust.",
  );
} else {
  const current = await readFile(outputUrl, "utf8");
  if (current !== serialized) {
    throw new Error(
      "Native property catalog drifted; review the diff and run " +
      "npm run record:native-properties to accept it",
    );
  }
  console.log(`Verified ${properties.length} native Chromium properties.`);
}
