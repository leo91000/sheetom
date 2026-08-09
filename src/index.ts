import { transform, transformStyleAttribute } from "lightningcss";
import * as csstree from "css-tree";

import {
  chromiumShorthandLonghands,
  chromiumSupportedProperties,
} from "./chromium-properties.js";

const encoder = new TextEncoder();
const decoder = new TextDecoder();
const arrayIndexPattern = /^(0|[1-9]\d*)$/;
const paddingLonghands = [
  "padding-top",
  "padding-right",
  "padding-bottom",
  "padding-left",
] as const;
const fourSideShorthandNames = new Set([
  "padding",
  "margin",
  "inset",
  "border-width",
  "border-style",
  "border-color",
  "scroll-margin",
  "scroll-padding",
]);
const regularSheetMetadata = new WeakMap<
  object,
  { href: string | null; baseURL: string }
>();
const zeroLengthProperties = new Set([
  "width",
  "height",
  "min-width",
  "min-height",
  "max-width",
  "max-height",
  "padding",
  ...paddingLonghands,
]);
const unsignedLongRange = 2 ** 32;

for (const shorthand of [
  "padding",
  "margin",
  "inset",
  "border-width",
  "scroll-margin",
  "scroll-padding",
]) {
  zeroLengthProperties.add(shorthand);
  for (const longhand of chromiumShorthandLonghands[shorthand] ?? []) {
    zeroLengthProperties.add(longhand);
  }
}

interface ParsedPropertyValue {
  observableValue: string;
  safeValue: string;
  pendingSubstitution: boolean;
}

interface PendingSubstitutionGroup {
  shorthand: string;
  observableValue: string;
  safeValue: string;
}

interface DeclarationRecord extends ParsedPropertyValue {
  name: string;
  important: boolean;
  pendingGroup: PendingSubstitutionGroup | null;
}

export interface SheetOMDiagnostic {
  code: string;
  severity: "warning";
  operation: "setProperty";
  message: string;
  property: string;
  input: string;
  location: null;
}

export interface CSSStyleSheetOptions {
  baseURL?: string;
  media?: string;
  disabled?: boolean;
  diagnostics?: boolean;
}

export interface ParseStyleSheetOptions extends CSSStyleSheetOptions {
  href?: string;
}

type ReportDiagnostic = (diagnostic: SheetOMDiagnostic) => void;

const ignoreDiagnostic: ReportDiagnostic = () => {};

function isUnknownRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

function isToken(value: unknown, type: string, tokenValue?: string): boolean {
  if (!isUnknownRecord(value) || value.type !== "token") return false;
  const token = value.value;
  if (!isUnknownRecord(token) || token.type !== type) return false;
  return tokenValue === undefined || token.value === tokenValue;
}

function isWhitespaceToken(value: unknown): boolean {
  return isToken(value, "white-space");
}

function significantTokens(values: unknown[]): unknown[] {
  return values.filter(value => !isWhitespaceToken(value));
}

function functionValue(
  value: unknown,
): { name: string; arguments: unknown[] } | null {
  if (!isUnknownRecord(value) || value.type !== "function") return null;
  const details = value.value;
  if (!isUnknownRecord(details) || typeof details.name !== "string") return null;
  if (!Array.isArray(details.arguments)) return null;
  return { name: details.name.toLowerCase(), arguments: details.arguments };
}

function isValidAttrType(value: unknown): boolean {
  const typeFunction = functionValue(value);
  if (!typeFunction || typeFunction.name !== "type") return false;

  const tokens = significantTokens(typeFunction.arguments);
  if (tokens.length === 1 && isToken(tokens[0], "delim", "*")) return true;
  if (tokens.length < 3) return false;
  if (!isToken(tokens[0], "delim", "<")) return false;
  if (!isToken(tokens[tokens.length - 1], "delim", ">")) return false;

  let depth = 0;
  for (const token of tokens) {
    if (isToken(token, "delim", "<")) depth += 1;
    if (!isToken(token, "delim", ">")) continue;
    depth -= 1;
    if (depth < 0) return false;
  }
  return depth === 0;
}

