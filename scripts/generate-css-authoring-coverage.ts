import assert from "node:assert/strict";
import { access, readFile, writeFile } from "node:fs/promises";
import target from "../compatibility/css-feature-target.json" with { type: "json" };
import probes from "../compatibility/css-authoring-probes.json" with { type: "json" };
import properties from "../compatibility/webref-property-branches.json" with { type: "json" };

const excludedApi = new Set(["CSSAnimation", "CSSTransition", "StyleSheetList"]);
const apiSuites = {
  CSS: ["tests/css-namespace.test.ts", "scripts/check-css-authoring-target.ts"],
  StyleSheet: ["tests/css-namespace.test.ts", "tests/stylesheet-metadata.test.ts"],
  CSSStyleSheet: ["tests/replace.test.ts", "tests/insert-rule.test.ts", "tests/stylesheet-metadata.test.ts"],
  CSSStyleDeclaration: ["tests/declaration-css-text.test.ts", "tests/webidl-conversion.test.ts", "tests/custom-properties.test.ts"],
  CSSRule: ["tests/rule-webidl.test.ts", "tests/rule-identity.test.ts"],
  CSSRuleList: ["tests/rule-webidl.test.ts", "tests/rule-identity.test.ts"],
  CSSStyleRule: ["tests/selector-text.test.ts", "tests/grouping-rules.test.ts"],
  CSSGroupingRule: ["tests/grouping-rules.test.ts", "tests/rule-identity.test.ts"],
  CSSConditionRule: ["tests/grouping-rules.test.ts"],
  CSSMediaRule: ["tests/grouping-rules.test.ts"],
  MediaList: ["tests/grouping-rules.test.ts", "tests/webidl-conversion.test.ts"],
  CSSSupportsRule: ["tests/grouping-rules.test.ts"],
  CSSContainerRule: ["tests/grouping-rules.test.ts"],
  CSSScopeRule: ["tests/grouping-rules.test.ts"],
  CSSStartingStyleRule: ["tests/grouping-rules.test.ts"],
  CSSLayerBlockRule: ["tests/grouping-rules.test.ts"],
  CSSLayerStatementRule: ["tests/static-rule-interfaces.test.ts"],
  CSSKeyframeRule: ["tests/keyframes.test.ts"],
  CSSKeyframesRule: ["tests/keyframes.test.ts"],
  CSSCounterStyleRule: ["tests/descriptor-rules.test.ts"],
  CSSPageRule: ["tests/declaration-rules.test.ts"],
  CSSFontFaceRule: ["tests/declaration-rules.test.ts"],
  CSSFontFeatureValuesRule: ["tests/descriptor-rules.test.ts"],
  CSSFontPaletteValuesRule: ["tests/static-rule-interfaces.test.ts"],
  CSSImportRule: ["tests/static-rule-interfaces.test.ts", "tests/stylesheet-metadata.test.ts"],
  CSSNamespaceRule: ["tests/static-rule-interfaces.test.ts", "tests/selector-text.test.ts"],
  CSSNestedDeclarations: ["tests/grouping-rules.test.ts"],
  CSSPositionTryRule: ["tests/baseline-september.test.ts"],
  CSSPositionTryDescriptors: ["tests/baseline-september.test.ts"],
  CSSPropertyRule: ["tests/static-rule-interfaces.test.ts"],
};
const surfaces = new Map();
const features = [];
for (const feature of target.features) {
  const dispositions = [];
  for (const branch of feature.branches) {
    if (!branch.eligible) { dispositions.push({ key: branch.key, disposition: branch.eligibility }); continue; }
    const [namespace, category, name] = branch.key.split(".");
    if (namespace !== "css" && !(namespace === "api" && /^(CSS|StyleSheet|MediaList)/u.test(category))
      || namespace === "api" && (excludedApi.has(category) || ["highlights_static", "registerProperty_static"].includes(name))) {
      dispositions.push({ key: branch.key, disposition: "dom-or-evaluation", reason: "DOM association, document registry, browser collection, rendering, or animation execution; accompanying CSS syntax is audited separately." });
      continue;
    }
    const root = namespace === "api" ? `api.${category}` : `css.${category}.${name}`;
    if (!surfaces.has(root)) {
      const evidence = [];
      const profileIds = [];
      const probeIds = probes.probes.filter(probe => probe.root === root).map(probe => probe.id);
      if (namespace === "api") {
        assert.ok(apiSuites[category], `Unmapped API ${branch.key}`);
        evidence.push(...apiSuites[category], "scripts/test-wasm-backend.ts");
      } else if (category === "properties") {
        for (const profile of properties.profiles) if (profile.properties.includes(name)) profileIds.push(profile.id);
        assert.ok(profileIds.length || name === "custom-property", `Unmapped property ${branch.key}`);
        evidence.push("scripts/check-webref-property-branches.ts");
        if (name === "custom-property") evidence.push("tests/custom-properties.test.ts");
      } else {
        assert.ok(probeIds.length, `Unmapped syntax ${branch.key}`);
        evidence.push("scripts/check-css-authoring-target.ts");
        if (category === "at-rules") evidence.push("scripts/rule-browser-differential.ts");
      }
      if (probeIds.length) evidence.push("scripts/check-css-authoring-target.ts");
      surfaces.set(root, { root, evidence: [...new Set(evidence)], profileIds, probeIds });
    }
    dispositions.push({ key: branch.key, disposition: "authoring-surface", surface: root });
  }
  features.push({ id: feature.id, branches: dispositions });
}
for (const surface of surfaces.values()) for (const file of surface.evidence) await access(new URL(`../${file}`, import.meta.url));
const summary = { families: features.length, authoringSurfaces: surfaces.size, authoringKeyRecords: features.flatMap(f => f.branches).filter(b => b.disposition === "authoring-surface").length, domOrEvaluationKeyRecords: features.flatMap(f => f.branches).filter(b => b.disposition === "dom-or-evaluation").length, deferredKeyRecords: target.summary.deferredKeyRecords };
const output = { schemaVersion: 1, cutoff: target.cutoff, evidenceGranularity: "Authoring surfaces: properties link to executable grammar profiles; selector/type/rule roots link to operation probes; APIs link to operation suites. BCD rendering subkeys do not imply evaluation support or a standalone test per compatibility key.", summary, surfaces: [...surfaces.values()].sort((a, b) => a.root.localeCompare(b.root, "en")), features };
const url = new URL("../compatibility/css-authoring-coverage.json", import.meta.url);
const serialized = `${JSON.stringify(output, null, 2)}\n`;
if (process.argv.includes("--record")) await writeFile(url, serialized);
else assert.equal(await readFile(url, "utf8"), serialized, "CSS authoring evidence mapping is stale");
console.log(`Verified CSS authoring evidence mapping: ${JSON.stringify(summary)}. Execution results are separate from this mapping.`);
