import type { SheetOMDiagnosticCode } from "./diagnostics.js";
import { NativeDeclarationBlock } from "./internal/native-declaration-block.js";
import { scanTopLevelRules } from "./internal/css-rule-scanner.js";
import {
  parseNativeRule,
  parseNativeRuleWithErrorRecovery,
  type NativeRuleDescription,
} from "./internal/native-rule-parser.js";
import {
  normalizeNativeMedia,
  normalizeNativeSelector,
  parseNativeContainerPrelude,
  parseNativeScopePrelude,
} from "./internal/native-rule-syntax.js";
import { RuleTree } from "./internal/rule-tree.js";
import {
  parseNativeCounterStyleDescriptor,
  parseNativeCounterStyleDescriptors,
  parseNativeCounterStyleName,
  serializeNativeIdentifier,
  serializeNativeFontFamily,
} from "./internal/native-counter-style.js";
import {
  Serializer,
  type RuleSerializationPlan,
} from "./internal/serializer.js";
import {
  assertInternalConstructor,
  constructInternally,
} from "./internal/webidl-construction.js";
import {
  defaultResourceBudget,
  normalizeResourceBudget,
  type NativeResourceBudget,
  type SheetOMResourceBudget,
} from "./internal/resource-budget.js";

export type { SheetOMDiagnosticCode } from "./diagnostics.js";
export type { SheetOMResourceBudget } from "./internal/resource-budget.js";

const arrayIndexPattern = /^(0|[1-9]\d*)$/;
const regularSheetMetadata = new WeakMap<
  object,
  { href: string | null; baseURL: string }
>();
const ruleParentage = new WeakMap<
  object,
  { parentRule: CSSRule | null; parentStyleSheet: CSSStyleSheet | null }
>();
const ruleResourceBudgets = new WeakMap<object, NativeResourceBudget>();
const sheetResourceBudgets = new WeakMap<object, NativeResourceBudget>();
const sheetRuleArrays = new WeakMap<object, CSSRule[]>();
const functionRuleHeaders = new WeakMap<object, string>();
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
  code: SheetOMDiagnosticCode;
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
  resourceBudget?: SheetOMResourceBudget | null;
}

/** Options for forgiving parsing of an existing regular stylesheet. */
export interface ParseStyleSheetOptions extends CSSStyleSheetOptions {
  href?: string;
}