function isValidAttrFunction(argumentsList: unknown[]): boolean {
  const tokens = significantTokens(argumentsList);
  if (!isToken(tokens[0], "ident")) return false;
  if (tokens.length === 1) return true;

  let index = 1;
  const second = tokens[index];
  if (isToken(second, "comma")) return true;

  if (isValidAttrType(second)) {
    index += 1;
  } else if (
    isToken(second, "ident") ||
    isToken(second, "delim", "%")
  ) {
    index += 1;
  } else {
    return false;
  }

  if (index === tokens.length) return true;
  return isToken(tokens[index], "comma");
}

function isConditionFunction(value: unknown): boolean {
  const condition = functionValue(value);
  if (!condition || !["media", "style", "supports"].includes(condition.name)) {
    return false;
  }
  return significantTokens(condition.arguments).length > 0;
}

function isElseToken(value: unknown): boolean {
  return isToken(value, "ident", "else");
}

function isValidIfFunction(argumentsList: unknown[]): boolean {
  const tokens = significantTokens(argumentsList);
  const branches: unknown[][] = [[]];
  for (const token of tokens) {
    if (isToken(token, "semicolon")) {
      branches.push([]);
      continue;
    }
    branches[branches.length - 1]?.push(token);
  }

  if (branches.length === 0 || branches.some(branch => branch.length === 0)) {
    return false;
  }

  let hasCondition = false;
  for (let index = 0; index < branches.length; index += 1) {
    const branch = branches[index];
    if (!branch) return false;

    const colonIndex = branch.findIndex(token => isToken(token, "colon"));
    if (colonIndex <= 0 || colonIndex === branch.length - 1) return false;

    const condition = branch.slice(0, colonIndex);
    const isElse = condition.length === 1 && isElseToken(condition[0]);
    if (isElse) {
      if (index !== branches.length - 1) return false;
      continue;
    }

    if (!condition.some(isConditionFunction)) return false;
    hasCondition = true;
  }

  return hasCondition;
}

interface SubstitutionAnalysis {
  found: boolean;
  valid: boolean;
}

function analyzeSubstitutions(value: unknown): SubstitutionAnalysis {
  if (Array.isArray(value)) {
    let found = false;
    for (const item of value) {
      const result = analyzeSubstitutions(item);
      found ||= result.found;
      if (!result.valid) return { found, valid: false };
    }
    return { found, valid: true };
  }

  if (!isUnknownRecord(value)) return { found: false, valid: true };

  let found = value.type === "var" || value.type === "env";
  const cssFunction = functionValue(value);
  if (cssFunction?.name === "attr") {
    found = true;
    if (!isValidAttrFunction(cssFunction.arguments)) {
      return { found, valid: false };
    }
  }
  if (cssFunction?.name === "if") {
    found = true;
    if (!isValidIfFunction(cssFunction.arguments)) {
      return { found, valid: false };
    }
  }

  for (const key in value) {
    if (!Object.hasOwn(value, key)) continue;
    const result = analyzeSubstitutions(value[key]);
    found ||= result.found;
    if (!result.valid) return { found, valid: false };
  }

  return { found, valid: true };
}

function serializeIdentifier(value: string): string {
  const characters = [...value];
  let result = "";

  for (let index = 0; index < characters.length; index += 1) {
    const character = characters[index];
    if (character === undefined) continue;
    const codePoint = character.codePointAt(0) ?? 0;

    if (codePoint === 0) {
      result += "�";
      continue;
    }
    if ((codePoint >= 1 && codePoint <= 31) || codePoint === 127) {
      result += `\\${codePoint.toString(16)} `;
      continue;
    }
    if (
      codePoint >= 48 &&
      codePoint <= 57 &&
      (index === 0 || (index === 1 && characters[0] === "-"))
    ) {
      result += `\\${codePoint.toString(16)} `;
      continue;
    }
    if (index === 0 && character === "-" && characters.length === 1) {
      result += "\\-";
      continue;
    }
    if (
      codePoint >= 128 ||
      character === "-" ||
      character === "_" ||
      (codePoint >= 48 && codePoint <= 57) ||
      (codePoint >= 65 && codePoint <= 90) ||
      (codePoint >= 97 && codePoint <= 122)
    ) {
      result += character;
      continue;
    }
    result += `\\${character}`;
  }

  return result;
}

function containsPriorityDelimiter(value: string): boolean {
  let found = false;
  csstree.tokenize(value, (type, start, end) => {
    if (csstree.tokenNames[type] !== "delim-token") return;
    if (value.slice(start, end) === "!") found = true;
  });
  return found;
}

