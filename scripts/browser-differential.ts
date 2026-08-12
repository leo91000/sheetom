import assert from "node:assert/strict";
import fc from "fast-check";
import { chromium, firefox, webkit } from "playwright";

import {
  CSSStyleRule,
  CSSStyleSheet,
} from "../dist/index.js";

const trackedProperties = [
  "--token",
  "background-image",
  "color",
  "font-family",
  "overflow-x",
  "overflow-y",
  "padding-bottom",
  "padding-left",
  "padding-right",
  "padding-top",
  "transform",
  "width",
];

function setMutation(name, values, priorities = ["", "important", "bogus"]) {
  return fc.record({
    operation: fc.constant("set"),
    name: fc.constant(name),
    value: fc.constantFrom(...values),
    priority: fc.constantFrom(...priorities),
  });
}

const mutation = fc.oneof(
  setMutation("width", [
    "0",
    "10px",
    "calc(1px + 2px)",
    "calc(1px",
    "var(--token, 10px)",
    "10px; color:red",
  ]),
  setMutation("color", [
    "red",
    "rgb(1 2 3 / 50%)",
    "rgb(1 2 3",
    "red/*comment",
    "var(--token, rgb(1 2 3))",
    "red !important",
  ]),
  setMutation("font-family", ["serif"]),
  setMutation("background-image", [
    "none",
    'url("https://example.com/a:b")',
    "url(foo",
    "linear-gradient(red, blue",
  ]),
  setMutation("transform", ["none", "translateX(1px)", "translateX(1px"]),
  setMutation("padding", ["1px", "1px 2px", "var(--token, 1px)"]),
  setMutation("overflow", ["auto", "hidden", "clip scroll"]),
  setMutation("--token", [
    "red",
    '"a;b"',
    "func(a;b)",
    "foo\\!bar",
    "url(foo!bar)",
    "var(--fallback, red)",
  ], [""]),
  fc.record({
    operation: fc.constant("remove"),
    name: fc.constantFrom(...trackedProperties, "padding"),
  }),
);
const sequence = fc.array(mutation, { minLength: 1, maxLength: 20 });

function observe(style) {
  return {
    cssText: style.cssText,
    length: style.length,
    items: Array.from({ length: style.length }, (_, index) => style.item(index)),
    values: Object.fromEntries(
      trackedProperties.map(name => [name, style.getPropertyValue(name)]),
    ),
    priorities: Object.fromEntries(
      trackedProperties.map(name => [name, style.getPropertyPriority(name)]),
    ),
  };
}

function applyMutation(style, candidate) {
  if (candidate.operation === "remove") {
    style.removeProperty(candidate.name);
    return;
  }
  style.setProperty(candidate.name, candidate.value, candidate.priority);
}

function sheetOMObservations(mutations) {
  const sheet = new CSSStyleSheet();
  sheet.insertRule(".fuzz {}");
  const rule = sheet.cssRules[0];
  if (!(rule instanceof CSSStyleRule)) throw new TypeError("Expected style rule");
  const observations = [];
  for (const candidate of mutations) {
    applyMutation(rule.style, candidate);
    observations.push(observe(rule.style));
  }
  return observations;
}

async function nativeObservations(page, mutations) {
  return page.evaluate(({ candidates, properties }) => {
    const style = document.createElement("div").style;
    const observations = [];
    for (const candidate of candidates) {
      if (candidate.operation === "remove") {
        style.removeProperty(candidate.name);
      } else {
        style.setProperty(candidate.name, candidate.value, candidate.priority);
      }
      observations.push({
        cssText: style.cssText,
        length: style.length,
        items: Array.from({ length: style.length }, (_, index) => style.item(index)),
        values: Object.fromEntries(
          properties.map(name => [name, style.getPropertyValue(name)]),
        ),
        priorities: Object.fromEntries(
          properties.map(name => [name, style.getPropertyPriority(name)]),
        ),
      });
    }
    return observations;
  }, { candidates: mutations, properties: trackedProperties });
}

