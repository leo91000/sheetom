import assert from "node:assert/strict";
import { createRequire } from "node:module";
import path from "node:path";
import { fileURLToPath } from "node:url";

import { readNativeEngineRevision } from "./native-engine-revision.mjs";

const repositoryRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const nativeDirectory = path.join(repositoryRoot, "native");
const require = createRequire(import.meta.url);
const binding = require(path.join(nativeDirectory, "index.cjs"));
const expectedEngineRevision = await readNativeEngineRevision(repositoryRoot);

assert.equal(binding.nativeEngineRevision(), expectedEngineRevision);

const result = binding.canonicalizeDeclarationBlock(
    "background: image-set(url(a.png) 1x, url(b.png) 2x) center/cover no-repeat red",
);

assert.match(result, /background:/u);
assert.match(result, /image-set\(/u);

const parsedRules = JSON.parse(binding.parseStylesheetTreeJson(
    "@media screen {.x {width:1px;} @supports (display:grid) {.y {color:red;}}}",
    false,
));
assert.equal(parsedRules.length, 1);
assert.equal(parsedRules[0].kind, "media");
assert.equal(parsedRules[0].prelude, "screen");
assert.equal(parsedRules[0].children[0].kind, "style");
assert.equal(parsedRules[0].children[0].declarations, "width:1px");
assert.equal(parsedRules[0].children[1].kind, "supports");

assert.deepEqual(
    JSON.parse(binding.scanTopLevelRulesJson(
        '/* lead */ @unknown fn(a;b) { value: "}"; } .é { --x: "a;b"; } @tail x;',
    )),
    ['@unknown fn(a;b) { value: "}"; }', '.é { --x: "a;b"; }', "@tail x;"],
);

const fontFace = JSON.parse(binding.parseRuleTreeJson(
    "@font-face {font-family:Test;src:local(Test)}",
));
assert.equal(fontFace.kind, "font-face");
assert.equal(fontFace.declarations.includes('local("Test")'), true);
assert.throws(
    () => binding.parseRuleTreeJson(".a{} .b{}"),
    /exactly one rule/u,
);

const recoveredStyle = JSON.parse(binding.parseRecoveredRuleTreeJson(
    ".x { padding: 72px var(--space, var(--space,; }",
));
assert.equal(recoveredStyle.kind, "style");
assert.equal(recoveredStyle.prelude, ".x");
assert.equal(recoveredStyle.declarations, "padding: 72px var(--space, var(--space,;");

const recoveredGroup = JSON.parse(binding.parseRecoveredRuleTreeJson(
    "@layer app { @media (max-width: 767px) { .x:hover { color: red; } } }",
));
assert.equal(recoveredGroup.kind, "layer-block");
assert.equal(recoveredGroup.prelude, "app");
assert.equal(recoveredGroup.children[0].kind, "media");
assert.equal(recoveredGroup.children[0].prelude, "(max-width: 767px)");
assert.equal(recoveredGroup.children[0].children[0].prelude, ".x:hover");

const customFunction = JSON.parse(binding.parseRecoveredRuleTreeJson(
    "@function --mix(--x <number>: 1, --rest type(*)) returns <number> { --local: foo(a;b); result: calc(var(--x) * 2); @supports (width: 100px) { result: 100px; } }",
));
assert.equal(customFunction.kind, "function");
assert.equal(customFunction.prelude, "--mix");
assert.equal(customFunction.declarations, "<number>");
assert.equal(customFunction.children[0].kind, "function-parameter");
assert.equal(customFunction.children[0].children[0].declarations, "1");
assert.equal(customFunction.children[2].kind, "function-declarations");
assert.equal(customFunction.children[3].kind, "supports");
assert.equal(customFunction.children[3].children[0].kind, "function-declarations");

assert.equal(binding.normalizeSelector(":is(.a,.b)>.child"), ":is(.a, .b) > .child");
assert.equal(
    binding.normalizeMedia("screen and (max-width:767px),print"),
    "screen and (max-width: 767px), print",
);
assert.equal(binding.normalizeSupports("(display:grid)"), "(display:grid)");
assert.deepEqual(
    JSON.parse(binding.parseContainerPreludeJson("card (max-width:767px)")),
    {
        conditionText: "card (max-width: 767px)",
        name: "card",
        query: "(max-width: 767px)",
    },
);
assert.deepEqual(
    JSON.parse(binding.parseScopePreludeJson("(.a,.b) to (.c)")),
    { start: ".a, .b", end: ".c" },
);
assert.equal(binding.parseCounterStyleDescriptorValue("system", "fixed"), "fixed 1");
assert.equal(binding.parseCounterStyleDescriptorValue("range", "10 1"), null);
assert.deepEqual(
    JSON.parse(binding.parseCounterStyleDescriptorsJson(
        'system: fixed; symbols: "a" "b"; range: 1 10; symbols: var(--invalid);',
    )),
    [
        { name: "system", value: "fixed 1" },
        { name: "symbols", value: '"a" "b"' },
        { name: "range", value: "1 10" },
    ],
);
assert.deepEqual(JSON.parse(binding.parseCounterStyleNameJson("\\78")), {
    name: "x",
    serialized: "x",
});
assert.equal(binding.parseCounterStyleNameJson("bad name"), null);
assert.equal(binding.serializeIdentifierValue("bad name"), "bad\\ name");
assert.equal(binding.serializeFontFamilyValue('"A B", Test'), '"\\\"A B\\\"", Test');

const fontFeatureValues = JSON.parse(binding.parseRecoveredRuleTreeJson(
    '@font-feature-values Test { @styleset { a: 1; } @styleset { b: 2; a: 3; } }',
));
assert.equal(fontFeatureValues.kind, "font-feature-values");
assert.equal(fontFeatureValues.children[0].kind, "font-feature-map");
assert.deepEqual(
    fontFeatureValues.children[0].children.map(entry => [entry.prelude, entry.declarations]),
    [["a", "3"], ["b", "2"]],
);

const state = new binding.NativeDeclarationState();
assert.equal(state.setProperty("color", "red", ""), "applied");
assert.equal(state.setProperty("overflow", "hidden auto", "important"), "applied");
assert.equal(state.length, 3);
assert.equal(state.item(0), "color");
assert.equal(state.item(1), "overflow-x");
assert.equal(state.item(2), "overflow-y");
assert.equal(state.getPropertyValue("overflow"), "hidden auto");
assert.equal(state.getPropertyPriority("overflow"), "important");
assert.equal(state.setProperty("color", "blue; width: 1px", ""), "invalid-value");
assert.equal(state.getPropertyValue("color"), "red");
assert.equal(state.removeProperty("overflow"), "hidden auto");
assert.equal(state.serializeLonghands(), "color: red;");

state.replaceCssText(
    "width: 1px !important; color: red; width: 2px; height: 3px !important;",
);
assert.deepEqual(
    Array.from({ length: state.length }, (_, index) => state.item(index)),
    ["color", "width", "height"],
);
assert.equal(state.cssText, "color: red; width: 1px !important; height: 3px !important;");

state.replaceCssText("padding: var(--space) !important;");
assert.equal(state.getPropertyValue("padding"), "var(--space)");
assert.equal(state.getPropertyValue("padding-top"), "");
assert.equal(state.getPropertyPriority("padding"), "important");
assert.equal(state.cssText, "padding: var(--space) !important;");
assert.equal(state.serializeSafe(), "padding: var(--space) !important;");

state.clear();
assert.equal(
    state.setProperty("padding", "72px var(--space, var(--space,", ""),
    "applied",
);
assert.equal(state.getPropertyValue("padding"), "72px var(--space, var(--space,");
assert.match(state.serializeSafe(), /^padding: .*\)\);$/u);

console.log("Native foundation loaded through the package loader successfully.");