function parsePropertyValue(
  name: string,
  observableValue: string,
): ParsedPropertyValue | null {
  if (name === "--") return null;
  if (!name.startsWith("--") && !chromiumSupportedProperties.has(name)) {
    return null;
  }
  if (containsPriorityDelimiter(observableValue)) return null;

  let declaration: unknown;

  try {
    const result = transformStyleAttribute({
      code: encoder.encode(
        `${name.startsWith("--") ? serializeIdentifier(name) : name}: ${observableValue}`,
      ),
      visitor: {
        Declaration(candidate) {
          declaration = candidate;
        },
      },
    });

    if (!isUnknownRecord(declaration)) return null;
    if (!Object.hasOwn(declaration, "property")) return null;

    const pendingSubstitution = declaration.property === "unparsed";
    if (pendingSubstitution) {
      const analysis = analyzeSubstitutions(declaration);
      if (!analysis.found || !analysis.valid) return null;
    }

    const serialized = decoder.decode(result.code);
    const colonIndex = serialized.indexOf(":");
    if (colonIndex === -1) return null;

    const trimmedObservableValue = observableValue.trim();
    return {
      observableValue:
        trimmedObservableValue === "0" && zeroLengthProperties.has(name)
          ? "0px"
          : trimmedObservableValue,
      safeValue: serialized.slice(colonIndex + 1).trim(),
      pendingSubstitution,
    };
  } catch {
    return null;
  }
}

function splitTopLevelWhitespace(value: string): string[] {
  const components: string[] = [];
  let current = "";
  let depth = 0;
  let quote = "";
  let escaped = false;

  for (const character of value) {
    if (escaped) {
      current += character;
      escaped = false;
      continue;
    }

    if (character === "\\") {
      current += character;
      escaped = true;
      continue;
    }

    if (quote !== "") {
      current += character;
      if (character === quote) quote = "";
      continue;
    }

    if (character === '"' || character === "'") {
      current += character;
      quote = character;
      continue;
    }

    if (character === "(" || character === "[" || character === "{") {
      current += character;
      depth += 1;
      continue;
    }

    if (character === ")" || character === "]" || character === "}") {
      current += character;
      if (depth > 0) depth -= 1;
      continue;
    }

    if (/\s/.test(character) && depth === 0) {
      if (current !== "") components.push(current);
      current = "";
      continue;
    }

    current += character;
  }

  if (current !== "") components.push(current);
  return components;
}

function expandFourSides(value: string): [string, string, string, string] | null {
  const components = splitTopLevelWhitespace(value);
  if (components.length < 1 || components.length > 4) return null;

  const top = components[0];
  if (top === undefined) return null;

  const right = components[1] ?? top;
  const bottom = components[2] ?? top;
  const left = components[3] ?? right;
  return [top, right, bottom, left];
}

function compressFourSides(values: [string, string, string, string]): string {
  const [top, right, bottom, left] = values;
  if (top === right && top === bottom && top === left) return top;
  if (top === bottom && right === left) return `${top} ${right}`;
  if (right === left) return `${top} ${right} ${bottom}`;
  return values.join(" ");
}

function expandStaticFourSide(
  name: string,
  parsed: ParsedPropertyValue,
  important: boolean,
): DeclarationRecord[] | null {
  if (!fourSideShorthandNames.has(name)) return null;
  const longhands = getShorthandLonghands(name);
  if (!longhands || longhands.length !== 4) return null;

  const observableSides = expandFourSides(parsed.observableValue);
  const safeSides = expandFourSides(parsed.safeValue);
  if (!observableSides || !safeSides) return null;

  const records: DeclarationRecord[] = [];
  for (let index = 0; index < longhands.length; index += 1) {
    const longhand = longhands[index];
    const observableValue = observableSides[index];
    const safeValue = safeSides[index];
    if (!longhand || observableValue === undefined || safeValue === undefined) {
      return null;
    }
    records.push({
      name: longhand,
      observableValue,
      safeValue,
      pendingSubstitution: false,
      important,
      pendingGroup: null,
    });
  }
  return records;
}

