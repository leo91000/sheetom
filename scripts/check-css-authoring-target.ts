import assert from "node:assert/strict";
import { readFile, writeFile } from "node:fs/promises";
import { chromium, firefox } from "playwright";
import corpus from "../compatibility/css-authoring-probes.json" with { type: "json" };
import propertyCorpus from "../compatibility/webref-property-branches.json" with { type: "json" };
import { assertAuthoringRoundTrip, snapshotAuthoringRule as snapshotRule } from "./css-authoring-roundtrip.ts";

const requestedBackend = process.argv.find(argument => argument.startsWith("--backend="))?.slice(10);
assert.ok(requestedBackend === undefined || ["native", "wasm"].includes(requestedBackend));
const backends = [];
if (requestedBackend !== "wasm") backends.push(["native", await import("../dist/index.js")]);
if (requestedBackend !== "native") backends.push(["wasm", await (await import("../packages/wasm/dist/index.js")).createSheetOM(new Uint8Array(await readFile(new URL("../packages/wasm/dist/sheetom_wasm_bg.wasm", import.meta.url))).buffer)]);
// This function is also executed verbatim in Chromium. Snapshots cover authored
// state, invalid-mutation atomicity, parentage, and live collection identity.

function observe(api, probe) {
  const sheet = new api.CSSStyleSheet();
  const snapshots = [];
  if (probe.kind === "rule") sheet.replaceSync(probe.value);
  else {
    sheet.replaceSync(".probe { color: blue; }");
    const rule = sheet.cssRules[0];
    if (probe.kind === "selector") rule.selectorText = probe.value;
    else rule.style.setProperty(probe.property, probe.value, "important");
    snapshots.push(snapshotRule(rule));
    if (probe.kind === "selector") rule.selectorText = ":not(";
    else rule.style.setProperty(probe.property, `${probe.value}; color: green`, "important");
    snapshots.push(snapshotRule(rule));
  }
  snapshots.push(Array.from(sheet.cssRules, snapshotRule));
  const list = sheet.cssRules;
  if (sheet.cssRules.length) {
    const rule = sheet.cssRules[sheet.cssRules.length - 1];
    const attached = rule.parentStyleSheet === sheet && rule.parentRule === null;
    sheet.deleteRule(sheet.cssRules.length - 1);
    snapshots.push({ attached, detached: rule.parentStyleSheet === null, sameList: sheet.cssRules === list });
  }
  return snapshots;
}
const browser = await chromium.launch();
const mediaBrowser = await firefox.launch();
try {
  assert.equal(browser.version(), "151.0.7922.34", "Review the pinned browser version before advancing evidence");
  const page = await browser.newPage();
  const mediaPage = await mediaBrowser.newPage();
  assert.equal(mediaBrowser.version(), "153.0", "Review the media selector oracle before advancing evidence");
  const mismatches = [];
  const evaluator = `(function () { const snapshotRule = ${snapshotRule.toString()}; return ${observe.toString()}; })()`;
  for (const probe of corpus.probes) {
    const reference = probe.oracle === "firefox" ? mediaPage : page;
    const expected = await reference.evaluate(({ evaluator, probe }) => (0, eval)(`(${evaluator})`)(globalThis, probe), { evaluator, probe });
    for (const [backend, api] of backends) {
      const actual = observe(api, probe);
      try { assert.deepEqual(actual, expected); } catch { mismatches.push({ id: probe.id, backend, actual, expected }); }
      const sheet = new api.CSSStyleSheet();
      sheet.replaceSync(probe.kind === "rule" ? probe.value : probe.kind === "selector" ? `${probe.value} { color: red; }` : `.probe { ${probe.property}: ${probe.value}; }`);
      const serialized = sheet.serializeStrict();
      const reparse = new api.CSSStyleSheet(); reparse.replaceSync(serialized);
      try {
        assertAuthoringRoundTrip(api, sheet, reparse, snapshotRule);
      } catch {
        mismatches.push({ id: probe.id, backend, roundTripState: {
          before: Array.from(sheet.cssRules, snapshotRule),
          after: Array.from(reparse.cssRules, snapshotRule),
          serialized,
        } });
      }
      try { assert.equal(reparse.serializeStrict(), serialized); } catch { mismatches.push({ id: probe.id, backend, roundTrip: serialized }); }
    }
  }
  const declarations = corpus.probes.filter(probe => probe.kind === "declaration").map(probe => [probe.property, probe.value]).concat(propertyCorpus.profiles.flatMap(profile => profile.properties.flatMap(property => profile.samples.map(sample => [property, sample.input]))));
  const conditions = ["color: red", "(color: red !important)", "not (color:red;)", "not(color:red)", "not (invalid:value)", "(color:red) and (width:1px) or (height:1px)", "(color:red) and", "(color:red) trailing", "(color:red) or (width:invalid)", ...corpus.probes.filter(p => p.kind === "selector").map(p => `selector(${p.value})`), "selector(:is(div, :unknown))", "selector(:lang(en, fr))", "not (unknown(\"bad\nstring\"))", "not (color: red])", "selector(div, p)", "selector(:unknown)", ...["woff", "woff2", "truetype", "opentype", "collection", "svg", "embedded-opentype", "unknown"].map(name => `font-format(${name})`), ...["features-opentype", "features-aat", "features-graphite", "variations", "palettes", "color-colrv0", "color-colrv1", "color-sbix", "color-cbdt", "color-svg", "incremental", "unknown"].map(name => `font-tech(${name})`)];
  for (const depth of [80, 4000]) for (const selector of [":is(.a)", ":not(.a)", ":has(> .a)", ":is(.a, :unknown)", ":has(:has(.a))", ":nth-child(2n of .a)", ":where([data-x=odd])"]) conditions.push(`selector(${":is(".repeat(depth)}${selector}${")".repeat(depth)})`);
  const expectedSupports = await page.evaluate(({ declarations, conditions }) => [...declarations.map(([p, v]) => CSS.supports(p, v)), ...conditions.map(c => CSS.supports(c))], { declarations, conditions });
  // Chromium 151 predates Baseline media selectors. Use an explicit positive
  // Firefox reference for these conditions, keeping the other oracles pinned.
  for (const probe of corpus.probes.filter(p => p.oracle === "firefox" && p.kind === "selector")) {
    const condition = `selector(${probe.value})`;
    const index = conditions.indexOf(condition);
    expectedSupports[declarations.length + index] = await mediaPage.evaluate(value => CSS.supports(value), condition);
  }
  for (const [backend, api] of backends) {
    const actual = [...declarations.map(([p, v]) => api.CSS.supports(p, v)), ...conditions.map(c => api.CSS.supports(c))];
    for (let i = 0; i < actual.length; i++) if (actual[i] !== expectedSupports[i]) mismatches.push({ backend, supports: i < declarations.length ? declarations[i] : conditions[i - declarations.length], actual: actual[i], expected: expectedSupports[i] });
  }
  const identifiers = [...Array.from({ length: 256 }, (_, index) => String.fromCharCode(index)), "-", "-0x", "123", "é🐕", "\uD800", "\uDC00", "a\uD800b", "--name"];
  const expectedEscapes = await page.evaluate(values => values.map(value => CSS.escape(value)), identifiers);
  for (const [, api] of backends) assert.deepEqual(identifiers.map(value => api.CSS.escape(value)), expectedEscapes);
  const report = { browser: browser.version(), mediaSelectorBrowser: mediaBrowser.version(), probes: corpus.probes.length, supportsChecksPerBackend: expectedSupports.length, escapeChecksPerBackend: identifiers.length, mismatches };
  await writeFile(new URL("../target/css-authoring-target-report.json", import.meta.url), `${JSON.stringify(report, null, 2)}\n`);
  console.log(JSON.stringify({ ...report, mismatches: mismatches.length }));
  assert.equal(mismatches.length, 0, "See target/css-authoring-target-report.json");
} finally { await browser.close(); await mediaBrowser.close(); }
