import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { readFile, writeFile } from "node:fs/promises";

const targetUrl = new URL("../compatibility/css-feature-target.json", import.meta.url);
const cutoff = "2026-09-09";
const sourceSha256 = "7633fe15d2ac393c69682150cd2e20b1164f4b8396fd9ecbc8226e25777dcac0";
const beforeCutoff = status => ["low", "high"].includes(status?.baseline)
  && typeof status.baseline_low_date === "string" && status.baseline_low_date <= cutoff;

function summarize(features) {
  const keys = new Set(features.flatMap(feature => feature.branches.map(branch => branch.key)));
  return {
    featureFamilies: features.length,
    uniqueCompatibilityKeys: keys.size,
    eligibleKeyRecords: features.reduce((sum, feature) => sum + feature.branches.filter(branch => branch.eligible).length, 0),
    deferredKeyRecords: features.reduce((sum, feature) => sum + feature.branches.filter(branch => !branch.eligible).length, 0),
  };
}

const sourcePath = process.argv.find(argument => argument.startsWith("--source="))?.slice(9);
if (sourcePath) {
  const bytes = await readFile(sourcePath);
  assert.equal(createHash("sha256").update(bytes).digest("hex"), sourceSha256, "unrecognized WebDX source");
  const data = JSON.parse(bytes.toString("utf8"));
  const cssGroup = id => id === "css" || Boolean(data.groups[id]?.parent && cssGroup(data.groups[id].parent));
  const features = Object.entries(data.features).filter(([, feature]) =>
    feature.kind === "feature" && (beforeCutoff(feature.status)
      || (feature.compat_features ?? []).some(key => beforeCutoff(feature.status?.by_compat_key?.[key]))) && (
      (feature.group ?? []).some(cssGroup)
      || (feature.compat_features ?? []).some(key => key.startsWith("css.") || /^api\.(CSS|StyleSheet|MediaList)/.test(key))
    )
  ).sort(([a], [b]) => a.localeCompare(b, "en")).map(([id, feature]) => ({
    id,
    name: feature.name,
    baselineLowDate: feature.status.baseline_low_date ?? null,
    specifications: feature.spec ?? [],
    branches: [...(feature.compat_features ?? [])].sort().map(key => {
      const status = feature.status.by_compat_key?.[key];
      return {
        key,
        baseline: status?.baseline ?? null,
        baselineLowDate: status?.baseline_low_date ?? null,
        eligible: beforeCutoff(status),
        eligibility: !status ? "undated" : beforeCutoff(status) ? "within-cutoff"
          : status.baseline_low_date > cutoff ? "after-cutoff" : "not-baseline",
      };
    }),
  }));
  const manifest = {
    schemaVersion: 1,
    cutoff,
    source: {
      package: "web-features", version: "3.37.0",
      revision: "60e8982caeaa28c4913f45c2cb20b5f6114de510",
      url: "https://registry.npmjs.org/web-features/-/web-features-3.37.0.tgz",
      dataSha256: sourceSha256,
    },
    experimental: {
      features: ["mixin", "function"],
      mixinDraftRevision: "5e68d5c1ca6656dd7a4b32f821d187aa9652ad5d",
      mixinDraftSha256: "4967c197dfd1b86cb321b835ebcdb2577974bc80d7097294294cd75a8249a76c",
    },
    summary: summarize(features),
    features,
  };
  const expected = `${JSON.stringify(manifest, null, 2)}\n`;
  if (process.argv.includes("--record")) await writeFile(targetUrl, expected);
  else assert.equal(await readFile(targetUrl, "utf8"), expected, "CSS target differs from the pinned source");
}

const manifest = JSON.parse(await readFile(targetUrl, "utf8"));
assert.equal(manifest.schemaVersion, 1);
assert.equal(manifest.cutoff, cutoff);
assert.equal(manifest.source.dataSha256, sourceSha256);
assert.equal(manifest.source.version, "3.37.0");
assert.equal(manifest.features.length, 328);
assert.equal(new Set(manifest.features.map(feature => feature.id)).size, manifest.features.length);
for (const feature of manifest.features) {
  assert.ok(feature.branches.some(branch => branch.eligible) || feature.baselineLowDate && feature.baselineLowDate <= cutoff, feature.id);
  assert.equal(new Set(feature.branches.map(branch => branch.key)).size, feature.branches.length);
  for (const branch of feature.branches) {
    assert.equal(branch.eligible, beforeCutoff({ baseline: branch.baseline, baseline_low_date: branch.baselineLowDate }), branch.key);
  }
}
assert.deepEqual(manifest.summary, summarize(manifest.features));
console.log(`Verified dated CSS target: ${JSON.stringify(manifest.summary)}. Eligibility is not conformance evidence.`);
