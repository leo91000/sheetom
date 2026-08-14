import * as SheetOM from "../src/index.js";

const sheet = new SheetOM.CSSStyleSheet();
const rule = sheet.cssRules[0];
if (rule instanceof SheetOM.CSSStyleRule) {
  rule.style.backgroundColor = "red";
  rule.style.webkitLineClamp = "2";
  rule.style.cssFloat = "inline-start";
  rule.style.float = "inline-start";
  const mutations: readonly SheetOM.CSSDeclarationMutation[] = [
    { kind: "set", property: "color", value: "red" },
    { kind: "remove", property: "margin" },
  ];
  const results: readonly SheetOM.CSSDeclarationMutationResult[] =
    rule.style.applyMutations(mutations);
  void results;

  // @ts-expect-error Unknown names are not part of the pinned browser contract.
  rule.style.notACssProperty = "invalid";
}

// CSSOM creates these objects; SheetOM only exposes their live interfaces.
// @ts-expect-error CSSRule is not publicly constructible.
new SheetOM.CSSRule(0);
// @ts-expect-error CSSRuleList is not publicly constructible.
new SheetOM.CSSRuleList([]);
// @ts-expect-error MediaList is not publicly constructible.
new SheetOM.MediaList();
// @ts-expect-error CSSStyleDeclaration is not publicly constructible.
new SheetOM.CSSStyleDeclaration(null as never);
// @ts-expect-error CSSGroupingRule is not publicly constructible.
new SheetOM.CSSGroupingRule(0);
// @ts-expect-error CSSConditionRule is not publicly constructible.
new SheetOM.CSSConditionRule(0, "screen");
// @ts-expect-error CSSStyleRule is not publicly constructible.
new SheetOM.CSSStyleRule(".example");
// @ts-expect-error CSSMediaRule is not publicly constructible.
new SheetOM.CSSMediaRule("screen");
// @ts-expect-error CSSSupportsRule is not publicly constructible.
new SheetOM.CSSSupportsRule("(display: grid)");
// @ts-expect-error CSSContainerRule is not publicly constructible.
new SheetOM.CSSContainerRule("(width > 1px)");
// @ts-expect-error CSSLayerStatementRule is not publicly constructible.
new SheetOM.CSSLayerStatementRule(["reset"]);
// @ts-expect-error CSSLayerBlockRule is not publicly constructible.
new SheetOM.CSSLayerBlockRule("reset");
// @ts-expect-error CSSScopeRule is not publicly constructible.
new SheetOM.CSSScopeRule(null, null);
// @ts-expect-error CSSStartingStyleRule is not publicly constructible.
new SheetOM.CSSStartingStyleRule();
// @ts-expect-error CSSImportRule is not publicly constructible.
new SheetOM.CSSImportRule("theme.css", "", null, null);
// @ts-expect-error CSSNamespaceRule is not publicly constructible.
new SheetOM.CSSNamespaceRule("urn:test", "", "");
// @ts-expect-error CSSPropertyRule is not publicly constructible.
new SheetOM.CSSPropertyRule("--x", "*", false, "", "");
// @ts-expect-error CSSFontPaletteValuesRule is not publicly constructible.
new SheetOM.CSSFontPaletteValuesRule("--x", "serif", "", "", "");
// @ts-expect-error CSSViewTransitionRule is not publicly constructible.
new SheetOM.CSSViewTransitionRule("auto", [], "");
// @ts-expect-error CSSFontFaceRule is not publicly constructible.
new SheetOM.CSSFontFaceRule();
// @ts-expect-error CSSNestedDeclarations is not publicly constructible.
new SheetOM.CSSNestedDeclarations();
// @ts-expect-error CSSFunctionRule is not publicly constructible.
new SheetOM.CSSFunctionRule("--f", [], "*");
// @ts-expect-error CSSFunctionDeclarations is not publicly constructible.
new SheetOM.CSSFunctionDeclarations();
// @ts-expect-error CSSFunctionDescriptors is not publicly constructible.
new SheetOM.CSSFunctionDescriptors(null as never);
// @ts-expect-error CSSMarginRule is not publicly constructible.
new SheetOM.CSSMarginRule("top-left");
// @ts-expect-error CSSPageRule is not publicly constructible.
new SheetOM.CSSPageRule("");
// @ts-expect-error CSSPositionTryRule is not publicly constructible.
new SheetOM.CSSPositionTryRule("--fallback");
// @ts-expect-error CSSKeyframeRule is not publicly constructible.
new SheetOM.CSSKeyframeRule("0%");
// @ts-expect-error CSSKeyframesRule is not publicly constructible.
new SheetOM.CSSKeyframesRule("fade");
// @ts-expect-error CSSCounterStyleRule is not publicly constructible.
new SheetOM.CSSCounterStyleRule("custom");
// @ts-expect-error CSSFontFeatureValuesMap is not publicly constructible.
new SheetOM.CSSFontFeatureValuesMap();
// @ts-expect-error CSSFontFeatureValuesRule is not publicly constructible.
new SheetOM.CSSFontFeatureValuesRule("Inter");