function namedPropertyToCSS(property: string): string {
  if (property === "cssFloat") return "float";

  let cssName = property.replace(/[A-Z]/g, character => `-${character.toLowerCase()}`);
  if (/^(webkit|moz|ms|o)-/.test(cssName)) cssName = `-${cssName}`;
  return cssName;
}

function getShorthandLonghands(name: string): readonly string[] | null {
  if (!Object.hasOwn(chromiumShorthandLonghands, name)) return null;
  return chromiumShorthandLonghands[name] ?? null;
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

export class CSSRule {
  static readonly STYLE_RULE = 1;
  static readonly IMPORT_RULE = 3;
  static readonly MEDIA_RULE = 4;
  static readonly FONT_FACE_RULE = 5;
  static readonly PAGE_RULE = 6;
  static readonly KEYFRAMES_RULE = 7;
  static readonly NAMESPACE_RULE = 10;
  static readonly SUPPORTS_RULE = 12;

  readonly type: number;
  parentRule: CSSRule | null = null;
  parentStyleSheet: CSSStyleSheet | null = null;

  protected constructor(type: number) {
    this.type = type;
  }

  get cssText(): string {
    return "";
  }
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
}

export class CSSStyleDeclaration {
  readonly [index: number]: string | undefined;

  readonly parentRule: CSSStyleRule;

  readonly #records: DeclarationRecord[] = [];
  readonly #reportDiagnostic: ReportDiagnostic;

  constructor(
    parentRule: CSSStyleRule,
    reportDiagnostic: ReportDiagnostic = ignoreDiagnostic,
  ) {
    this.parentRule = parentRule;
    this.#reportDiagnostic = reportDiagnostic;

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
    return this.#serializeRecords(false, "", " ");
  }

  set cssText(value: string) {
    let parsed: csstree.CssNode;
    try {
      parsed = csstree.parse(`${value}`, { context: "declarationList" });
    } catch {
      this.#records.splice(0);
      return;
    }

    if (parsed.type !== "DeclarationList") {
      this.#records.splice(0);
      return;
    }

    const winners = new Map<
      string,
      DeclarationRecord & { sourceIndex: number; subIndex: number }
    >();
    let sourceIndex = 0;

    const consider = (
      record: DeclarationRecord,
      subIndex: number,
    ): void => {
      const existing = winners.get(record.name);
      if (existing?.important && !record.important) return;
      winners.set(record.name, { ...record, sourceIndex, subIndex });
    };

    for (const child of parsed.children) {
      if (child.type !== "Declaration") continue;

      const name = child.property.startsWith("--")
        ? child.property
        : child.property.toLowerCase();
      const valueText = csstree.generate(child.value);
      const propertyValue = parsePropertyValue(name, valueText);
      const important = child.important === true || child.important === "important";
      if (!propertyValue) {
        sourceIndex += 1;
        continue;
      }

      const shorthandLonghands = getShorthandLonghands(name);
      if (propertyValue.pendingSubstitution && shorthandLonghands) {
        const pendingGroup: PendingSubstitutionGroup = {
          shorthand: name,
          observableValue: propertyValue.observableValue,
          safeValue: propertyValue.safeValue,
        };
        for (let index = 0; index < shorthandLonghands.length; index += 1) {
          const longhand = shorthandLonghands[index];
          if (!longhand) continue;
          consider(
            {
              name: longhand,
              observableValue: "",
              safeValue: "",
              pendingSubstitution: true,
              important,
              pendingGroup,
            },
            index,
          );
        }
        sourceIndex += 1;
        continue;
      }

      const expanded = expandStaticFourSide(name, propertyValue, important);
      if (expanded) {
        for (let index = 0; index < expanded.length; index += 1) {
          const record = expanded[index];
          if (!record) continue;
          consider(record, index);
        }
        sourceIndex += 1;
        continue;
      }

      if (name !== "padding") {
        consider({ name, ...propertyValue, important, pendingGroup: null }, 0);
        sourceIndex += 1;
        continue;
      }
      sourceIndex += 1;
    }

    const records = [...winners.values()].sort((left, right) => {
      if (left.important !== right.important) return left.important ? 1 : -1;
      return left.sourceIndex - right.sourceIndex || left.subIndex - right.subIndex;
    });
    this.#records.splice(0, this.#records.length, ...records);
  }

  get length(): number {
    return this.#records.length;
  }

  item(index: number): string {
    return this.#records[toUnsignedLong(index)]?.name ?? "";
  }

  getPropertyValue(name: string): string {
    const stringName = `${name}`;
    const normalizedName = stringName.startsWith("--")
      ? stringName
      : stringName.toLowerCase();
    const shorthand = this.#shorthand(normalizedName, false);
    if (shorthand) {
      return shorthand.value;
    }
    return this.#records.find(record => record.name === normalizedName)?.observableValue ?? "";
  }

  getPropertyPriority(name: string): string {
    const stringName = `${name}`;
    const normalizedName = stringName.startsWith("--")
      ? stringName
      : stringName.toLowerCase();
    const shorthand = this.#shorthand(normalizedName, false);
    if (shorthand) {
      return shorthand.important ? "important" : "";
    }
    const record = this.#records.find(candidate => candidate.name === normalizedName);
    return record?.important ? "important" : "";
  }

  setProperty(name: string, value: string | null, priority = ""): void {
    const stringName = `${name}`;
    const stringValue = value === null ? "" : `${value}`;
    const stringPriority = `${priority}`;
    const normalizedName = stringName.startsWith("--")
      ? stringName
      : stringName.toLowerCase();
    const normalizedPriority = stringPriority.toLowerCase();
    if (normalizedPriority !== "" && normalizedPriority !== "important") {
      this.#reportDiagnostic({
        code: "INVALID_PRIORITY",
        severity: "warning",
        operation: "setProperty",
        message: `The mutation was ignored because ${stringPriority} is not a valid priority.`,
        property: normalizedName,
        input: stringPriority,
        location: null,
      });
      return;
    }

    if (stringValue === "") {
      this.removeProperty(normalizedName);
      return;
    }

    const parsed = parsePropertyValue(normalizedName, stringValue);
    if (!parsed) {
      this.#reportDiagnostic({
        code: "INVALID_PROPERTY_VALUE",
        severity: "warning",
        operation: "setProperty",
        message: `The value was ignored because it is invalid for ${normalizedName}.`,
        property: normalizedName,
        input: stringValue,
        location: null,
      });
      return;
    }

    const shorthandLonghands = getShorthandLonghands(normalizedName);
    if (parsed.pendingSubstitution && shorthandLonghands) {
      const pendingGroup: PendingSubstitutionGroup = {
        shorthand: normalizedName,
        observableValue: parsed.observableValue,
        safeValue: parsed.safeValue,
      };
      for (const longhand of shorthandLonghands) {
        this.#commitRecord(
          longhand,
          { observableValue: "", safeValue: "", pendingSubstitution: true },
          normalizedPriority === "important",
          pendingGroup,
        );
      }
      return;
    }

    const expanded = expandStaticFourSide(
      normalizedName,
      parsed,
      normalizedPriority === "important",
    );
    if (expanded) {
      for (const record of expanded) {
        this.#commitRecord(record.name, record, record.important);
      }
      return;
    }

    this.#commitRecord(normalizedName, parsed, normalizedPriority === "important");
  }

  removeProperty(name: string): string {
    const stringName = `${name}`;
    const normalizedName = stringName.startsWith("--")
      ? stringName
      : stringName.toLowerCase();
    const shorthandLonghands = getShorthandLonghands(normalizedName);
    if (shorthandLonghands) {
      const previousValue = this.getPropertyValue(normalizedName);
      const names = new Set([normalizedName, ...shorthandLonghands]);
      for (let index = this.#records.length - 1; index >= 0; index -= 1) {
        const record = this.#records[index];
        if (record && names.has(record.name)) {
          this.#records.splice(index, 1);
        }
      }
      return previousValue;
    }

    const index = this.#records.findIndex(record => record.name === normalizedName);
    if (index === -1) return "";

    const [removed] = this.#records.splice(index, 1);
    return removed?.observableValue ?? "";
  }

  serializeSafe(indent: string): string {
    return this.#serializeRecords(true, indent, "\n");
  }

  #commitRecord(
    name: string,
    parsed: ParsedPropertyValue,
    important: boolean,
    pendingGroup: PendingSubstitutionGroup | null = null,
  ): void {
    const existing = this.#records.find(record => record.name === name);
    if (existing) {
      existing.observableValue = parsed.observableValue;
      existing.safeValue = parsed.safeValue;
      existing.pendingSubstitution = parsed.pendingSubstitution;
      existing.important = important;
      existing.pendingGroup = pendingGroup;
      return;
    }

    this.#records.push({ name, ...parsed, important, pendingGroup });
  }

  #shorthand(
    name: string,
    safe: boolean,
  ): { value: string; important: boolean } | null {
    const longhands = getShorthandLonghands(name);
    if (!longhands) return null;
    const records = longhands.map(longhand =>
      this.#records.find(record => record.name === longhand),
    );
    if (records.some(record => record === undefined)) return null;

    const first = records[0];
    if (!first) return null;
    if (records.some(record => record?.important !== first.important)) return null;

    const pendingGroup = first.pendingGroup;
    if (
      pendingGroup?.shorthand === name &&
      records.every(record => record?.pendingGroup === pendingGroup)
    ) {
      return {
        value: safe ? pendingGroup.safeValue : pendingGroup.observableValue,
        important: first.important,
      };
    }
    if (records.some(record => record?.pendingGroup)) return null;
    if (!fourSideShorthandNames.has(name) || records.length !== 4) return null;

    const [top, right, bottom, left] = records;
    if (!top || !right || !bottom || !left) return null;
    return {
      value: compressFourSides([
        safe ? top.safeValue : top.observableValue,
        safe ? right.safeValue : right.observableValue,
        safe ? bottom.safeValue : bottom.observableValue,
        safe ? left.safeValue : left.observableValue,
      ]),
      important: first.important,
    };
  }

  #serializeRecords(safe: boolean, indent: string, separator: string): string {
    const declarations: string[] = [];
    const writtenPendingGroups = new Set<PendingSubstitutionGroup>();
    const writtenStaticShorthands = new Set<string>();
    const staticShorthands = new Map<
      string,
      { value: string; important: boolean }
    >();
    for (const name of fourSideShorthandNames) {
      const shorthand = this.#shorthand(name, safe);
      if (shorthand) staticShorthands.set(name, shorthand);
    }

    for (const record of this.#records) {
      const pendingGroup = record.pendingGroup;
      if (pendingGroup) {
        const shorthand = this.#shorthand(pendingGroup.shorthand, safe);
        if (shorthand) {
          if (writtenPendingGroups.has(pendingGroup)) continue;
          declarations.push(
            `${indent}${pendingGroup.shorthand}: ${shorthand.value}${shorthand.important ? " !important" : ""};`,
          );
          writtenPendingGroups.add(pendingGroup);
          continue;
        }
      }

      let staticShorthandWritten = false;
      for (const [name, shorthand] of staticShorthands) {
        const longhands = getShorthandLonghands(name);
        if (!longhands?.includes(record.name)) continue;
        if (!writtenStaticShorthands.has(name)) {
          declarations.push(
            `${indent}${name}: ${shorthand.value}${shorthand.important ? " !important" : ""};`,
          );
          writtenStaticShorthands.add(name);
        }
        staticShorthandWritten = true;
        break;
      }
      if (staticShorthandWritten) continue;

      declarations.push(
        `${indent}${record.name.startsWith("--") ? serializeIdentifier(record.name) : record.name}: ${safe ? record.safeValue : record.observableValue}${record.important ? " !important" : ""};`,
      );
    }

    return declarations.join(separator);
  }
}

