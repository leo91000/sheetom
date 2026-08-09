import { transform } from "lightningcss";
import * as csstree from "css-tree";
import {
  DeclarationBlock,
  type ParsedDeclaration,
  type ParsedPropertyValue,
} from "./internal/declaration-block.js";
import { scanTopLevelRules } from "./internal/css-rule-scanner.js";
import { RuleTree } from "./internal/rule-tree.js";
import {
  Serializer,
  type RuleSerializationPlan,
} from "./internal/serializer.js";
import {
  expandStaticFourSide,
  expandStaticShorthand,
  getShorthandLonghands,
  isFourSideShorthand,
} from "./internal/shorthand-registry.js";
import {
  parseAtruleDescriptorValue,
  parsePropertyValue,
  serializeIdentifier,
} from "./internal/value-gate.js";
import {
  assertInternalConstructor,
  constructInternally,
} from "./internal/webidl-construction.js";

import {
  chromiumSupportedProperties,
} from "./chromium-properties.js";

const encoder = new TextEncoder();
const decoder = new TextDecoder();
const arrayIndexPattern = /^(0|[1-9]\d*)$/;
const regularSheetMetadata = new WeakMap<
  object,
  { href: string | null; baseURL: string }
>();
const ruleParentage = new WeakMap<
  object,
  { parentRule: CSSRule | null; parentStyleSheet: CSSStyleSheet | null }
>();
const unsignedLongRange = 2 ** 32;
const pageMarginRuleNames = new Set([
  "top-left-corner",
  "top-left",
  "top-center",
  "top-right",
  "top-right-corner",
  "right-top",
  "right-middle",
  "right-bottom",
  "bottom-right-corner",
  "bottom-right",
  "bottom-center",
  "bottom-left",
  "bottom-left-corner",
  "left-bottom",
  "left-middle",
  "left-top",
]);

/** A structured explanation for an ignored or recovered mutation. */
export interface SheetOMDiagnostic {
  code: string;
  severity: "warning";
  operation: "setProperty";
  message: string;
  property: string;
  input: string;
  location: null;
}

/** Options for a constructed SheetOM stylesheet. */
export interface CSSStyleSheetOptions {
  baseURL?: string;
  media?: string;
  disabled?: boolean;
  diagnostics?: boolean;
}

/** Options for forgiving parsing of an existing regular stylesheet. */
export interface ParseStyleSheetOptions extends CSSStyleSheetOptions {
  href?: string;
}

type ReportDiagnostic = (diagnostic: SheetOMDiagnostic) => void;

const ignoreDiagnostic: ReportDiagnostic = () => {};
const ruleDiagnostics = new WeakMap<CSSRule, ReportDiagnostic>();
const ruleTree = new RuleTree<CSSRule, CSSStyleSheet>(
  (rule, parentRule, parentStyleSheet) => {
    ruleParentage.set(rule, { parentRule, parentStyleSheet });
  },
  rule => rule.parentStyleSheet,
);
const safeSerializer = new Serializer<CSSRule>(describeRuleSafe);

function parseDeclarationValue(
  declaration: CSSStyleDeclaration,
  name: string,
  observableValue: string,
): ParsedPropertyValue | null {
  const atrule = declaration.parentRule instanceof CSSFontFaceRule
    ? "font-face"
    : null;
  if (atrule) return parseAtruleDescriptorValue(atrule, name, observableValue);
  return parsePropertyValue(name, observableValue);
}

function namedPropertyToCSS(property: string): string {
  if (property === "cssFloat") return "float";

  let cssName = property.replace(/[A-Z]/g, character => `-${character.toLowerCase()}`);
  if (/^(webkit|moz|ms|o)-/.test(cssName)) cssName = `-${cssName}`;
  return cssName;
}

function normalizeSelectorText(value: string): string | null {
  let parsed: csstree.CssNode;
  try {
    parsed = csstree.parse(value, { context: "selectorList" });
  } catch {
    return null;
  }
  if (parsed.type !== "SelectorList" || parsed.children.isEmpty) return null;

  try {
    const generated = csstree.generate(parsed);
    const result = transform({
      filename: "sheetom-selector.css",
      code: encoder.encode(`${generated} { --sheetom-probe: 0; }`),
    });
    const serialized = decoder.decode(result.code);
    const blockIndex = serialized.indexOf(" {");
    if (blockIndex === -1) return null;
    return serialized.slice(0, blockIndex);
  } catch {
    return null;
  }
}

function toUnsignedLong(value: unknown): number {
  const number = Number(value);
  if (!Number.isFinite(number) || number === 0) return 0;

  const integer = Math.trunc(number);
  return ((integer % unsignedLongRange) + unsignedLongRange) % unsignedLongRange;
}

function requireArguments(
  actual: number,
  required: number,
  interfaceName: string,
  operation: string,
): void {
  if (actual >= required) return;
  const noun = required === 1 ? "argument" : "arguments";
  throw new TypeError(
    `Failed to execute '${operation}' on '${interfaceName}': ${required} ${noun} required, but only ${actual} present.`,
  );
}

function lockOwnProperties(target: object, ...properties: string[]): void {
  for (const property of properties) {
    const descriptor = Object.getOwnPropertyDescriptor(target, property);
    if (!descriptor || !("value" in descriptor)) continue;
    Object.defineProperty(target, property, { ...descriptor, writable: false });
  }
}

/** Base class for live stylesheet rules. */
export class CSSRule {
  static readonly STYLE_RULE = 1;
  static readonly CHARSET_RULE = 2;
  static readonly IMPORT_RULE = 3;
  static readonly MEDIA_RULE = 4;
  static readonly FONT_FACE_RULE = 5;
  static readonly PAGE_RULE = 6;
  static readonly KEYFRAMES_RULE = 7;
  static readonly KEYFRAME_RULE = 8;
  static readonly MARGIN_RULE = 9;
  static readonly NAMESPACE_RULE = 10;
  static readonly COUNTER_STYLE_RULE = 11;
  static readonly SUPPORTS_RULE = 12;
  static readonly FONT_FEATURE_VALUES_RULE = 14;

  declare readonly STYLE_RULE: 1;
  declare readonly CHARSET_RULE: 2;
  declare readonly IMPORT_RULE: 3;
  declare readonly MEDIA_RULE: 4;
  declare readonly FONT_FACE_RULE: 5;
  declare readonly PAGE_RULE: 6;
  declare readonly KEYFRAMES_RULE: 7;
  declare readonly KEYFRAME_RULE: 8;
  declare readonly MARGIN_RULE: 9;
  declare readonly NAMESPACE_RULE: 10;
  declare readonly COUNTER_STYLE_RULE: 11;
  declare readonly SUPPORTS_RULE: 12;
  declare readonly FONT_FEATURE_VALUES_RULE: 14;

  readonly #type: number;

  protected constructor(type: number) {
    assertInternalConstructor(new.target.name);
    this.#type = type;
    ruleParentage.set(this, { parentRule: null, parentStyleSheet: null });
  }

  get parentRule(): CSSRule | null {
    return ruleParentage.get(this)?.parentRule ?? null;
  }

  get type(): number {
    return this.#type;
  }

  get parentStyleSheet(): CSSStyleSheet | null {
    return ruleParentage.get(this)?.parentStyleSheet ?? null;
  }

  get cssText(): string {
    return "";
  }

  set cssText(_value: string) {}
}

