import { createHash } from "node:crypto";
import { readFile, writeFile } from "node:fs/promises";

import { chromium } from "playwright";

import {
  chromiumPropertyBaseline,
  chromiumSupportedProperties,
} from "../src/chromium-properties.ts";

const probesUrl = new URL("../compatibility/property-value-probes.json", import.meta.url);
const observationsUrl = new URL(
  "../compatibility/property-value-observations.json",
  import.meta.url,
);
const mode = process.argv[2] ?? "--check";
if (!["--check", "--record"].includes(mode)) {
  throw new Error("Usage: generate-property-value-observations.ts [--check|--record]");
}

const probesBytes = await readFile(probesUrl);
const probes = JSON.parse(probesBytes.toString("utf8"));
const properties = [...chromiumSupportedProperties];
const browser = await chromium.launch({ headless: true });
let browserResult;
try {
  const page = await browser.newPage();
  await page.setContent("<!doctype html><title>SheetOM property oracle</title>");
  browserResult = await page.evaluate(({ names, values }) => {
    const style = document.createElement("div").style;
    const accepted = [];
    let rejectedCount = 0;
    let atomicNoOpCount = 0;
    const observe = () => ({
      cssText: style.cssText,
      items: Array.from(style, property => ({
        property,
        value: style.getPropertyValue(property),
        priority: style.getPropertyPriority(property),
      })),
    });
    for (const property of names) {
      for (const probe of values) {
        style.cssText = "";
        style.setProperty(property, probe.input);
        if (style.length > 0) {
          accepted.push([
            property,
            probe.id,
            style.getPropertyValue(property),
            style.cssText,
            Array.from(style),
          ]);
          continue;
        }

        rejectedCount += 1;
        style.setProperty(property, "initial");
        const before = JSON.stringify(observe());
        style.setProperty(property, probe.input);
        if (JSON.stringify(observe()) === before) atomicNoOpCount += 1;
      }
    }
    return {
      userAgent: navigator.userAgent,
      compatMode: document.compatMode,
      accepted,
      rejectedCount,
      atomicNoOpCount,
    };
  }, { names: properties, values: probes.values });
} finally {
  await browser.close();
}

const baselineMajor = chromiumPropertyBaseline.match(/Chrome\/(\d+)/u)?.[1];
const observedMajor = browserResult.userAgent.match(/Chrome\/(\d+)/u)?.[1];
if (!baselineMajor || observedMajor !== baselineMajor) {
  throw new Error(`Unexpected Chromium oracle: ${browserResult.userAgent}`);
}
if (browserResult.compatMode !== "CSS1Compat") {
  throw new Error(`Property oracle must run in standards mode, got ${browserResult.compatMode}`);
}
if (browserResult.atomicNoOpCount !== browserResult.rejectedCount) {
  throw new Error(
    `Chromium mutated ${browserResult.rejectedCount - browserResult.atomicNoOpCount} ` +
    "properties after rejecting a Property Value probe",
  );
}

const observations = {
  $schema: "./schemas/property-value-observations.schema.json",
  schemaVersion: 1,
  baseline: {
    browser: "chromium",
    userAgent: browserResult.userAgent,
    compatMode: browserResult.compatMode,
    propertyCount: properties.length,
    probeCount: probes.values.length,
    acceptedCount: browserResult.accepted.length,
    rejectedCount: browserResult.rejectedCount,
    atomicNoOpCount: browserResult.atomicNoOpCount,
    probesSha256: createHash("sha256").update(probesBytes).digest("hex"),
  },
  accepted: browserResult.accepted,
};
const serialized = `${JSON.stringify({ ...observations, accepted: [] }, null, 2).replace(
  '"accepted": []',
  `"accepted": [\n${observations.accepted
    .map(candidate => `    ${JSON.stringify(candidate)}`)
    .join(",\n")}\n  ]`,
)}\n`;
if (mode === "--record") {
  await writeFile(observationsUrl, serialized);
  console.log(`Recorded ${browserResult.accepted.length} accepted property/value probes.`);
} else {
  const current = await readFile(observationsUrl, "utf8");
  if (current !== serialized) {
    throw new Error("Property Value Probe observations drifted; record and review the diff");
  }
  console.log(`Verified ${browserResult.accepted.length} accepted property/value probes.`);
}