async function verifyNativeReparsing(pages) {
  const sheet = new CSSStyleSheet();
  sheet.insertRule(".recovered { color: green; }", 0);
  sheet.insertRule(".following { color: blue; }", 1);
  const recovered = sheet.cssRules[0];
  if (!(recovered instanceof CSSStyleRule)) throw new TypeError("Expected style rule");
  recovered.style.setProperty("padding", "72px var(--space, var(--space,");
  const serialized = sheet.serialize();

  const substitutionCases = [
    { property: "content", malformed: '"hello' },
    { property: "width", malformed: "calc(10px" },
    { property: "background-image", malformed: "linear-gradient(red, blue" },
  ];
  const substitutionSheet = new CSSStyleSheet();
  for (const [index, candidate] of substitutionCases.entries()) {
    substitutionSheet.insertRule(`.sheetom-substitution-${index} {}`, index);
    const rule = substitutionSheet.cssRules[index];
    if (!(rule instanceof CSSStyleRule)) throw new TypeError("Expected style rule");
    rule.style.setProperty("--x", candidate.malformed);
    rule.style.setProperty(candidate.property, "var(--x)");
  }
  const substitutionCSS = substitutionSheet.serialize();

  for (const [engine, page] of pages) {
    const result = await page.evaluate(({ css, cases, customCSS }) => {
      const nativeSheet = new globalThis.CSSStyleSheet();
      nativeSheet.replaceSync(css);
      const nativeRecovered = nativeSheet.cssRules[0];
      const nativeFollowing = nativeSheet.cssRules[1];
      const reparsed = {
        ruleCount: nativeSheet.cssRules.length,
        color: nativeRecovered.style.getPropertyValue("color"),
        padding: nativeRecovered.style.getPropertyValue("padding"),
        followingColor: nativeFollowing.style.getPropertyValue("color"),
      };

      const style = document.createElement("style");
      style.textContent = customCSS;
      document.head.append(style);
      const substitutions = [];
      const elements = [];
      try {
        for (const [index, candidate] of cases.entries()) {
          const reference = document.createElement("div");
          reference.style.setProperty("--x", candidate.malformed);
          reference.style.setProperty(candidate.property, "var(--x)");
          const candidateElement = document.createElement("div");
          candidateElement.className = `sheetom-substitution-${index}`;
          document.body.append(reference, candidateElement);
          elements.push(reference, candidateElement);
          substitutions.push({
            property: candidate.property,
            reference: getComputedStyle(reference).getPropertyValue(candidate.property),
            serialized: getComputedStyle(candidateElement).getPropertyValue(candidate.property),
          });
        }
      } finally {
        style.remove();
        for (const element of elements) element.remove();
      }
      return { reparsed, substitutions };
    }, { css: serialized, cases: substitutionCases, customCSS: substitutionCSS });

    assert.equal(result.reparsed.ruleCount, 2, `${engine}: native rule count`);
    assert.equal(result.reparsed.color, "green", `${engine}: recovered color`);
    assert.notEqual(result.reparsed.padding, "", `${engine}: recovered padding`);
    assert.equal(
      result.reparsed.followingColor,
      "rgb(0, 0, 255)",
      `${engine}: following rule`,
    );
    for (const substitution of result.substitutions) {
      assert.equal(
        substitution.serialized,
        substitution.reference,
        `${engine}: ${substitution.property} substitution`,
      );
    }
  }
}