export class CSSStyleRule extends CSSRule {
  readonly style: CSSStyleDeclaration;
  #selectorText: string;

  constructor(
    selectorText: string,
    reportDiagnostic: ReportDiagnostic = ignoreDiagnostic,
  ) {
    super(CSSRule.STYLE_RULE);
    this.#selectorText = normalizeSelectorText(`${selectorText}`) ?? `${selectorText}`;
    this.style = new CSSStyleDeclaration(this, reportDiagnostic);
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
    return declarations === ""
      ? `${this.selectorText} { }`
      : `${this.selectorText} { ${declarations} }`;
  }

  serializeSafe(): string {
    const declarations = this.style.serializeSafe("  ");
    return declarations === ""
      ? `${this.selectorText} {\n}\n`
      : `${this.selectorText} {\n${declarations}\n}\n`;
  }
}

export class CSSRuleList {
  readonly [index: number]: CSSRule | undefined;

  readonly #rules: CSSRule[];

  constructor(rules: CSSRule[]) {
    this.#rules = rules;

    return new Proxy(this, {
      get(target, property) {
        if (typeof property !== "string" || !arrayIndexPattern.test(property)) {
          const result = Reflect.get(target, property, target);
          return typeof result === "function" ? result.bind(target) : result;
        }

        return target.#rules[Number(property)];
      },
    });
  }

