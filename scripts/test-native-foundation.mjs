import assert from "node:assert/strict";
import { createRequire } from "node:module";
import path from "node:path";
import { fileURLToPath } from "node:url";

const repositoryRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const nativeDirectory = path.join(repositoryRoot, "native");
const require = createRequire(import.meta.url);
const binding = require(path.join(nativeDirectory, "index.cjs"));

assert.equal(binding.nativeEngineRevision(), "lightningcss-1.33.0-c6a0c3ce-sheetom.5");

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