/** One parsed parameter exposed by `CSSFunctionRule.getParameters()`. */
export interface FunctionParameter {
  name: string;
  type: string;
  defaultValue?: string;
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
let activeConstructionResourceBudget = defaultResourceBudget;

function constructWithResourceBudget<T>(
  factory: () => T,
  resourceBudget: NativeResourceBudget,
): T {
  const previous = activeConstructionResourceBudget;
  activeConstructionResourceBudget = resourceBudget;
  try {
    return constructInternally(factory);
  } finally {
    activeConstructionResourceBudget = previous;
  }
}

function currentConstructionResourceBudget(): NativeResourceBudget {
  return activeConstructionResourceBudget;
}

function namedPropertyToCSS(property: string): string {
  if (property === "cssFloat") return "float";

  let cssName = property.replace(/[A-Z]/g, character => `-${character.toLowerCase()}`);
  if (/^(webkit|moz|ms|o)-/.test(cssName)) cssName = `-${cssName}`;
  return cssName;
}

function normalizeSelectorText(
  value: string,
  resourceBudget: NativeResourceBudget = defaultResourceBudget,
): string | null {
  return normalizeNativeSelector(value, resourceBudget);
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

function proxyMember(target: object, property: PropertyKey, value: unknown): unknown {
  if (property === "constructor" || typeof value !== "function") return value;
  return value.bind(target);
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
    ruleResourceBudgets.set(this, currentConstructionResourceBudget());
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
          return proxyMember(target, property, result);
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

function normalizeMediaText(
  value: string,
  resourceBudget: NativeResourceBudget = defaultResourceBudget,
): string | null {
  return normalizeNativeMedia(value, resourceBudget);
}

interface ContainerPrelude {
  conditionText: string;
  name: string;
  query: string;
}

function parseContainerPrelude(
  value: string,
  resourceBudget: NativeResourceBudget = defaultResourceBudget,
): ContainerPrelude | null {
  return parseNativeContainerPrelude(value, resourceBudget);
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
  readonly #resourceBudget: NativeResourceBudget;

  constructor(mediaText = "") {
    assertInternalConstructor("MediaList");
    this.#resourceBudget = currentConstructionResourceBudget();
    const input = `${mediaText}`.trim();
    this.#mediaText = input === ""
      ? ""
      : normalizeMediaText(input, this.#resourceBudget) ?? "not all";

    return new Proxy(this, {
      get(target, property) {
        if (typeof property !== "string" || !arrayIndexPattern.test(property)) {
          const result = Reflect.get(target, property, target);
          return proxyMember(target, property, result);
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
    const normalized = normalizeMediaText(input, this.#resourceBudget);
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
    const normalized = normalizeMediaText(`${medium}`, this.#resourceBudget);
    if (!normalized) return;
    const existing = splitMediaQueries(this.#mediaText);
    if (existing.includes(normalized)) return;
    this.#mediaText = [...existing, normalized].join(", ");
  }

  deleteMedium(medium: string): void {
    requireArguments(arguments.length, 1, "MediaList", "deleteMedium");
    const normalized = normalizeMediaText(`${medium}`, this.#resourceBudget);
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

function ruleChildren(rule: CSSRule): CSSRule[] {
  if (rule instanceof CSSGroupingRule || rule instanceof CSSKeyframesRule) {
    return ruleTree.children(rule);
  }
  return [];
}

function ruleForestSize(roots: readonly CSSRule[]): number {
  let count = 0;
  const pending = [...roots];
  while (pending.length > 0) {
    const rule = pending.pop();
    if (!rule) continue;
    count += 1;
    for (const child of ruleChildren(rule)) pending.push(child);
  }
  return count;
}

function assertRuleForestBudget(
  roots: readonly CSSRule[],
  resourceBudget: NativeResourceBudget,
): void {
  const count = ruleForestSize(roots);
  assertRuleCountBudget(count, resourceBudget);
}

function assertRuleCountBudget(
  count: number,
  resourceBudget: NativeResourceBudget,
): void {
  if (count <= resourceBudget.maxRuleCount) return;
  throw new RangeError(
    `SHEETOM_RULE_LIMIT: stylesheet has ${count} rules; the limit is ${resourceBudget.maxRuleCount} rules`,
  );
}

function assertRuleInsertionBudget(
  owner: CSSRule | CSSStyleSheet,
  inserted: CSSRule,
  resourceBudget: NativeResourceBudget,
): void {
  if (owner instanceof CSSStyleSheet) {
    const roots = sheetRuleArrays.get(owner) ?? [];
    assertRuleCountBudget(
      ruleForestSize(roots) + ruleForestSize([inserted]),
      resourceBudget,
    );
    return;
  }
  const sheet = owner.parentStyleSheet;
  if (sheet) {
    const roots = sheetRuleArrays.get(sheet) ?? [];
    assertRuleCountBudget(
      ruleForestSize(roots) + ruleForestSize([inserted]),
      resourceBudget,
    );
    return;
  }
  let root = owner;
  while (root.parentRule) root = root.parentRule;
  assertRuleCountBudget(
    ruleForestSize([root]) + ruleForestSize([inserted]),
    resourceBudget,
  );
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
    const resourceBudget = ruleResourceBudgets.get(this) ?? defaultResourceBudget;
    if (
      !(this instanceof CSSPageRule)
      && parseNativeRule(input, resourceBudget)?.kind === "import"
    ) {
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
      resourceBudget,
    );
    if (!rule) throw new DOMException("The rule could not be parsed.", "SyntaxError");

    if (
      this instanceof CSSStyleRule &&
      rule instanceof CSSStyleRule &&
      !rule.selectorText.trimStart().startsWith("&")
    ) {
      rule.selectorText = `& ${rule.selectorText}`;
    }

    assertRuleInsertionBudget(this, rule, resourceBudget);
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
    if (rules.length === 0) return `${header} {\n}`;
    return `${header} {\n${rules.map(rule => indentRuleText(rule.cssText)).join("\n")}\n}`;
  }
}

/** A grouping rule controlled by a serialized condition. */
export class CSSConditionRule extends CSSGroupingRule {
  readonly #conditionText: string;

  protected constructor(
    type: number,
    conditionText: string,
  ) {
    super(type);
    this.#conditionText = conditionText;
  }

  get conditionText(): string {
    return this.#conditionText;
  }
}

/** A live custom `@function` rule. */
export class CSSFunctionRule extends CSSGroupingRule {
  readonly name: string;
  readonly returnType: string;
  readonly #parameters: readonly FunctionParameter[];

  constructor(
    name: string,
    parameters: readonly FunctionParameter[],
    returnType: string,
  ) {
    super(0);
    this.name = name;
    this.returnType = returnType;
    this.#parameters = parameters.map(parameter => ({ ...parameter }));
    functionRuleHeaders.set(
      this,
      serializeFunctionHeader(this.name, this.#parameters, this.returnType),
    );
    lockOwnProperties(this, "name", "returnType");
  }

  getParameters(): FunctionParameter[] {
    return this.#parameters.map(parameter => ({ ...parameter }));
  }

  override get cssText(): string {
    const header = functionRuleHeaders.get(this) ?? "@function";
    const children = ruleTree.children(this);
    if (children.length === 0) return `${header} { }`;
    return `${header} { ${children.map(child => child.cssText).join(" ")} }`;
  }

  set cssText(_value: string) {}
}

function serializeFunctionHeader(
  name: string,
  parameters: readonly FunctionParameter[],
  returnType: string,
): string {
  const serializedParameters = parameters.map(parameter => {
    const type = serializeFunctionType(parameter.type);
    const defaultValue = Object.hasOwn(parameter, "defaultValue")
      ? `: ${parameter.defaultValue}`
      : "";
    return `${serializeNativeIdentifier(parameter.name)}${type}${defaultValue}`;
  }).join(", ");
  const serializedReturnType = returnType === "*"
    ? ""
    : ` returns${serializeFunctionType(returnType)}`;
  return `@function ${serializeNativeIdentifier(name)}(${serializedParameters})${serializedReturnType}`;
}

function serializeFunctionType(type: string): string {
  if (type === "*") return "";
  return type.includes(" | ") ? ` type(${type})` : ` ${type}`;
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
    super(CSSRule.SUPPORTS_RULE, conditionText.trim());
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
    const parsed = parseContainerPrelude(
      conditionText,
      currentConstructionResourceBudget(),
    );
    const normalizedCondition = parsed?.conditionText ?? conditionText.trim();
    super(0, normalizedCondition);
    this.containerName = parsed?.name ?? "";
    this.containerQuery = parsed?.query ?? normalizedCondition;
    lockOwnProperties(this, "containerName", "containerQuery");
  }

  override get cssText(): string {
    return this.serializeGroup(`@container ${this.conditionText}`);
  }

  set cssText(_value: string) {}
}

/** An immutable statement-form `@layer` rule. */
export class CSSLayerStatementRule extends CSSRule {
  readonly #names: readonly string[];

  constructor(names: readonly string[]) {
    super(0);
    this.#names = [...names];
  }

  get nameList(): readonly string[] {
    return Object.freeze([...this.#names]);
  }

  override get cssText(): string {
    return `@layer ${this.#names.join(", ")};`;
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

  constructor(
    start: string | null,
    end: string | null,
  ) {
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

/** An immutable `@namespace` rule. */
export class CSSNamespaceRule extends CSSRule {
  readonly namespaceURI: string;
  readonly prefix: string;
  readonly #cssText: string;

  constructor(
    namespaceURI: string,
    prefix: string,
    cssText: string,
  ) {
    super(CSSRule.NAMESPACE_RULE);
    this.namespaceURI = namespaceURI;
    this.prefix = prefix;
    this.#cssText = cssText;
    lockOwnProperties(this, "namespaceURI", "prefix");
  }

  override get cssText(): string {
    return this.#cssText;
  }

  set cssText(_value: string) {}
}

class CSSGenericRule extends CSSRule {
  readonly #cssText: string;

  constructor(
    type: number,
    cssText: string,
  ) {
    super(type);
    this.#cssText = cssText;
  }

  override get cssText(): string {
    return this.#cssText;
  }

  set cssText(_value: string) {}
}

class CSSOpaqueRule extends CSSGenericRule {}

/** An immutable `@property` registration rule. */
export class CSSPropertyRule extends CSSRule {
  readonly name: string;
  readonly syntax: string;
  readonly inherits: boolean;
  readonly initialValue: string;
  readonly #cssText: string;

  constructor(
    name: string,
    syntax: string,
    inherits: boolean,
    initialValue: string,
    cssText: string,
  ) {
    super(0);
    this.name = name;
    this.syntax = syntax;
    this.inherits = inherits;
    this.initialValue = initialValue;
    this.#cssText = cssText;
    lockOwnProperties(this, "name", "syntax", "inherits", "initialValue");
  }

  override get cssText(): string { return this.#cssText; }
  set cssText(_value: string) {}
}

/** An immutable `@font-palette-values` rule. */
export class CSSFontPaletteValuesRule extends CSSRule {
  readonly name: string;
  readonly fontFamily: string;
  readonly basePalette: string;
  readonly overrideColors: string;
  readonly #cssText: string;

  constructor(
    name: string,
    fontFamily: string,
    basePalette: string,
    overrideColors: string,
    cssText: string,
  ) {
    super(0);
    this.name = name;
    this.fontFamily = fontFamily;
    this.basePalette = basePalette;
    this.overrideColors = overrideColors;
    this.#cssText = cssText;
    lockOwnProperties(this, "name", "fontFamily", "basePalette", "overrideColors");
  }

  override get cssText(): string {
    return this.#cssText;
  }

  set cssText(_value: string) {}
}

/** An immutable `@view-transition` rule. */
export class CSSViewTransitionRule extends CSSRule {
  readonly navigation: string;
  readonly types: readonly string[];
  readonly #cssText: string;

  constructor(
    navigation: string,
    types: readonly string[],
    cssText: string,
  ) {
    super(0);
    this.navigation = navigation;
    this.types = Object.freeze([...types]);
    this.#cssText = cssText;
    lockOwnProperties(this, "navigation", "types");
  }

  override get cssText(): string {
    return this.#cssText;
  }

  set cssText(_value: string) {}
}

/** A live declaration block with indexed, named, and method-based access. */
export class CSSStyleDeclaration {
  readonly [index: number]: string | undefined;

  readonly parentRule: CSSRule;
  readonly #block: NativeDeclarationBlock;

  constructor(parentRule: CSSRule) {
    assertInternalConstructor("CSSStyleDeclaration");
    this.parentRule = parentRule;
    lockOwnProperties(this, "parentRule");
    const reportDeclarationDiagnostic = (
      code: SheetOMDiagnosticCode,
      property: string,
      input: string,
    ): void => {
      const priority = code === "INVALID_PRIORITY";
      const unsupportedShorthand = code === "UNSUPPORTED_SHORTHAND_VALUE";
      (ruleDiagnostics.get(this.parentRule) ?? ignoreDiagnostic)({
        code,
        severity: "warning",
        operation: "setProperty",
        message: priority
          ? `The mutation was ignored because ${input} is not a valid priority.`
          : unsupportedShorthand
            ? `The value was ignored because the shorthand codec for ${property} cannot expand it.`
            : `The value was ignored because it is invalid for ${property}.`,
        property,
        input,
        location: null,
      });
    };
    this.#block = new NativeDeclarationBlock(
      reportDeclarationDiagnostic,
      parentRule instanceof CSSFontFaceRule
        ? "font-face"
        : parentRule instanceof CSSFunctionDeclarations
          ? "function"
          : "style",
      ruleResourceBudgets.get(parentRule) ?? defaultResourceBudget,
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
        return proxyMember(target, property, result);
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
    return this.#block.cssText;
  }

  set cssText(value: string) {
    const input = `${value}`;
    this.#block.replaceCssText(input);
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

/** A run of live descriptors inside a custom function. */
export class CSSFunctionDeclarations extends CSSRule {
  readonly #style: CSSFunctionDescriptors;

  constructor() {
    super(0);
    this.#style = new CSSFunctionDescriptors(this);
  }

  get style(): CSSFunctionDescriptors {
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

/** The declaration interface used by `CSSFunctionDeclarations`. */
export class CSSFunctionDescriptors extends CSSStyleDeclaration {
  get result(): string {
    return this.getPropertyValue("result");
  }

  set result(_value: string | null) {}
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
    const parsed = parseKeyframeRule(
      `${ruleText}`,
      ruleDiagnostics.get(this) ?? ignoreDiagnostic,
      ruleResourceBudgets.get(this) ?? defaultResourceBudget,
    );
    if (!parsed) return;
    const rules = ruleTree.children(this);
    const resourceBudget = ruleResourceBudgets.get(this) ?? defaultResourceBudget;
    assertRuleInsertionBudget(this, parsed, resourceBudget);
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
  "pad",
  "range",
  "fallback",
  "speak-as",
] as const;

/** A mutable `@counter-style` descriptor rule. */
export class CSSCounterStyleRule extends CSSRule {
  readonly #descriptors = new Map<string, string>();
  #name: string;
  #serializedName: string;

  constructor(name: string) {
    super(CSSRule.COUNTER_STYLE_RULE);
    const parsed = parseNativeCounterStyleName(
      name,
      currentConstructionResourceBudget(),
    );
    this.#name = parsed?.name ?? name;
    this.#serializedName = parsed?.serialized ?? name;
  }

  get name(): string { return this.#name; }
  set name(value: string) {
    const parsed = parseNativeCounterStyleName(
      `${value}`,
      ruleResourceBudgets.get(this) ?? defaultResourceBudget,
    );
    if (!parsed) return;
    this.#name = parsed.name;
    this.#serializedName = parsed.serialized;
  }

  get system(): string { return this.#get("system"); }
  set system(_value: string) {}
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
    const parsed = parseNativeCounterStyleDescriptor(
      name,
      text,
      ruleResourceBudgets.get(this) ?? defaultResourceBudget,
    );
    if (!parsed) return;
    this.#descriptors.set(name, parsed);
  }

  /** @internal */
  setParsedDescriptor(name: string, value: string): void {
    if (!(counterDescriptorNames as readonly string[]).includes(name)) return;
    this.#descriptors.set(name, value);
  }

  override get cssText(): string {
    const declarations: string[] = [];
    for (const name of counterDescriptorNames) {
      const value = this.#descriptors.get(name);
      if (value !== undefined) declarations.push(`${name}: ${value};`);
    }
    return `@counter-style ${this.#serializedName} {${declarations.length === 0 ? "" : ` ${declarations.join(" ")}`} }`;
  }

  set cssText(_value: string) {}
}

/** A live map exposed by one font-feature-values category. */
export class CSSFontFeatureValuesMap implements Iterable<[string, number[]]> {
  readonly #values = new Map<string, number[]>();
  readonly #serializedNames = new Map<string, string>();

  constructor() {
    assertInternalConstructor("CSSFontFeatureValuesMap");
  }

  get size(): number {
    return this.#values.size;
  }

  set(featureValueName: string, values: number[]): this {
    requireArguments(arguments.length, 2, "CSSFontFeatureValuesMap", "set");
    const name = `${featureValueName}`;
    const normalized = Array.from(values, value => toUnsignedLong(value));
    this.#values.set(name, normalized);
    this.#serializedNames.set(name, serializeNativeIdentifier(name));
    return this;
  }

  clear(): void {
    this.#values.clear();
    this.#serializedNames.clear();
  }
  delete(name: string): boolean {
    const key = `${name}`;
    this.#serializedNames.delete(key);
    return this.#values.delete(key);
  }
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

  /** @internal */
  setParsed(name: string, serializedName: string, values: number[]): void {
    this.#values.set(name, [...values]);
    this.#serializedNames.set(name, serializedName);
  }

  /** @internal */
  serializedEntries(): Array<[string, number[]]> {
    return [...this.#values].map(([name, values]) => [
      this.#serializedNames.get(name) ?? serializeNativeIdentifier(name),
      values,
    ]);
  }
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
  set fontFamily(value: string) {
    this.#fontFamily = serializeNativeFontFamily(`${value}`);
  }

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
      const declarations = map.serializedEntries()
        .map(([key, values]) => `${key}:${values.length === 0 ? "" : ` ${values.join(" ")}`};`)
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
    this.#selectorText = normalizeSelectorText(
      `${selectorText}`,
      currentConstructionResourceBudget(),
    )
      ?? `${selectorText}`;
    this.style = new CSSStyleDeclaration(this);
    lockOwnProperties(this, "style");
  }

  get selectorText(): string {
    return this.#selectorText;
  }

  set selectorText(value: string) {
    const normalized = normalizeSelectorText(
      `${value}`,
      ruleResourceBudgets.get(this) ?? defaultResourceBudget,
    );
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

function parseKeyframeRule(
  ruleText: string,
  reportDiagnostic: ReportDiagnostic,
  resourceBudget: NativeResourceBudget,
): CSSKeyframeRule | null {
  const keyframes = parseNativeRule(`@keyframes sheetom { ${ruleText} }`, resourceBudget);
  if (keyframes?.kind !== "keyframes" || keyframes.children.length !== 1) return null;
  const frame = createRuleFromNative(
    keyframes.children[0]!,
    reportDiagnostic,
    false,
    resourceBudget,
  );
  return frame instanceof CSSKeyframeRule ? frame : null;
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
  if (rule instanceof CSSNestedDeclarations || rule instanceof CSSFunctionDeclarations) {
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

  return {
    kind: "block",
    header: groupingRuleHeader(rule),
    children: ruleTree.children(rule),
  };
}

function groupingRuleHeader(rule: CSSGroupingRule): string {
  if (rule instanceof CSSFunctionRule) {
    return functionRuleHeaders.get(rule) ?? "@function";
  }
  if (rule instanceof CSSMediaRule) return `@media ${rule.media.mediaText}`;
  if (rule instanceof CSSSupportsRule) return `@supports ${rule.conditionText}`;
  if (rule instanceof CSSContainerRule) return `@container ${rule.conditionText}`;
  if (rule instanceof CSSLayerBlockRule) {
    return `@layer${rule.name === "" ? "" : ` ${rule.name}`}`;
  }
  if (rule instanceof CSSScopeRule) {
    const start = rule.start === null ? "" : ` (${rule.start})`;
    const end = rule.end === null ? "" : ` to (${rule.end})`;
    return `@scope${start}${end}`;
  }
  if (rule instanceof CSSStartingStyleRule) return "@starting-style";

  const blockIndex = rule.cssText.indexOf("{");
  return blockIndex === -1 ? rule.cssText : rule.cssText.slice(0, blockIndex).trimEnd();
}

function parseScopePrelude(
  prelude: string,
  resourceBudget: NativeResourceBudget,
): [string | null, string | null] {
  const parsed = parseNativeScopePrelude(prelude, resourceBudget);
  return parsed ? [parsed.start, parsed.end] : [null, null];
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

function createRuleFromNative(
  description: NativeRuleDescription,
  reportDiagnostic: ReportDiagnostic,
  preserveImports: boolean,
  resourceBudget: NativeResourceBudget,
): CSSRule | null {
  return constructWithResourceBudget(() =>
    createRuleFromNativeInternal(
      description,
      reportDiagnostic,
      preserveImports,
      resourceBudget,
    ),
    resourceBudget,
  );
}

function createRuleFromNativeInternal(
  description: NativeRuleDescription,
  reportDiagnostic: ReportDiagnostic,
  preserveImports: boolean,
  resourceBudget: NativeResourceBudget,
): CSSRule | null {
  let rule: CSSRule;
  switch (description.kind) {
    case "style": {
      const style = new CSSStyleRule(description.prelude);
      style.style.cssText = description.declarations;
      replaceGroupingRules(
        style,
        createNativeChildren(
          description,
          reportDiagnostic,
          preserveImports,
          resourceBudget,
        ),
      );
      rule = style;
      break;
    }
    case "nested-declarations": {
      const nested = new CSSNestedDeclarations();
      nested.style.cssText = description.declarations;
      rule = nested;
      break;
    }
    case "function-declarations": {
      const declarations = new CSSFunctionDeclarations();
      declarations.style.cssText = description.declarations;
      rule = declarations;
      break;
    }
    case "import": {
      if (!preserveImports) return null;
      const prelude = description.cssText
        .replace(/^@import\s+/iu, "")
        .replace(/;\s*$/u, "");
      const parsed = parseImportPrelude(prelude);
      if (!parsed) return null;
      rule = new CSSImportRule(
        parsed.href,
        parsed.media,
        parsed.layer,
        parsed.supports,
      );
      break;
    }
    case "font-face": {
      const fontFace = new CSSFontFaceRule();
      fontFace.style.cssText = description.declarations;
      rule = fontFace;
      break;
    }
    case "page": {
      const page = new CSSPageRule(description.prelude);
      page.style.cssText = description.declarations;
      replaceGroupingRules(
        page,
        createNativeChildren(
          description,
          reportDiagnostic,
          preserveImports,
          resourceBudget,
        ),
      );
      rule = page;
      break;
    }
    case "margin": {
      const margin = new CSSMarginRule(description.prelude);
      margin.style.cssText = description.declarations;
      rule = margin;
      break;
    }
    case "position-try": {
      const positionTry = new CSSPositionTryRule(description.prelude);
      positionTry.style.cssText = description.declarations;
      rule = positionTry;
      break;
    }
    case "keyframes": {
      const keyframes = new CSSKeyframesRule(description.prelude);
      const frames = createNativeChildren(
        description,
        reportDiagnostic,
        preserveImports,
        resourceBudget,
      ).filter((candidate): candidate is CSSKeyframeRule => candidate instanceof CSSKeyframeRule);
      replaceKeyframeRules(keyframes, frames);
      rule = keyframes;
      break;
    }
    case "keyframe": {
      const keyframe = new CSSKeyframeRule(description.prelude);
      keyframe.style.cssText = description.declarations;
      rule = keyframe;
      break;
    }
    case "counter-style": {
      const counterStyle = new CSSCounterStyleRule(description.prelude);
      hydrateCounterStyleDescriptors(
        counterStyle,
        description.declarations,
        resourceBudget,
      );
      rule = counterStyle;
      break;
    }
    case "font-feature-values": {
      const featureValues = new CSSFontFeatureValuesRule(
        description.prelude,
      );
      for (const mapDescription of description.children) {
        if (mapDescription.kind !== "font-feature-map") continue;
        const map = featureValues.featureMap(mapDescription.prelude);
        if (!map) continue;
        for (const entry of mapDescription.children) {
          if (entry.kind !== "font-feature-entry") continue;
          const values = entry.declarations === ""
            ? []
            : entry.declarations.split(" ").map(value => Number(value));
          map.setParsed(entry.prelude, entry.cssText, values);
        }
      }
      rule = featureValues;
      break;
    }
    case "property": {
      return new CSSPropertyRule(
        description.prelude,
        nativeRuleDescriptor(description, "syntax"),
        nativeRuleDescriptor(description, "inherits") === "true",
        nativeRuleDescriptor(description, "initial-value"),
        description.cssText,
      );
    }
    case "font-palette-values":
      return new CSSFontPaletteValuesRule(
        description.prelude,
        nativeRuleDescriptor(description, "font-family"),
        nativeRuleDescriptor(description, "base-palette"),
        nativeRuleDescriptor(description, "override-colors"),
        description.cssText,
      );
    case "view-transition":
      return new CSSViewTransitionRule(
        nativeRuleDescriptor(description, "navigation"),
        description.children
          .filter(candidate => candidate.kind === "view-transition-type")
          .map(candidate => candidate.prelude),
        description.cssText,
      );
    case "function": {
      const parameters = description.children
        .filter(candidate => candidate.kind === "function-parameter")
        .map(candidate => {
          const defaultValue = candidate.children.find(
            child => child.kind === "property-descriptor" && child.prelude === "default-value",
          );
          return defaultValue
            ? {
                defaultValue: defaultValue.declarations,
                name: candidate.prelude,
                type: candidate.declarations,
              }
            : {
                name: candidate.prelude,
                type: candidate.declarations,
              };
        });
      rule = new CSSFunctionRule(
        description.prelude,
        parameters,
        description.declarations,
      );
      break;
    }
    case "namespace":
      return new CSSNamespaceRule(
        nativeRuleDescriptor(description, "namespace-uri"),
        nativeRuleDescriptor(description, "prefix"),
        description.cssText,
      );
    case "media":
      rule = new CSSMediaRule(description.prelude);
      break;
    case "supports":
      rule = new CSSSupportsRule(description.prelude);
      break;
    case "container":
      rule = new CSSContainerRule(description.prelude);
      break;
    case "layer-block":
      rule = new CSSLayerBlockRule(description.prelude);
      break;
    case "scope": {
      const [start, end] = parseScopePrelude(description.prelude, resourceBudget);
      rule = new CSSScopeRule(start, end);
      break;
    }
    case "starting-style":
      rule = new CSSStartingStyleRule();
      break;
    case "layer-statement": {
      const names = description.children
        .filter(candidate => candidate.kind === "layer-name")
        .map(candidate => candidate.prelude);
      return new CSSLayerStatementRule(names);
    }
    default:
      return new CSSGenericRule(
        genericRuleType(description.cssText.match(/^@([^\s{;]+)/u)?.[1] ?? ""),
        description.cssText,
      );
  }

  ruleDiagnostics.set(rule, reportDiagnostic);
  if (
    rule instanceof CSSGroupingRule
    && !(rule instanceof CSSStyleRule)
    && !(rule instanceof CSSPageRule)
    && !(rule instanceof CSSKeyframesRule)
  ) {
    replaceGroupingRules(
      rule,
      createNativeChildren(
        description,
        reportDiagnostic,
        preserveImports,
        resourceBudget,
      ),
    );
  }
  return rule;
}

function nativeRuleDescriptor(description: NativeRuleDescription, name: string): string {
  return description.children
    .find(candidate => candidate.kind === "property-descriptor" && candidate.prelude === name)
    ?.declarations ?? "";
}

function hydrateCounterStyleDescriptors(
  rule: CSSCounterStyleRule,
  declarations: string,
  resourceBudget: NativeResourceBudget,
): void {
  for (const descriptor of parseNativeCounterStyleDescriptors(
    declarations,
    resourceBudget,
  )) {
    rule.setParsedDescriptor(descriptor.name, descriptor.value);
  }
}

function createNativeChildren(
  description: NativeRuleDescription,
  reportDiagnostic: ReportDiagnostic,
  preserveImports: boolean,
  resourceBudget: NativeResourceBudget,
): CSSRule[] {
  const children: CSSRule[] = [];
  for (const child of description.children) {
    if (
      child.kind === "function-parameter"
      || child.kind === "property-descriptor"
      || child.kind === "view-transition-type"
      || child.kind === "layer-name"
    ) continue;
    const rule = createRuleFromNative(
      child,
      reportDiagnostic,
      preserveImports,
      resourceBudget,
    );
    if (rule instanceof CSSFunctionDeclarations && rule.style.length === 0) continue;
    if (rule) children.push(rule);
  }
  return children;
}

function parseStrictRule(
  ruleText: string,
  reportDiagnostic: ReportDiagnostic,
  preserveImports = false,
  parentRule: CSSGroupingRule | null = null,
  resourceBudget: NativeResourceBudget = defaultResourceBudget,
): CSSRule | null {
  if (parentRule instanceof CSSPageRule) {
    const page = parseNativeRule(`@page { ${ruleText} }`, resourceBudget);
    if (page?.kind !== "page" || page.children.length !== 1) return null;
    const child = page.children[0];
    if (!child || child.kind !== "margin" || !pageMarginRuleNames.has(child.prelude)) return null;
    return createRuleFromNative(
      child,
      reportDiagnostic,
      preserveImports,
      resourceBudget,
    );
  }
  const description = parseNativeRule(ruleText, resourceBudget)
    ?? (parentRule && hasFunctionAncestor(parentRule)
      ? parseNativeRuleWithErrorRecovery(ruleText, resourceBudget)
      : null);
  if (!description || (description.kind === "import" && !preserveImports)) return null;
  return createRuleFromNative(
    description,
    reportDiagnostic,
    preserveImports,
    resourceBudget,
  );
}

function hasFunctionAncestor(rule: CSSRule): boolean {
  let current: CSSRule | null = rule;
  while (current) {
    if (current instanceof CSSFunctionRule) return true;
    current = current.parentRule;
  }
  return false;
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
  resourceBudget: NativeResourceBudget,
): CSSRule[] {
  const rules: CSSRule[] = [];
  let parsedRuleCount = 0;
  for (const rawRule of scanTopLevelRules(cssText, resourceBudget)) {
    const rule = parseStrictRule(
      rawRule,
      reportDiagnostic,
      preserveImports,
      null,
      resourceBudget,
    );
    if (rule) {
      parsedRuleCount += ruleForestSize([rule]);
      assertRuleCountBudget(parsedRuleCount, resourceBudget);
      rules.push(rule);
      continue;
    }
    if (!preserveImports) continue;
    const opaque = constructWithResourceBudget(
      () => new CSSOpaqueRule(0, rawRule),
      resourceBudget,
    );
    parsedRuleCount += 1;
    assertRuleCountBudget(parsedRuleCount, resourceBudget);
    rules.push(opaque);
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
    const resourceBudget = normalizeResourceBudget(normalizedOptions.resourceBudget);
    sheetResourceBudgets.set(this, resourceBudget);
    sheetRuleArrays.set(this, this.#rules);
    this.#diagnostics = Boolean(normalizedOptions.diagnostics) ? [] : null;
    this.#constructedBaseURL = normalizedOptions.baseURL === undefined
      ? "about:blank"
      : `${normalizedOptions.baseURL}`;
    const media = normalizedOptions.media === undefined
      ? ""
      : `${normalizedOptions.media}`;
    this.media = constructWithResourceBudget(
      () => new MediaList(media),
      resourceBudget,
    );
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
    const resourceBudget = sheetResourceBudgets.get(this) ?? defaultResourceBudget;
    const rule = parseStrictRule(
      `${ruleText}`,
      this.#reportDiagnostic,
      regular,
      null,
      resourceBudget,
    );
    if (!rule) throw new DOMException("The rule could not be parsed.", "SyntaxError");

    const precedingRules = this.#rules.slice(0, normalizedIndex);
    const followingRules = this.#rules.slice(normalizedIndex);
    const invalidImportOrder = rule instanceof CSSImportRule
      ? precedingRules.some(candidate => !(candidate instanceof CSSImportRule))
      : followingRules.some(candidate => candidate instanceof CSSImportRule);
    if (invalidImportOrder) {
      throw new DOMException("The rule violates stylesheet ordering.", "HierarchyRequestError");
    }

    assertRuleInsertionBudget(this, rule, resourceBudget);
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
      sheetResourceBudgets.get(this) ?? defaultResourceBudget,
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