  get length(): number {
    return this.#rules.length;
  }

  item(index: number): CSSRule | null {
    return this.#rules[toUnsignedLong(index)] ?? null;
  }
}

function createStyleRule(
  node: csstree.Rule,
  reportDiagnostic: ReportDiagnostic,
): CSSStyleRule {
  const rule = new CSSStyleRule(csstree.generate(node.prelude), reportDiagnostic);
  const blockText = csstree.generate(node.block);
  rule.style.cssText = blockText.slice(1, -1);
  return rule;
}

function parseStrictRule(
  ruleText: string,
  reportDiagnostic: ReportDiagnostic,
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
  if (node.type === "Rule") return createStyleRule(node, reportDiagnostic);
  if (node.type !== "Atrule" || node.name.toLowerCase() === "import") return null;

  return new CSSGenericRule(genericRuleType(node.name), csstree.generate(node));
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
  let parsed: csstree.CssNode;
  try {
    parsed = csstree.parse(cssText);
  } catch {
    return [];
  }
  if (parsed.type !== "StyleSheet") return [];

  const rules: CSSRule[] = [];
  for (const node of parsed.children) {
    if (node.type === "Rule") {
      rules.push(createStyleRule(node, reportDiagnostic));
      continue;
    }
    if (node.type !== "Atrule") continue;
    if (node.name.toLowerCase() === "import" && !preserveImports) continue;

    rules.push(new CSSGenericRule(genericRuleType(node.name), csstree.generate(node)));
  }

  return rules;
}