for (const [name, value] of Object.entries({
  STYLE_RULE: CSSRule.STYLE_RULE,
  CHARSET_RULE: CSSRule.CHARSET_RULE,
  IMPORT_RULE: CSSRule.IMPORT_RULE,
  MEDIA_RULE: CSSRule.MEDIA_RULE,
  FONT_FACE_RULE: CSSRule.FONT_FACE_RULE,
  PAGE_RULE: CSSRule.PAGE_RULE,
  KEYFRAMES_RULE: CSSRule.KEYFRAMES_RULE,
  KEYFRAME_RULE: CSSRule.KEYFRAME_RULE,
  MARGIN_RULE: CSSRule.MARGIN_RULE,
  NAMESPACE_RULE: CSSRule.NAMESPACE_RULE,
  COUNTER_STYLE_RULE: CSSRule.COUNTER_STYLE_RULE,
  SUPPORTS_RULE: CSSRule.SUPPORTS_RULE,
  FONT_FEATURE_VALUES_RULE: CSSRule.FONT_FEATURE_VALUES_RULE,
})) {
  Object.defineProperty(CSSRule.prototype, name, {
    value,
    writable: false,
    enumerable: true,
    configurable: false,
  });
}

/** A stable, live, indexed collection of rules. */
export class CSSRuleList {
  readonly [index: number]: CSSRule | undefined;

  readonly #rules: CSSRule[];

  /** @internal */
  constructor(rules: CSSRule[]) {
    assertInternalConstructor("CSSRuleList");
    this.#rules = rules;

    return new Proxy(this, {
      get(target, property) {
        if (typeof property !== "string" || !arrayIndexPattern.test(property)) {
          const result = Reflect.get(target, property, target);
          return typeof result === "function" ? result.bind(target) : result;
        }

        return target.#rules[Number(property)];
      },
      set(target, property, value) {
        if (typeof property === "string" && arrayIndexPattern.test(property)) return false;
        return Reflect.set(target, property, value, target);
      },
    });
  }

  get length(): number {
    return this.#rules.length;
  }

  item(index: number): CSSRule | null {
    requireArguments(arguments.length, 1, "CSSRuleList", "item");
    return this.#rules[toUnsignedLong(index)] ?? null;
  }
}

function normalizeMediaText(value: string): string | null {
  try {
    const result = transform({
      filename: "sheetom-media.css",
      code: encoder.encode(`@media ${value} { .sheetom-probe { --sheetom: 0; } }`),
    });
    const serialized = decoder.decode(result.code);
    const blockIndex = serialized.indexOf("{");
    if (!serialized.startsWith("@media ") || blockIndex === -1) return null;
    return serialized.slice("@media ".length, blockIndex).trim();
  } catch {
    return null;
  }
}

function normalizeConditionalPrelude(name: "supports" | "container", value: string): string {
  try {
    const result = transform({
      filename: `sheetom-${name}.css`,
      code: encoder.encode(`@${name} ${value} { .sheetom-probe { --sheetom: 0; } }`),
    });
    const serialized = decoder.decode(result.code);
    const blockIndex = serialized.indexOf("{");
    const prefix = `@${name} `;
    if (!serialized.startsWith(prefix) || blockIndex === -1) return value;
    return serialized.slice(prefix.length, blockIndex).trim();
  } catch {
    return value;
  }
}

function splitMediaQueries(value: string): string[] {
  const queries: string[] = [];
  let start = 0;
  let depth = 0;
  for (let index = 0; index < value.length; index += 1) {
    const character = value[index];
    if (character === "(") depth += 1;
    if (character === ")" && depth > 0) depth -= 1;
    if (character !== "," || depth !== 0) continue;
    queries.push(value.slice(start, index).trim());
    start = index + 1;
  }
  queries.push(value.slice(start).trim());
  return queries.filter(query => query !== "");
}

/** A live serialized list of media queries. */
export class MediaList {
  readonly [index: number]: string | undefined;

  #mediaText: string;

  constructor(mediaText = "") {
    assertInternalConstructor("MediaList");
    const input = `${mediaText}`.trim();
    this.#mediaText = input === "" ? "" : normalizeMediaText(input) ?? "not all";

    return new Proxy(this, {
      get(target, property) {
        if (typeof property !== "string" || !arrayIndexPattern.test(property)) {
          const result = Reflect.get(target, property, target);
          return typeof result === "function" ? result.bind(target) : result;
        }
        return target.item(Number(property)) || undefined;
      },
      set(target, property, value) {
        if (typeof property === "string" && arrayIndexPattern.test(property)) return false;
        return Reflect.set(target, property, value, target);
      },
    });
  }

  get mediaText(): string {
    return this.#mediaText;
  }

  set mediaText(value: string) {
    const input = `${value}`.trim();
    if (input === "") {
      this.#mediaText = "";
      return;
    }
    const normalized = normalizeMediaText(input);
    this.#mediaText = normalized ?? "not all";
  }