async function verifyShorthandRenderingWitness(page) {
  const cases = [
    [
      "background",
      "linear-gradient(red, blue) center / cover no-repeat, green",
      "background-color",
      "blue",
      "background-image",
    ],
    [
      "mask",
      "linear-gradient(black, transparent) center / cover no-repeat",
      "mask-size",
      "10px",
      "mask-image",
    ],
    [
      "border-image",
      "linear-gradient(red, blue) 30 / 10 / 0 stretch",
      "border-image-slice",
      "20",
      "border-image-source",
    ],
    [
      "grid-template",
      '"a a" 100px "b c" 1fr / 1fr 2fr',
      "grid-template-rows",
      "20px",
      "grid-template-columns",
    ],
    ["font", "italic 16px serif", "font-size", "20px", "font-size"],
    ["animation", "1s linear foo", "animation-duration", "2s", "animation-duration"],
    ["transition", "color 1s linear", "transition-duration", "2s", "transition-duration"],
    [
      "offset",
      'path("M 0 0 L 100 100") 50% auto 45deg / center',
      "offset-distance",
      "20%",
      "offset-path",
    ],
  ];
  const sheet = new CSSStyleSheet();
  for (const [index, [shorthand, value, longhand, override]] of cases.entries()) {
    sheet.insertRule(`.sheetom-shorthand-${index} {}`, index);
    const rule = sheet.cssRules[index];
    if (!(rule instanceof CSSStyleRule)) throw new TypeError("Expected style rule");
    rule.style.setProperty(shorthand, value);
    rule.style.setProperty(longhand, override);
    rule.style.removeProperty(longhand);
  }

  const observations = await page.evaluate(({ css, definitions }) => {
    const style = document.createElement("style");
    style.textContent = css;
    document.head.append(style);
    const results = [];
    const elements = [];
    try {
      for (const [index, [shorthand, value, longhand, override, witness]] of definitions.entries()) {
        const reference = document.createElement("div");
        reference.style.setProperty(shorthand, value);
        reference.style.setProperty(longhand, override);
        reference.style.removeProperty(longhand);
        const candidate = document.createElement("div");
        candidate.className = `sheetom-shorthand-${index}`;
        document.body.append(reference, candidate);
        elements.push(reference, candidate);
        results.push({
          shorthand,
          reference: getComputedStyle(reference).getPropertyValue(witness),
          serialized: getComputedStyle(candidate).getPropertyValue(witness),
        });
      }
    } finally {
      style.remove();
      for (const element of elements) element.remove();
    }
    return results;
  }, { css: sheet.serialize(), definitions: cases });

  for (const observation of observations) {
    assert.equal(
      observation.serialized,
      observation.reference,
      `Chromium rendering witness for ${observation.shorthand}`,
    );
  }
}

const availableBrowserTypes = { chromium, firefox, webkit };
const requestedBrowsers = (process.env.SHEETOM_DIFFERENTIAL_BROWSERS ?? "chromium,firefox,webkit")
  .split(",")
  .map(name => name.trim())
  .filter(Boolean);
for (const name of requestedBrowsers) {
  if (!Object.hasOwn(availableBrowserTypes, name)) {
    throw new Error(`Unsupported differential browser: ${name}`);
  }
}

const browsers = new Map();
try {
  for (const name of requestedBrowsers) {
    const browserType = availableBrowserTypes[name];
    const browser = await browserType.launch({ headless: true });
    browsers.set(name, browser);
  }
  const pages = new Map();
  for (const [name, browser] of browsers) pages.set(name, await browser.newPage());

  await fc.assert(fc.asyncProperty(sequence, async mutations => {
    const sheetOM = sheetOMObservations(mutations);
    const native = new Map();
    for (const [engine, page] of pages) {
      native.set(engine, await nativeObservations(page, mutations));
    }
    const [firstEngine, firstObservations] = native.entries().next().value;
    for (const [engine, observations] of native) {
      assert.deepEqual(
        observations,
        firstObservations,
        `Native divergence between ${firstEngine} and ${engine}; promote the minimized sequence to an Operation Fixture`,
      );
    }
    assert.deepEqual(
      sheetOM,
      firstObservations,
      `SheetOM differs from the ${firstEngine} native consensus`,
    );
  }), {
    seed: 0x5e37_0b,
    numRuns: Number.parseInt(process.env.SHEETOM_DIFFERENTIAL_RUNS ?? "100", 10),
  });

  await verifyNativeReparsing(pages);
  await verifyShorthandRenderingWitness(pages.get("chromium"));
  console.log("Browser differential and native reparsing evidence passed.");
} finally {
  for (const browser of browsers.values()) await browser.close();
}