export class CSSStyleSheet {
  readonly cssRules: CSSRuleList;
  readonly media: string;
  disabled: boolean;

  readonly #rules: CSSRule[] = [];
  readonly #diagnostics: SheetOMDiagnostic[] | null;
  readonly #constructedBaseURL: string;

  constructor(options: CSSStyleSheetOptions = {}) {
    this.#diagnostics = options.diagnostics ? [] : null;
    this.#constructedBaseURL = options.baseURL ?? "about:blank";
    this.media = options.media ?? "";
    this.disabled = options.disabled ?? false;
    this.cssRules = new CSSRuleList(this.#rules);
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
    const normalizedIndex = toUnsignedLong(index);
    if (normalizedIndex > this.#rules.length) {
      throw new DOMException("The index is outside the allowed range.", "IndexSizeError");
    }

    const rule = parseStrictRule(`${ruleText}`, this.#reportDiagnostic);
    if (!rule) throw new DOMException("The rule could not be parsed.", "SyntaxError");

    rule.parentStyleSheet = this;
    this.#rules.splice(normalizedIndex, 0, rule);
    return normalizedIndex;
  }

  deleteRule(index: number): void {
    const normalizedIndex = toUnsignedLong(index);
    if (normalizedIndex >= this.#rules.length) {
      throw new DOMException("The index is outside the allowed range.", "IndexSizeError");
    }

    const [removed] = this.#rules.splice(normalizedIndex, 1);
    if (!removed) return;
    removed.parentRule = null;
    removed.parentStyleSheet = null;
  }

  replaceSync(cssText: string): void {
    const replacement = parseStyleSheetRules(
      `${cssText}`,
      this.#reportDiagnostic,
      regularSheetMetadata.has(this),
    );

    for (const rule of this.#rules) {
      rule.parentRule = null;
      rule.parentStyleSheet = null;
    }
    for (const rule of replacement) rule.parentStyleSheet = this;

    this.#rules.splice(0, this.#rules.length, ...replacement);
  }

  async replace(cssText: string): Promise<CSSStyleSheet> {
    this.replaceSync(cssText);
    return this;
  }

  serialize(): string {
    return this.#rules
      .map(rule =>
        rule instanceof CSSStyleRule ? rule.serializeSafe() : `${rule.cssText}\n`,
      )
      .join("");
  }

  takeDiagnostics(): SheetOMDiagnostic[] {
    if (!this.#diagnostics) return [];
    return this.#diagnostics.splice(0);
  }
}

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