  get length(): number {
    return splitMediaQueries(this.#mediaText).length;
  }

  item(index: number): string | null {
    requireArguments(arguments.length, 1, "MediaList", "item");
    return splitMediaQueries(this.#mediaText)[toUnsignedLong(index)] ?? null;
  }

  appendMedium(medium: string): void {
    requireArguments(arguments.length, 1, "MediaList", "appendMedium");
    const normalized = normalizeMediaText(`${medium}`);
    if (!normalized) return;
    const existing = splitMediaQueries(this.#mediaText);
    if (existing.includes(normalized)) return;
    this.#mediaText = [...existing, normalized].join(", ");
  }

  deleteMedium(medium: string): void {
    requireArguments(arguments.length, 1, "MediaList", "deleteMedium");
    const normalized = normalizeMediaText(`${medium}`);
    const existing = splitMediaQueries(this.#mediaText);
    const index = normalized ? existing.indexOf(normalized) : -1;
    if (index === -1) {
      throw new DOMException("The medium was not found.", "NotFoundError");
    }
    existing.splice(index, 1);
    this.#mediaText = existing.join(", ");
  }
}

function indentRuleText(value: string): string {
  return value.split("\n").map(line => `  ${line}`).join("\n");
}

/** A rule containing a live nested rule list. */
export class CSSGroupingRule extends CSSRule {
  readonly cssRules: CSSRuleList;

  protected constructor(type: number) {
    super(type);
    const rules = ruleTree.createChildren(this);
    this.cssRules = new CSSRuleList(rules);
    lockOwnProperties(this, "cssRules");
  }

  insertRule(ruleText: string, index = 0): number {
    requireArguments(arguments.length, 1, this.constructor.name, "insertRule");
    const rules = ruleTree.children(this);
    const normalizedIndex = toUnsignedLong(index);
    if (normalizedIndex > rules.length) {
      throw new DOMException("The index is outside the allowed range.", "IndexSizeError");
    }

    const input = `${ruleText}`;
    if (!(this instanceof CSSPageRule) && parsedAtRuleName(input) === "import") {
      throw new DOMException(
        "@import rules cannot be inserted inside a group rule.",
        "HierarchyRequestError",
      );
    }

    const rule = parseStrictRule(
      input,
      ruleDiagnostics.get(this) ?? ignoreDiagnostic,
      false,
      this,
    );
    if (!rule) throw new DOMException("The rule could not be parsed.", "SyntaxError");

    if (
      this instanceof CSSStyleRule &&
      rule instanceof CSSStyleRule &&
      !rule.selectorText.trimStart().startsWith("&")
    ) {
      rule.selectorText = `& ${rule.selectorText}`;
    }

    rules.splice(normalizedIndex, 0, rule);
    attachRuleTree(rule, this, this.parentStyleSheet);
    return normalizedIndex;
  }

  deleteRule(index: number): void {
    requireArguments(arguments.length, 1, this.constructor.name, "deleteRule");
    const rules = ruleTree.children(this);
    const normalizedIndex = toUnsignedLong(index);
    if (normalizedIndex >= rules.length) {
      throw new DOMException("The index is outside the allowed range.", "IndexSizeError");
    }
    const [removed] = rules.splice(normalizedIndex, 1);
    if (removed) attachRuleTree(removed, null, null);
  }

  protected serializeGroup(header: string): string {
    const rules = ruleTree.children(this);
    if (rules.length === 0) return `${header} { }`;
    return `${header} {\n${rules.map(rule => indentRuleText(rule.cssText)).join("\n")}\n}`;
  }
}

/** A grouping rule controlled by a serialized condition. */
export class CSSConditionRule extends CSSGroupingRule {
  readonly #conditionText: string;

  protected constructor(type: number, conditionText: string) {
    super(type);
    this.#conditionText = conditionText;
  }

  get conditionText(): string {
    return this.#conditionText;
  }
}

/** A live `@media` rule. */
export class CSSMediaRule extends CSSConditionRule {
  readonly media: MediaList;

  constructor(conditionText: string) {
    const media = new MediaList(conditionText);
    super(CSSRule.MEDIA_RULE, media.mediaText);
    this.media = media;
    lockOwnProperties(this, "media");
  }

  override get conditionText(): string {
    return this.media.mediaText;
  }

  override get cssText(): string {
    return this.serializeGroup(`@media ${this.media.mediaText}`);
  }

  set cssText(_value: string) {}
}

/** A live `@supports` rule. */
export class CSSSupportsRule extends CSSConditionRule {
  constructor(conditionText: string) {
    super(CSSRule.SUPPORTS_RULE, normalizeConditionalPrelude("supports", conditionText));
  }

  override get cssText(): string {
    return this.serializeGroup(`@supports ${this.conditionText}`);
  }

  set cssText(_value: string) {}
}

/** A live `@container` rule. */
export class CSSContainerRule extends CSSConditionRule {
  readonly containerName: string;
  readonly containerQuery: string;

  constructor(conditionText: string) {
    const normalizedCondition = normalizeConditionalPrelude("container", conditionText);
    super(0, normalizedCondition);
    const openingParenthesis = normalizedCondition.indexOf("(");
    this.containerName = openingParenthesis === -1
      ? ""
      : normalizedCondition.slice(0, openingParenthesis).trim();
    this.containerQuery = openingParenthesis === -1
      ? normalizedCondition
      : normalizedCondition.slice(openingParenthesis).trim();
    lockOwnProperties(this, "containerName", "containerQuery");
  }

  override get cssText(): string {
    return this.serializeGroup(`@container ${this.conditionText}`);
  }

  set cssText(_value: string) {}
}

/** A live block-form `@layer` rule. */
export class CSSLayerBlockRule extends CSSGroupingRule {
  readonly name: string;

  constructor(name: string) {
    super(0);
    this.name = name;
    lockOwnProperties(this, "name");
  }

  override get cssText(): string {
    return this.serializeGroup(`@layer${this.name === "" ? "" : ` ${this.name}`}`);
  }

  set cssText(_value: string) {}
}

/** A live `@scope` rule. */
export class CSSScopeRule extends CSSGroupingRule {
  readonly start: string | null;
  readonly end: string | null;

  constructor(start: string | null, end: string | null) {
    super(0);
    this.start = start;
    this.end = end;
    lockOwnProperties(this, "start", "end");
  }

  override get cssText(): string {
    const start = this.start === null ? "" : ` (${this.start})`;
    const end = this.end === null ? "" : ` to (${this.end})`;
    return this.serializeGroup(`@scope${start}${end}`);
  }

  set cssText(_value: string) {}
}

/** A live `@starting-style` rule. */
export class CSSStartingStyleRule extends CSSGroupingRule {
  constructor() {
    super(0);
  }

  override get cssText(): string {
    return this.serializeGroup("@starting-style");
  }

  set cssText(_value: string) {}
}

/** A non-loading `@import` rule with mutable media. */
export class CSSImportRule extends CSSRule {
  readonly media: MediaList;
  readonly styleSheet: null = null;
  readonly layerName: string | null;
  readonly supportsText: string | null;
  readonly #rawHref: string;

  constructor(
    href: string,
    mediaText = "",
    layerName: string | null = null,
    supportsText: string | null = null,
  ) {
    super(CSSRule.IMPORT_RULE);
    this.#rawHref = href;
    this.media = new MediaList(mediaText);
    this.layerName = layerName;
    this.supportsText = supportsText;
    lockOwnProperties(this, "media", "styleSheet", "layerName", "supportsText");
  }

  get href(): string {
    const baseURL = this.parentStyleSheet?.baseURL;
    if (!baseURL || baseURL === "about:blank") return this.#rawHref;
    try {
      return new URL(this.#rawHref, baseURL).href;
    } catch {
      return this.#rawHref;
    }
  }

  override get cssText(): string {
    const href = this.#rawHref.replace(/["\\]/g, character => `\\${character}`);
    const layer = this.layerName === null
      ? ""
      : this.layerName === ""
        ? " layer"
        : ` layer(${this.layerName})`;
    const supports = this.supportsText === null
      ? ""
      : ` supports(${this.supportsText})`;
    const media = this.media.mediaText === "" ? "" : ` ${this.media.mediaText}`;
    return `@import url("${href}")${layer}${supports}${media};`;
  }

  set cssText(_value: string) {}
}

class CSSGenericRule extends CSSRule {
  readonly #cssText: string;

  constructor(type: number, cssText: string) {
    super(type);
    this.#cssText = cssText;
  }

  override get cssText(): string {
    return this.#cssText;
  }

  set cssText(_value: string) {}
}

class CSSOpaqueRule extends CSSGenericRule {}

/** A live declaration block with indexed, named, and method-based access. */
export class CSSStyleDeclaration {
  readonly [index: number]: string | undefined;

  readonly parentRule: CSSRule;
  readonly #block: DeclarationBlock;

  constructor(parentRule: CSSRule) {
    assertInternalConstructor("CSSStyleDeclaration");
    this.parentRule = parentRule;
    lockOwnProperties(this, "parentRule");
    this.#block = new DeclarationBlock(
      {
        parseValue: (name, value) => parseDeclarationValue(this, name, value),
        shorthandLonghands: getShorthandLonghands,
        expandFourSide: expandStaticFourSide,
        expandShorthand: expandStaticShorthand,
        serializeIdentifier,
        normalizeIndex: toUnsignedLong,
        isFourSideShorthand,
      },
      (code, property, input) => {
        const priority = code === "INVALID_PRIORITY";
        (ruleDiagnostics.get(this.parentRule) ?? ignoreDiagnostic)({
          code,
          severity: "warning",
          operation: "setProperty",
          message: priority
            ? `The mutation was ignored because ${input} is not a valid priority.`
            : `The value was ignored because it is invalid for ${property}.`,
          property,
          input,
          location: null,
        });
      },
    );

    return new Proxy(this, {
      get(target, property) {
        if (typeof property === "string" && arrayIndexPattern.test(property)) {
          return target.item(Number(property)) || undefined;
        }

        if (typeof property === "string" && !Reflect.has(target, property)) {
          return target.getPropertyValue(namedPropertyToCSS(property));
        }

        const result = Reflect.get(target, property, target);
        return typeof result === "function" ? result.bind(target) : result;
      },
      set(target, property, value) {
        if (typeof property === "string" && !Reflect.has(target, property)) {
          Reflect.apply(target.setProperty, target, [namedPropertyToCSS(property), value]);
          return true;
        }

        return Reflect.set(target, property, value, target);
      },
    });
  }

  get cssText(): string {
    return this.#block.serialize(false, "", " ");
  }

  set cssText(value: string) {
    let parsed: csstree.CssNode;
    try {
      parsed = csstree.parse(`${value}`, { context: "declarationList" });
    } catch {
      this.#block.replace(null);
      return;
    }

    if (parsed.type !== "DeclarationList") {
      this.#block.replace(null);
      return;
    }

    const declarations: ParsedDeclaration[] = [];
    for (const child of parsed.children) {
      if (child.type !== "Declaration") continue;
      declarations.push({
        name: child.property,
        value: csstree.generate(child.value),
        important: child.important === true || child.important === "important",
      });
    }
    this.#block.replace(declarations);
  }

  get length(): number {
    return this.#block.length;
  }

  item(index: number): string {
    requireArguments(arguments.length, 1, "CSSStyleDeclaration", "item");
    return this.#block.item(index);
  }

  getPropertyValue(name: string): string {
    requireArguments(arguments.length, 1, "CSSStyleDeclaration", "getPropertyValue");
    return this.#block.getPropertyValue(`${name}`);
  }

  getPropertyPriority(name: string): string {
    requireArguments(arguments.length, 1, "CSSStyleDeclaration", "getPropertyPriority");
    return this.#block.getPropertyPriority(`${name}`);
  }

  setProperty(name: string, value: string | null, priority = ""): void {
    requireArguments(arguments.length, 2, "CSSStyleDeclaration", "setProperty");
    const stringName = `${name}`;
    const stringValue = value === null ? "" : `${value}`;
    const stringPriority = priority === null ? "" : `${priority}`;
    this.#block.setProperty(stringName, stringValue, stringPriority);
  }

  removeProperty(name: string): string {
    requireArguments(arguments.length, 1, "CSSStyleDeclaration", "removeProperty");
    return this.#block.removeProperty(`${name}`);
  }

  /** @internal */
  serializeSafe(indent: string): string {
    return this.#block.serialize(true, indent, "\n");
  }
}

/** A live `@font-face` descriptor rule. */
export class CSSFontFaceRule extends CSSRule {
  readonly style: CSSStyleDeclaration;

  constructor() {
    super(CSSRule.FONT_FACE_RULE);
    this.style = new CSSStyleDeclaration(this);
    lockOwnProperties(this, "style");
  }

  override get cssText(): string {
    return `@font-face {${this.style.cssText === "" ? "" : ` ${this.style.cssText}`} }`;
  }

  set cssText(_value: string) {}
}

/** Declarations ordered after a nested style rule. */
export class CSSNestedDeclarations extends CSSRule {
  readonly #style: CSSStyleDeclaration;

  constructor() {
    super(0);
    this.#style = new CSSStyleDeclaration(this);
  }

  get style(): CSSStyleDeclaration {
    return this.#style;
  }

  set style(cssText: string) {
    this.#style.cssText = `${cssText}`;
  }

  override get cssText(): string {
    return this.style.cssText;
  }

  set cssText(_value: string) {}
}

/** A page-margin rule nested inside `CSSPageRule`. */
export class CSSMarginRule extends CSSRule {
  readonly name: string;
  readonly style: CSSStyleDeclaration;

  constructor(name: string) {
    super(CSSRule.MARGIN_RULE);
    this.name = name;
    this.style = new CSSStyleDeclaration(this);
    lockOwnProperties(this, "name", "style");
  }

  override get cssText(): string {
    return `@${this.name} {${this.style.cssText === "" ? "" : ` ${this.style.cssText}`} }`;
  }

  set cssText(_value: string) {}
}

/** A live `@page` rule with declarations and margin rules. */
export class CSSPageRule extends CSSGroupingRule {
  readonly style: CSSStyleDeclaration;
  #selectorText: string;

  constructor(selectorText: string) {
    super(CSSRule.PAGE_RULE);
    this.#selectorText = selectorText;
    this.style = new CSSStyleDeclaration(this);
    lockOwnProperties(this, "style");
  }

  get selectorText(): string {
    return this.#selectorText;
  }

  set selectorText(value: string) {
    this.#selectorText = `${value}`.trim();
  }

  override get cssText(): string {
    const contents: string[] = [];
    if (this.style.cssText !== "") contents.push(this.style.cssText);
    for (const child of ruleTree.children(this)) contents.push(child.cssText);
    const selector = this.selectorText === "" ? "" : ` ${this.selectorText}`;
    return `@page${selector} {${contents.length === 0 ? "" : ` ${contents.join(" ")}`} }`;
  }

  set cssText(_value: string) {}
}

/** A live `@position-try` declaration rule. */
export class CSSPositionTryRule extends CSSRule {
  readonly name: string;
  readonly style: CSSStyleDeclaration;

  constructor(name: string) {
    super(0);
    this.name = name;
    this.style = new CSSStyleDeclaration(this);
    lockOwnProperties(this, "name", "style");
  }

  override get cssText(): string {
    return `@position-try ${this.name} {${this.style.cssText === "" ? "" : ` ${this.style.cssText}`} }`;
  }

  set cssText(_value: string) {}
}

function normalizeKeyText(value: string): string | null {
  const parts = value.split(",").map(part => part.trim());
  if (parts.length === 0 || parts.some(part => part === "")) return null;
  const normalized: string[] = [];
  for (const part of parts) {
    const lower = part.toLowerCase();
    if (lower === "from") {
      normalized.push("0%");
      continue;
    }
    if (lower === "to") {
      normalized.push("100%");
      continue;
    }
    const match = lower.match(/^([+-]?(?:\d+(?:\.\d*)?|\.\d+))%$/);
    if (!match) return null;
    const percentage = Number(match[1]);
    if (percentage < 0 || percentage > 100) return null;
    normalized.push(`${percentage}%`);
  }
  return normalized.join(", ");
}

/** One mutable keyframe selector and declaration block. */
export class CSSKeyframeRule extends CSSRule {
  readonly style: CSSStyleDeclaration;
  #keyText: string;

  constructor(keyText: string) {
    super(CSSRule.KEYFRAME_RULE);
    this.#keyText = normalizeKeyText(keyText) ?? keyText;
    this.style = new CSSStyleDeclaration(this);
    lockOwnProperties(this, "style");
  }

  get keyText(): string {
    return this.#keyText;
  }

  set keyText(value: string) {
    const normalized = normalizeKeyText(`${value}`);
    if (normalized !== null) this.#keyText = normalized;
  }

  override get cssText(): string {
    return `${this.keyText} {${this.style.cssText === "" ? "" : ` ${this.style.cssText}`} }`;
  }

  set cssText(_value: string) {}
}

/** A mutable `@keyframes` rule. */
export class CSSKeyframesRule extends CSSRule {
  readonly cssRules: CSSRuleList;
  #name: string;

  constructor(name: string) {
    super(CSSRule.KEYFRAMES_RULE);
    this.#name = name;
    const rules = ruleTree.createChildren(this);
    this.cssRules = new CSSRuleList(rules);
    lockOwnProperties(this, "cssRules");
  }

  get name(): string {
    return this.#name;
  }

  set name(value: string) {
    this.#name = `${value}`;
  }

  get length(): number {
    return this.cssRules.length;
  }

  appendRule(ruleText: string): void {
    requireArguments(arguments.length, 1, "CSSKeyframesRule", "appendRule");
    const parsed = parseKeyframeRule(`${ruleText}`, ruleDiagnostics.get(this) ?? ignoreDiagnostic);
    if (!parsed) return;
    const rules = ruleTree.children(this);
    rules.push(parsed);
    attachRuleTree(parsed, this, this.parentStyleSheet);
  }

  deleteRule(select: string): void {
    requireArguments(arguments.length, 1, "CSSKeyframesRule", "deleteRule");
    const normalized = normalizeKeyText(`${select}`);
    if (normalized === null) return;
    const rules = ruleTree.children(this);
    let index = -1;
    for (let candidate = rules.length - 1; candidate >= 0; candidate -= 1) {
      const rule = rules[candidate];
      if (!(rule instanceof CSSKeyframeRule) || rule.keyText !== normalized) continue;
      index = candidate;
      break;
    }
    if (index === -1) return;
    const [removed] = rules.splice(index, 1);
    if (removed) attachRuleTree(removed, null, null);
  }

  findRule(select: string): CSSKeyframeRule | null {
    requireArguments(arguments.length, 1, "CSSKeyframesRule", "findRule");
    const normalized = normalizeKeyText(`${select}`);
    if (normalized === null) return null;
    const rules = ruleTree.children(this);
    for (let index = rules.length - 1; index >= 0; index -= 1) {
      const rule = rules[index];
      if (rule instanceof CSSKeyframeRule && rule.keyText === normalized) return rule;
    }
    return null;
  }

  override get cssText(): string {
    const rules = ruleTree.children(this);
    if (rules.length === 0) return `@keyframes ${this.name} { }`;
    return `@keyframes ${this.name} { \n${rules.map(rule => indentRuleText(rule.cssText)).join("\n")}\n}`;
  }

  set cssText(_value: string) {}
}

const counterDescriptorNames = [
  "system",
  "symbols",
  "additive-symbols",
  "negative",
  "prefix",
  "suffix",
  "range",
  "pad",
  "speak-as",
  "fallback",
] as const;

/** A mutable `@counter-style` descriptor rule. */
export class CSSCounterStyleRule extends CSSRule {
  readonly #descriptors = new Map<string, string>();
  #name: string;

  constructor(name: string) {
    super(CSSRule.COUNTER_STYLE_RULE);
    this.#name = name;
  }

  get name(): string { return this.#name; }
  set name(value: string) { this.#name = `${value}`; }

  get system(): string { return this.#get("system"); }
  set system(value: string) { this.#set("system", value); }
  get symbols(): string { return this.#get("symbols"); }
  set symbols(value: string) { this.#set("symbols", value); }
  get additiveSymbols(): string { return this.#get("additive-symbols"); }
  set additiveSymbols(value: string) { this.#set("additive-symbols", value); }
  get negative(): string { return this.#get("negative"); }
  set negative(value: string) { this.#set("negative", value); }
  get prefix(): string { return this.#get("prefix"); }
  set prefix(value: string) { this.#set("prefix", value); }
  get suffix(): string { return this.#get("suffix"); }
  set suffix(value: string) { this.#set("suffix", value); }
  get range(): string { return this.#get("range"); }
  set range(value: string) { this.#set("range", value); }
  get pad(): string { return this.#get("pad"); }
  set pad(value: string) { this.#set("pad", value); }
  get speakAs(): string { return this.#get("speak-as"); }
  set speakAs(value: string) { this.#set("speak-as", value); }
  get fallback(): string { return this.#get("fallback"); }
  set fallback(value: string) { this.#set("fallback", value); }

  #get(name: string): string {
    return this.#descriptors.get(name) ?? "";
  }

  #set(name: string, value: string): void {
    const text = `${value}`;
    if (text === "") {
      this.#descriptors.delete(name);
      return;
    }
    const parsed = parseAtruleDescriptorValue("counter-style", name, text);
    if (!parsed) return;
    this.#descriptors.set(name, parsed.observableValue);
  }

  /** @internal */
  setParsedDescriptor(name: string, value: string): void {
    if (!(counterDescriptorNames as readonly string[]).includes(name)) return;
    this.#set(name, value);
  }

  override get cssText(): string {
    const declarations: string[] = [];
    for (const name of counterDescriptorNames) {
      const value = this.#descriptors.get(name);
      if (value !== undefined) declarations.push(`${name}: ${value};`);
    }
    return `@counter-style ${this.name} {${declarations.length === 0 ? "" : ` ${declarations.join(" ")}`} }`;
  }

  set cssText(_value: string) {}
}

/** A live map exposed by one font-feature-values category. */
export class CSSFontFeatureValuesMap implements Iterable<[string, number[]]> {
  readonly #values = new Map<string, number[]>();

  constructor() {
    assertInternalConstructor("CSSFontFeatureValuesMap");
  }

  get size(): number {
    return this.#values.size;
  }

  set(featureValueName: string, values: number[]): this {
    const name = `${featureValueName}`;
    if (!/^[-_a-zA-Z][-_a-zA-Z0-9]*$/.test(name)) {
      throw new DOMException("The feature value name is invalid.", "SyntaxError");
    }
    const normalized = Array.from(values, value => Math.trunc(Number(value)));
    if (normalized.some(value => !Number.isFinite(value) || value < 0)) {
      throw new TypeError("Feature values must be non-negative integers.");
    }
    this.#values.set(name, normalized);
    return this;
  }

  clear(): void { this.#values.clear(); }
  delete(name: string): boolean { return this.#values.delete(`${name}`); }
  get(name: string): number[] | undefined {
    const value = this.#values.get(`${name}`);
    return value ? [...value] : undefined;
  }
  has(name: string): boolean { return this.#values.has(`${name}`); }
  entries(): MapIterator<[string, number[]]> {
    return new Map(
      [...this.#values].map(([name, values]) => [name, [...values]]),
    ).entries();
  }
  keys(): MapIterator<string> { return this.#values.keys(); }
  values(): MapIterator<number[]> {
    return new Map(
      [...this.#values].map(([name, values]) => [name, [...values]]),
    ).values();
  }
  forEach(
    callback: (value: number[], key: string, map: CSSFontFeatureValuesMap) => void,
    thisArg?: unknown,
  ): void {
    for (const [name, values] of this.#values) {
      callback.call(thisArg, [...values], name, this);
    }
  }
  [Symbol.iterator](): MapIterator<[string, number[]]> { return this.entries(); }
}

const fontFeatureMapNames = [
  "annotation",
  "ornaments",
  "stylistic",
  "swash",
  "character-variant",
  "styleset",
] as const;

/** A mutable `@font-feature-values` rule. */
export class CSSFontFeatureValuesRule extends CSSRule {
  readonly annotation = new CSSFontFeatureValuesMap();
  readonly ornaments = new CSSFontFeatureValuesMap();
  readonly stylistic = new CSSFontFeatureValuesMap();
  readonly swash = new CSSFontFeatureValuesMap();
  readonly characterVariant = new CSSFontFeatureValuesMap();
  readonly styleset = new CSSFontFeatureValuesMap();
  #fontFamily: string;

  constructor(fontFamily: string) {
    super(CSSRule.FONT_FEATURE_VALUES_RULE);
    this.#fontFamily = fontFamily;
    lockOwnProperties(
      this,
      "annotation",
      "ornaments",
      "stylistic",
      "swash",
      "characterVariant",
      "styleset",
    );
  }

  get fontFamily(): string { return this.#fontFamily; }
  set fontFamily(value: string) { this.#fontFamily = `${value}`.trim(); }

  /** @internal */
  featureMap(name: string): CSSFontFeatureValuesMap | null {
    switch (name) {
      case "annotation": return this.annotation;
      case "ornaments": return this.ornaments;
      case "stylistic": return this.stylistic;
      case "swash": return this.swash;
      case "character-variant": return this.characterVariant;
      case "styleset": return this.styleset;
      default: return null;
    }
  }

  override get cssText(): string {
    const blocks: string[] = [];
    for (const name of fontFeatureMapNames) {
      const map = this.featureMap(name);
      if (!map || map.size === 0) continue;
      const declarations = [...map]
        .map(([key, values]) => `${key}: ${values.join(" ")};`)
        .join(" ");
      blocks.push(`@${name} { ${declarations} }`);
    }
    return `@font-feature-values ${this.fontFamily} {${blocks.length === 0 ? "" : ` ${blocks.join(" ")}`} }`;
  }

  set cssText(_value: string) {}
}

/** A live style rule with declarations and nested rules. */
export class CSSStyleRule extends CSSGroupingRule {
  readonly style: CSSStyleDeclaration;
  #selectorText: string;

  constructor(selectorText: string) {
    super(CSSRule.STYLE_RULE);
    this.#selectorText = normalizeSelectorText(`${selectorText}`) ?? `${selectorText}`;
    this.style = new CSSStyleDeclaration(this);
    lockOwnProperties(this, "style");
  }

  get selectorText(): string {
    return this.#selectorText;
  }

  set selectorText(value: string) {
    const normalized = normalizeSelectorText(`${value}`);
    if (normalized === null) return;
    this.#selectorText = normalized;
  }

  override get cssText(): string {
    const declarations = this.style.cssText;
    const children = ruleTree.children(this);
    if (children.length === 0) {
      return declarations === ""
        ? `${this.selectorText} { }`
        : `${this.selectorText} { ${declarations} }`;
    }

    const contents: string[] = [];
    if (declarations !== "") contents.push(indentRuleText(declarations));
    for (const child of children) contents.push(indentRuleText(child.cssText));
    return `${this.selectorText} {\n${contents.join("\n")}\n}`;
  }

  set cssText(_value: string) {}

}

function createStyleRule(
  node: csstree.Rule,
  reportDiagnostic: ReportDiagnostic,
): CSSStyleRule | null {
  const selectorText = normalizeSelectorText(csstree.generate(node.prelude));
  if (selectorText === null) return null;
  const rule = new CSSStyleRule(selectorText);
  ruleDiagnostics.set(rule, reportDiagnostic);
  let declarationNodes: csstree.Declaration[] = [];
  const childRules: CSSRule[] = [];
  let foundNestedRule = false;

  const flushDeclarations = (): void => {
    if (declarationNodes.length === 0) return;
    const declarationText = declarationNodes
      .map(child => csstree.generate(child))
      .join(";");
    if (!foundNestedRule) {
      rule.style.cssText = declarationText;
      declarationNodes = [];
      return;
    }

    const nestedDeclarations = new CSSNestedDeclarations();
    ruleDiagnostics.set(nestedDeclarations, reportDiagnostic);
    nestedDeclarations.style.cssText = declarationText;
    childRules.push(nestedDeclarations);
    declarationNodes = [];
  };

  for (const child of node.block.children) {
    if (child.type === "Declaration") {
      declarationNodes.push(child);
      continue;
    }
    if (child.type !== "Rule" && child.type !== "Atrule") continue;
    flushDeclarations();
    foundNestedRule = true;
    const parsedChild = createRuleFromNode(child, reportDiagnostic, true);
    if (parsedChild) childRules.push(parsedChild);
  }
  flushDeclarations();
  replaceGroupingRules(rule, childRules);
  return rule;
}

function declarationTextFromBlock(block: csstree.Block): string {
  const declarations: string[] = [];
  for (const child of block.children) {
    if (child.type === "Declaration") declarations.push(csstree.generate(child));
  }
  return declarations.join(";");
}

function generateDescriptorInput(value: csstree.Value | csstree.Raw): string {
  if (value.type === "Raw") return value.value;
  return value.children.toArray().map(child => csstree.generate(child)).join(" ");
}

function createKeyframeRuleFromNode(
  node: csstree.Rule,
  reportDiagnostic: ReportDiagnostic,
): CSSKeyframeRule {
  return constructInternally(() => {
    const rule = new CSSKeyframeRule(csstree.generate(node.prelude));
    ruleDiagnostics.set(rule, reportDiagnostic);
    rule.style.cssText = declarationTextFromBlock(node.block);
    return rule;
  });
}

function parseKeyframeRule(
  ruleText: string,
  reportDiagnostic: ReportDiagnostic,
): CSSKeyframeRule | null {
  try {
    const parsed = csstree.parse(`@keyframes sheetom { ${ruleText} }`);
    if (parsed.type !== "StyleSheet") return null;
    const keyframes = parsed.children.first;
    if (keyframes?.type !== "Atrule" || !keyframes.block) return null;
    const node = keyframes.block.children.first;
    if (node?.type !== "Rule" || keyframes.block.children.size !== 1) return null;
    return createKeyframeRuleFromNode(node, reportDiagnostic);
  } catch {
    return null;
  }
}

function replaceKeyframeRules(rule: CSSKeyframesRule, rules: CSSKeyframeRule[]): void {
  ruleTree.replace(rule, rules);
}

function replaceGroupingRules(group: CSSGroupingRule, rules: CSSRule[]): void {
  ruleTree.replace(group, rules);
}

function attachRuleTree(
  rule: CSSRule,
  parentRule: CSSRule | null,
  parentStyleSheet: CSSStyleSheet | null,
): void {
  ruleTree.attach(rule, parentRule, parentStyleSheet);
}

function serializeRuleSafe(rule: CSSRule): string {
  return safeSerializer.serialize(rule);
}

function describeRuleSafe(rule: CSSRule): RuleSerializationPlan<CSSRule> {
  if (rule instanceof CSSStyleRule) {
    return {
      kind: "block",
      header: rule.selectorText,
      declarations: rule.style.serializeSafe("  "),
      children: ruleTree.children(rule),
    };
  }
  if (rule instanceof CSSNestedDeclarations) {
    return { kind: "declarations", declarations: rule.style.serializeSafe("") };
  }
  if (
    rule instanceof CSSFontFaceRule ||
    rule instanceof CSSMarginRule ||
    rule instanceof CSSPositionTryRule ||
    rule instanceof CSSKeyframeRule
  ) {
    const blockIndex = rule.cssText.indexOf("{");
    const header = rule.cssText.slice(0, blockIndex).trimEnd();
    return {
      kind: "block",
      header,
      declarations: rule.style.serializeSafe("  "),
    };
  }
  if (rule instanceof CSSKeyframesRule) {
    return {
      kind: "block",
      header: `@keyframes ${rule.name}`,
      children: ruleTree.children(rule),
    };
  }
  if (rule instanceof CSSPageRule) {
    const selector = rule.selectorText === "" ? "" : ` ${rule.selectorText}`;
    return {
      kind: "block",
      header: `@page${selector}`,
      declarations: rule.style.serializeSafe("  "),
      children: ruleTree.children(rule),
    };
  }
  if (!(rule instanceof CSSGroupingRule)) {
    return { kind: "raw", cssText: rule.cssText };
  }

  const blockIndex = rule.cssText.indexOf("{");
  const header = blockIndex === -1 ? rule.cssText : rule.cssText.slice(0, blockIndex).trimEnd();
  return { kind: "block", header, children: ruleTree.children(rule) };
}

function parseScopePrelude(prelude: string): [string | null, string | null] {
  const text = prelude.trim();
  if (text === "") return [null, null];

  const consumeGroup = (value: string): [string | null, string] => {
    if (!value.startsWith("(")) return [null, value];
    let depth = 0;
    for (let index = 0; index < value.length; index += 1) {
      const character = value[index];
      if (character === "(") depth += 1;
      if (character !== ")") continue;
      depth -= 1;
      if (depth !== 0) continue;
      return [value.slice(1, index).trim(), value.slice(index + 1).trim()];
    }
    return [null, value];
  };

  const [start, remainder] = consumeGroup(text);
  if (!remainder.startsWith("to")) return [start, null];
  const [end] = consumeGroup(remainder.slice(2).trim());
  return [start, end];
}

function parseImportPrelude(
  prelude: string,
): { href: string; media: string; layer: string | null; supports: string | null } | null {
  const hrefMatch = prelude.match(
    /^(?:url\(\s*(?:"([^"]*)"|'([^']*)'|([^)]*))\s*\)|"([^"]*)"|'([^']*)')\s*(.*)$/,
  );
  if (!hrefMatch) return null;
  const href = hrefMatch[1] ?? hrefMatch[2] ?? hrefMatch[3]?.trim() ?? hrefMatch[4] ?? hrefMatch[5];
  if (href === undefined) return null;
  let remainder = hrefMatch[6]?.trim() ?? "";
  let layer: string | null = null;
  let supports: string | null = null;

  const layerMatch = remainder.match(/^layer(?:\(([^)]*)\))?(?:\s+|$)(.*)$/);
  if (layerMatch) {
    layer = layerMatch[1]?.trim() ?? "";
    remainder = layerMatch[2]?.trim() ?? "";
  }

  if (remainder.startsWith("supports(")) {
    let depth = 0;
    let end = -1;
    for (let index = "supports".length; index < remainder.length; index += 1) {
      const character = remainder[index];
      if (character === "(") depth += 1;
      if (character !== ")") continue;
      depth -= 1;
      if (depth !== 0) continue;
      end = index;
      break;
    }
    if (end !== -1) {
      supports = remainder.slice("supports(".length, end).trim();
      remainder = remainder.slice(end + 1).trim();
    }
  }

  return { href, media: remainder, layer, supports };
}

function createRuleFromNode(
  node: csstree.Rule | csstree.Atrule,
  reportDiagnostic: ReportDiagnostic,
  preserveImports: boolean,
): CSSRule | null {
  return constructInternally(() =>
    createRuleFromNodeInternal(node, reportDiagnostic, preserveImports),
  );
}

function createRuleFromNodeInternal(
  node: csstree.Rule | csstree.Atrule,
  reportDiagnostic: ReportDiagnostic,
  preserveImports: boolean,
): CSSRule | null {
  if (node.type === "Rule") return createStyleRule(node, reportDiagnostic);

  const name = node.name.toLowerCase();
  if (name === "import" && !preserveImports) return null;
  const prelude = node.prelude ? csstree.generate(node.prelude) : "";
  let rule: CSSRule;

  switch (name) {
    case "import": {
      const parsedImport = parseImportPrelude(prelude);
      if (!parsedImport) return null;
      rule = new CSSImportRule(
        parsedImport.href,
        parsedImport.media,
        parsedImport.layer,
        parsedImport.supports,
      );
      break;
    }
    case "font-face": {
      const fontFace = new CSSFontFaceRule();
      if (node.block) fontFace.style.cssText = declarationTextFromBlock(node.block);
      rule = fontFace;
      break;
    }
    case "page": {
      const page = new CSSPageRule(prelude);
      if (node.block) {
        page.style.cssText = declarationTextFromBlock(node.block);
        const margins: CSSMarginRule[] = [];
        for (const child of node.block.children) {
          if (child.type !== "Atrule" || !child.block) continue;
          const margin = new CSSMarginRule(child.name.toLowerCase());
          margin.style.cssText = declarationTextFromBlock(child.block);
          ruleDiagnostics.set(margin, reportDiagnostic);
          margins.push(margin);
        }
        replaceGroupingRules(page, margins);
      }
      rule = page;
      break;
    }
    case "position-try": {
      if (!prelude.startsWith("--")) return null;
      const positionTry = new CSSPositionTryRule(prelude);
      if (node.block) positionTry.style.cssText = declarationTextFromBlock(node.block);
      rule = positionTry;
      break;
    }
    case "keyframes":
    case "-webkit-keyframes": {
      if (prelude === "") return null;
      const keyframes = new CSSKeyframesRule(prelude);
      if (node.block) {
        const frames: CSSKeyframeRule[] = [];
        for (const child of node.block.children) {
          if (child.type === "Rule") {
            frames.push(createKeyframeRuleFromNode(child, reportDiagnostic));
          }
        }
        replaceKeyframeRules(keyframes, frames);
      }
      rule = keyframes;
      break;
    }
    case "counter-style": {
      if (prelude === "") return null;
      const counterStyle = new CSSCounterStyleRule(prelude);
      if (node.block) {
        for (const child of node.block.children) {
          if (child.type !== "Declaration") continue;
          counterStyle.setParsedDescriptor(
            child.property.toLowerCase(),
            generateDescriptorInput(child.value),
          );
        }
      }
      rule = counterStyle;
      break;
    }
    case "font-feature-values": {
      if (prelude === "") return null;
      const featureValues = new CSSFontFeatureValuesRule(prelude);
      if (node.block) {
        for (const child of node.block.children) {
          if (child.type !== "Atrule" || !child.block) continue;
          const map = featureValues.featureMap(child.name.toLowerCase());
          if (!map) continue;
          for (const declaration of child.block.children) {
            if (declaration.type !== "Declaration") continue;
            const values = csstree.generate(declaration.value)
              .trim()
              .split(/\s+/)
              .map(value => Number(value));
            if (values.some(value => !Number.isFinite(value))) continue;
            map.set(declaration.property, values);
          }
        }
      }
      rule = featureValues;
      break;
    }
    case "media":
      if (prelude === "") return null;
      rule = new CSSMediaRule(prelude);
      break;
    case "supports":
      if (prelude === "") return null;
      rule = new CSSSupportsRule(prelude);
      break;
    case "container":
      if (prelude === "") return null;
      rule = new CSSContainerRule(prelude);
      break;
    case "layer":
      if (!node.block) return new CSSGenericRule(0, csstree.generate(node));
      rule = new CSSLayerBlockRule(prelude);
      break;
    case "scope": {
      const [start, end] = parseScopePrelude(prelude);
      rule = new CSSScopeRule(start, end);
      break;
    }
    case "starting-style":
      rule = new CSSStartingStyleRule();
      break;
    default:
      return new CSSGenericRule(genericRuleType(name), csstree.generate(node));
  }

  ruleDiagnostics.set(rule, reportDiagnostic);
  if (
    !(rule instanceof CSSGroupingRule) ||
    rule instanceof CSSPageRule ||
    !node.block
  ) return rule;

  const children: CSSRule[] = [];
  for (const child of node.block.children) {
    if (child.type !== "Rule" && child.type !== "Atrule") continue;
    const parsedChild = createRuleFromNode(child, reportDiagnostic, true);
    if (parsedChild) children.push(parsedChild);
  }
  replaceGroupingRules(rule, children);
  return rule;
}

function parseStrictRule(
  ruleText: string,
  reportDiagnostic: ReportDiagnostic,
  preserveImports = false,
  parentRule: CSSGroupingRule | null = null,
): CSSRule | null {
  let parsed: csstree.CssNode;
  try {
    parsed = csstree.parse(ruleText);
  } catch {
    return null;
  }

  if (parsed.type !== "StyleSheet" || parsed.children.size !== 1) return null;
  const node = parsed.children.first;
  if (!node) return null;
  if (node.type !== "Rule" && node.type !== "Atrule") return null;
  if (parentRule instanceof CSSPageRule) {
    if (
      node.type !== "Atrule" ||
      !node.block ||
      !pageMarginRuleNames.has(node.name.toLowerCase())
    ) return null;
    return constructInternally(() => {
      const margin = new CSSMarginRule(node.name.toLowerCase());
      margin.style.cssText = declarationTextFromBlock(node.block as csstree.Block);
      ruleDiagnostics.set(margin, reportDiagnostic);
      return margin;
    });
  }
  if (node.type === "Atrule" && node.name.toLowerCase() === "import" && !preserveImports) {
    return null;
  }
  return createRuleFromNode(node, reportDiagnostic, preserveImports);
}

function parsedAtRuleName(ruleText: string): string | null {
  try {
    const parsed = csstree.parse(ruleText);
    if (parsed.type !== "StyleSheet" || parsed.children.size !== 1) return null;
    const node = parsed.children.first;
    return node?.type === "Atrule" ? node.name.toLowerCase() : null;
  } catch {
    return null;
  }
}

function genericRuleType(name: string): number {
  switch (name.toLowerCase()) {
    case "import":
      return CSSRule.IMPORT_RULE;
    case "media":
      return CSSRule.MEDIA_RULE;
    case "font-face":
      return CSSRule.FONT_FACE_RULE;
    case "page":
      return CSSRule.PAGE_RULE;
    case "keyframes":
    case "-webkit-keyframes":
      return CSSRule.KEYFRAMES_RULE;
    case "namespace":
      return CSSRule.NAMESPACE_RULE;
    case "supports":
      return CSSRule.SUPPORTS_RULE;
    default:
      return 0;
  }
}

function parseStyleSheetRules(
  cssText: string,
  reportDiagnostic: ReportDiagnostic,
  preserveImports: boolean,
): CSSRule[] {
  const rules: CSSRule[] = [];
  for (const rawRule of scanTopLevelRules(cssText)) {
    const rule = parseStrictRule(rawRule, reportDiagnostic, preserveImports);
    if (rule) {
      rules.push(rule);
      continue;
    }
    if (!preserveImports) continue;
    rules.push(constructInternally(() => new CSSOpaqueRule(0, rawRule)));
  }

  return rules;
}

/** A mutable, browser-shaped authoring stylesheet. */
export class CSSStyleSheet {
  readonly cssRules: CSSRuleList;
  readonly media: MediaList;
  readonly ownerNode: null = null;
  readonly parentStyleSheet: null = null;
  readonly ownerRule: null = null;
  readonly title: null = null;
  readonly type = "text/css";
  disabled: boolean;

  readonly #rules: CSSRule[] = [];
  readonly #diagnostics: SheetOMDiagnostic[] | null;
  readonly #constructedBaseURL: string;

  constructor(options: CSSStyleSheetOptions | null = {}) {
    const normalizedOptions = options ?? {};
    this.#diagnostics = Boolean(normalizedOptions.diagnostics) ? [] : null;
    this.#constructedBaseURL = normalizedOptions.baseURL === undefined
      ? "about:blank"
      : `${normalizedOptions.baseURL}`;
    const media = normalizedOptions.media === undefined
      ? ""
      : `${normalizedOptions.media}`;
    this.media = constructInternally(() => new MediaList(media));
    this.disabled = Boolean(normalizedOptions.disabled);
    this.cssRules = constructInternally(() => new CSSRuleList(this.#rules));
    lockOwnProperties(
      this,
      "cssRules",
      "media",
      "ownerNode",
      "parentStyleSheet",
      "ownerRule",
      "title",
      "type",
    );
  }

  get href(): string | null {
    return regularSheetMetadata.get(this)?.href ?? null;
  }

  get baseURL(): string {
    return regularSheetMetadata.get(this)?.baseURL ?? this.#constructedBaseURL;
  }

  readonly #reportDiagnostic: ReportDiagnostic = diagnostic => {
    this.#diagnostics?.push(diagnostic);
  };

  insertRule(ruleText: string, index = 0): number {
    requireArguments(arguments.length, 1, "CSSStyleSheet", "insertRule");
    const normalizedIndex = toUnsignedLong(index);
    if (normalizedIndex > this.#rules.length) {
      throw new DOMException("The index is outside the allowed range.", "IndexSizeError");
    }

    const regular = regularSheetMetadata.has(this);
    const rule = parseStrictRule(`${ruleText}`, this.#reportDiagnostic, regular);
    if (!rule) throw new DOMException("The rule could not be parsed.", "SyntaxError");

    const precedingRules = this.#rules.slice(0, normalizedIndex);
    const followingRules = this.#rules.slice(normalizedIndex);
    const invalidImportOrder = rule instanceof CSSImportRule
      ? precedingRules.some(candidate => !(candidate instanceof CSSImportRule))
      : followingRules.some(candidate => candidate instanceof CSSImportRule);
    if (invalidImportOrder) {
      throw new DOMException("The rule violates stylesheet ordering.", "HierarchyRequestError");
    }

    attachRuleTree(rule, null, this);
    this.#rules.splice(normalizedIndex, 0, rule);
    return normalizedIndex;
  }

  deleteRule(index: number): void {
    requireArguments(arguments.length, 1, "CSSStyleSheet", "deleteRule");
    const normalizedIndex = toUnsignedLong(index);
    if (normalizedIndex >= this.#rules.length) {
      throw new DOMException("The index is outside the allowed range.", "IndexSizeError");
    }

    const [removed] = this.#rules.splice(normalizedIndex, 1);
    if (!removed) return;
    attachRuleTree(removed, null, null);
  }

  replaceSync(cssText: string): void {
    requireArguments(arguments.length, 1, "CSSStyleSheet", "replaceSync");
    const replacement = parseStyleSheetRules(
      `${cssText}`,
      this.#reportDiagnostic,
      regularSheetMetadata.has(this),
    );

    for (const rule of this.#rules) {
      attachRuleTree(rule, null, null);
    }
    for (const rule of replacement) attachRuleTree(rule, null, this);

    this.#rules.splice(0, this.#rules.length, ...replacement);
  }

  async replace(cssText: string): Promise<CSSStyleSheet> {
    requireArguments(arguments.length, 1, "CSSStyleSheet", "replace");
    this.replaceSync(cssText);
    return this;
  }

  /** Serialize current state as reparsable CSS without mutating live objects. */
  serialize(): string {
    return this.#rules.map(serializeRuleSafe).join("");
  }

  /** Drain opt-in mutation diagnostics without affecting CSSOM behavior. */
  takeDiagnostics(): SheetOMDiagnostic[] {
    if (!this.#diagnostics) return [];
    return this.#diagnostics.splice(0);
  }
}

/** Forgivingly parse an existing regular stylesheet without loading imports. */
export function parseStyleSheet(
  cssText: string,
  options: ParseStyleSheetOptions = {},
): CSSStyleSheet {
  const sheet = new CSSStyleSheet(options);
  const href = options.href ?? null;
  regularSheetMetadata.set(sheet, {
    href,
    baseURL: options.baseURL ?? href ?? "about:blank",
  });
  sheet.replaceSync(`${cssText}`);
  return sheet;
}
