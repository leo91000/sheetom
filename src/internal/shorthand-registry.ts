import { CSSStyleDeclaration as CSSStyleDeclarationOracle } from "cssstyle";
import * as csstree from "css-tree";
import {
  transformStyleAttribute,
  type ReturnedDeclaration,
} from "lightningcss";

import { chromiumShorthandLonghands } from "../chromium-properties.js";
import type {
  DeclarationRecord,
  ParsedPropertyValue,
} from "./declaration-block.js";

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
const twoValueShorthandNames = new Set([
  "gap",
  "grid-gap",
  "inset-block",
  "inset-inline",
  "margin-block",
  "margin-inline",
  "overscroll-behavior",
  "padding-block",
  "padding-inline",
  "scroll-margin-block",
  "scroll-margin-inline",
  "scroll-padding-block",
  "scroll-padding-inline",
]);
const slashPairShorthandNames = new Set(["grid-column", "grid-row"]);
const encoder = new TextEncoder();
const decoder = new TextDecoder();
const cssWideKeywords = new Set(["initial", "inherit", "unset", "revert", "revert-layer"]);
const shorthandResidualDefaults: Readonly<Record<string, Readonly<Record<string, string>>>> = {
  border: {
    "border-image-source": "none",
    "border-image-slice": "100%",
    "border-image-width": "1",
    "border-image-outset": "0",
    "border-image-repeat": "stretch",
  },
  font: {
    "font-variant-ligatures": "normal",
    "font-variant-numeric": "normal",
    "font-variant-east-asian": "normal",
    "font-variant-alternates": "normal",
    "font-size-adjust": "none",
    "font-language-override": "normal",
    "font-kerning": "auto",
    "font-optical-sizing": "auto",
    "font-feature-settings": "normal",
    "font-variation-settings": "normal",
    "font-variant-position": "normal",
    "font-variant-emoji": "normal",
  },
  animation: {
    "animation-timeline": "auto",
    "animation-range-start": "normal",
    "animation-range-end": "normal",
  },
  transition: {
    "transition-behavior": "normal",
  },
};
const staticShorthandNames = Object.entries(chromiumShorthandLonghands)
  .filter(([, longhands]) => longhands.length > 1)
  .map(([name]) => name);

function isAllowedResidual(
  shorthand: string,
  property: string,
  records: readonly DeclarationRecord[],
  safe: boolean,
): boolean {
  const defaults = shorthandResidualDefaults[shorthand];
  if (!defaults) return false;
  const covered = getShorthandLonghands(property) ?? [property];
  for (const longhand of covered) {
    const expected = defaults[longhand];
    const record = records.find(candidate => candidate.name === longhand);
    if (!record || expected === undefined) return false;
    const value = safe ? record.safeValue : record.observableValue;
    if (value !== expected) return false;
  }
  return true;
}

export function isFourSideShorthand(name: string): boolean {
  return fourSideShorthandNames.has(name);
}

export function getShorthandLonghands(name: string): readonly string[] | null {
  if (!Object.hasOwn(chromiumShorthandLonghands, name)) return null;
  return chromiumShorthandLonghands[name] ?? null;
}

export function getStaticShorthandNames(): readonly string[] {
  return staticShorthandNames;
}

function canonicalShorthandName(name: string, longhands: readonly string[]): string {
  if (!name.startsWith("-webkit-")) return name;
  const unprefixed = name.slice("-webkit-".length);
  const unprefixedLonghands = getShorthandLonghands(unprefixed);
  if (
    unprefixedLonghands?.length === longhands.length &&
    unprefixedLonghands.every((longhand, index) => longhand === longhands[index])
  ) {
    return unprefixed;
  }
  return name;
}

function recordValue(
  records: readonly DeclarationRecord[],
  name: string,
  safe: boolean,
): string | null {
  const record = records.find(candidate => candidate.name === name);
  if (!record) return null;
  return safe ? record.safeValue : record.observableValue;
}

function synthesizeAnimation(
  records: readonly DeclarationRecord[],
  safe: boolean,
): string | null {
  for (const [longhand, expected] of [
    ["animation-timeline", "auto"],
    ["animation-range-start", "normal"],
    ["animation-range-end", "normal"],
  ] as const) {
    if (recordValue(records, longhand, safe) !== expected) return null;
  }

  const fields = [
    "animation-duration",
    "animation-timing-function",
    "animation-delay",
    "animation-iteration-count",
    "animation-direction",
    "animation-fill-mode",
    "animation-play-state",
    "animation-name",
  ];
  const lists = fields.map(field => {
    const value = recordValue(records, field, safe);
    return value === null ? [] : splitTopLevelDelimiter(value, ",");
  });
  const length = lists[0]?.length ?? 0;
  if (length === 0 || lists.some(values => values.length !== length)) return null;
  const animations: string[] = [];
  for (let index = 0; index < length; index += 1) {
    const values = lists.map(list => list[index]);
    if (values.some(value => !value)) return null;
    animations.push(values.join(" "));
  }
  return animations.join(", ");
}

export function synthesizeStaticShorthand(
  name: string,
  records: readonly DeclarationRecord[],
  safe: boolean,
): string | null {
  const longhands = getShorthandLonghands(name);
  if (!longhands || records.length !== longhands.length) return null;
  if (records.some(record => record.pendingGroup !== null)) return null;
  const serializationName = canonicalShorthandName(name, longhands);
  const recordValues = records.map(record => safe ? record.safeValue : record.observableValue);
  if (
    recordValues.length > 0 &&
    recordValues.every(value => value === recordValues[0]) &&
    cssWideKeywords.has(recordValues[0] ?? "")
  ) {
    return recordValues[0] ?? null;
  }

  if (serializationName === "animation") {
    return synthesizeAnimation(records, safe);
  }

  if (serializationName === "background") {
    const color = recordValue(records, "background-color", safe);
    const resetLonghands = longhands.filter(longhand => longhand !== "background-color");
    if (
      color !== null &&
      resetLonghands.every(longhand => recordValue(records, longhand, safe) === "initial")
    ) {
      return color;
    }
  }

  if (
    serializationName === "font-variant" &&
    recordValues.every(value => value === "normal")
  ) {
    return "normal";
  }

  if (name === "white-space") {
    const values = recordValues;
    const serialized = new Map([
      ["collapse\0wrap", "normal"],
      ["preserve\0nowrap", "pre"],
      ["collapse\0nowrap", "nowrap"],
      ["preserve\0wrap", "pre-wrap"],
      ["preserve-breaks\0wrap", "pre-line"],
      ["break-spaces\0wrap", "break-spaces"],
    ]).get(values.join("\0"));
    return serialized ?? values.join(" ");
  }

  const source = records.map(record => {
    const value = safe ? record.safeValue : record.observableValue;
    return `${record.name}: ${value}${record.important ? " !important" : ""}`;
  }).join(";");

  try {
    const result = transformStyleAttribute({ code: encoder.encode(source) });
    const serialized = decoder.decode(result.code);
    const declarations = csstree.parse(serialized, {
      context: "declarationList",
      positions: true,
    });
    if (declarations.type !== "DeclarationList") return null;

    const longhandSet = new Set(longhands);
    for (const declaration of declarations.children) {
      if (declaration.type !== "Declaration") continue;
      const property = csstree.ident.decode(declaration.property);
      if (property === serializationName) continue;
      const nestedLonghands = getShorthandLonghands(property);
      if (
        longhandSet.has(property) ||
        nestedLonghands?.some(longhand => longhandSet.has(longhand))
      ) {
        if (!isAllowedResidual(serializationName, property, records, safe)) return null;
      }
    }

    for (const declaration of declarations.children) {
      if (declaration.type !== "Declaration") continue;
      if (csstree.ident.decode(declaration.property) !== serializationName) continue;
      if (declaration.important !== records[0]?.important) return null;
      const location = declaration.value.loc;
      return (location
        ? serialized.slice(location.start.offset, location.end.offset)
        : csstree.generate(declaration.value)).trim();
    }
  } catch {
    return null;
  }
  return null;
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

function splitTopLevelDelimiter(value: string, delimiter: string): string[] {
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
    if (character === "(" || character === "[" || character === "{") depth += 1;
    if (character === ")" || character === "]" || character === "}") depth -= 1;
    if (character === delimiter && depth === 0) {
      components.push(current.trim());
      current = "";
      continue;
    }
    current += character;
  }
  components.push(current.trim());
  return components;
}

function extractSerializedValue(serialized: string, property: string): string | null {
  const declarations = csstree.parse(serialized, {
    context: "declarationList",
    positions: true,
  });
  if (declarations.type !== "DeclarationList") return null;
  for (const declaration of declarations.children) {
    if (declaration.type !== "Declaration") continue;
    if (csstree.ident.decode(declaration.property) !== property) continue;
    const location = declaration.value.loc;
    return location
      ? serialized.slice(location.start.offset, location.end.offset)
      : csstree.generate(declaration.value);
  }
  return null;
}

function parseTypedDeclaration(name: string, value: string): Record<string, unknown> | null {
  let declaration: unknown;
  let count = 0;
  try {
    transformStyleAttribute({
      code: encoder.encode(`${name}: ${value}`),
      visitor: {
        Declaration(candidate) {
          count += 1;
          declaration = candidate;
        },
      },
    });
  } catch {
    return null;
  }
  if (count !== 1 || typeof declaration !== "object" || declaration === null) return null;
  return declaration as Record<string, unknown>;
}

function serializeTypedLonghand(property: string, value: unknown): string | null {
  try {
    const result = transformStyleAttribute({
      code: encoder.encode("color: black"),
      visitor: {
        Declaration() {
          return { property, value } as ReturnedDeclaration;
        },
      },
    });
    return extractSerializedValue(decoder.decode(result.code), property);
  } catch {
    return null;
  }
}

function expandAnimationValue(value: string): ReadonlyMap<string, string> | null {
  const declaration = parseTypedDeclaration("animation", value);
  if (declaration?.property !== "animation" || !Array.isArray(declaration.value)) return null;
  const animations = declaration.value;
  const field = (name: string): unknown[] | null => {
    const values: unknown[] = [];
    for (const animation of animations) {
      if (typeof animation !== "object" || animation === null) return null;
      if (!Object.hasOwn(animation, name)) return null;
      values.push((animation as Record<string, unknown>)[name]);
    }
    return values;
  };
  const definitions = [
    ["animation-duration", "duration"],
    ["animation-timing-function", "timingFunction"],
    ["animation-delay", "delay"],
    ["animation-iteration-count", "iterationCount"],
    ["animation-direction", "direction"],
    ["animation-fill-mode", "fillMode"],
    ["animation-play-state", "playState"],
    ["animation-name", "name"],
    ["animation-timeline", "timeline"],
  ] as const;
  const result = new Map<string, string>();
  for (const [property, fieldName] of definitions) {
    const values = field(fieldName);
    if (!values) return null;
    const serialized = serializeTypedLonghand(property, values);
    if (serialized === null) return null;
    result.set(property, serialized);
  }
  result.set("animation-range-start", "normal");
  result.set("animation-range-end", "normal");
  return result;
}

function expandTransitionValue(value: string): ReadonlyMap<string, string> | null {
  const declaration = parseTypedDeclaration("transition", value);
  if (declaration?.property !== "transition" || !Array.isArray(declaration.value)) return null;
  const transitions = declaration.value;
  const field = (name: string): unknown[] | null => {
    const values: unknown[] = [];
    for (const transition of transitions) {
      if (typeof transition !== "object" || transition === null) return null;
      if (!Object.hasOwn(transition, name)) return null;
      values.push((transition as Record<string, unknown>)[name]);
    }
    return values;
  };
  const result = new Map<string, string>();
  result.set("transition-behavior", "normal");
  for (const [property, fieldName] of [
    ["transition-duration", "duration"],
    ["transition-timing-function", "timingFunction"],
    ["transition-delay", "delay"],
    ["transition-property", "property"],
  ] as const) {
    const values = field(fieldName);
    if (!values) return null;
    const serialized = serializeTypedLonghand(property, values);
    if (serialized === null) return null;
    result.set(property, serialized);
  }
  return result;
}

function expandBorderRadiusValue(value: string): ReadonlyMap<string, string> | null {
  const axes = splitTopLevelDelimiter(value, "/");
  if (axes.length > 2) return null;
  const horizontal = expandFourSides(axes[0] ?? "");
  const vertical = expandFourSides(axes[1] ?? axes[0] ?? "");
  if (!horizontal || !vertical) return null;
  const names = [
    "border-top-left-radius",
    "border-top-right-radius",
    "border-bottom-right-radius",
    "border-bottom-left-radius",
  ];
  const result = new Map<string, string>();
  for (let index = 0; index < names.length; index += 1) {
    const name = names[index];
    const x = horizontal[index];
    const y = vertical[index];
    if (!name || !x || !y) return null;
    result.set(name, x === y ? x : `${x} ${y}`);
  }
  return result;
}

function expandWhiteSpaceValue(value: string): ReadonlyMap<string, string> | null {
  const normalized = value.trim().replace(/\s+/g, " ");
  const aliases: Readonly<Record<string, readonly [string, string]>> = {
    normal: ["collapse", "wrap"],
    pre: ["preserve", "nowrap"],
    nowrap: ["collapse", "nowrap"],
    "pre-wrap": ["preserve", "wrap"],
    "pre-line": ["preserve-breaks", "wrap"],
    "break-spaces": ["break-spaces", "wrap"],
  };
  const values = aliases[normalized] ?? splitTopLevelWhitespace(normalized);
  if (values.length !== 2 || !values[0] || !values[1]) return null;
  return new Map([
    ["white-space-collapse", values[0]],
    ["text-wrap-mode", values[1]],
  ]);
}

function expandContainerValue(value: string): ReadonlyMap<string, string> | null {
  const components = splitTopLevelDelimiter(value, "/");
  if (components.length > 2) return null;
  const name = components[0];
  if (!name) return null;
  return new Map([
    ["container-name", name],
    ["container-type", components[1] || "normal"],
  ]);
}

function expandOverflowValue(value: string): ReadonlyMap<string, string> | null {
  const components = splitTopLevelWhitespace(value);
  if (components.length < 1 || components.length > 2 || !components[0]) return null;
  return new Map([
    ["overflow-x", components[0]],
    ["overflow-y", components[1] ?? components[0]],
  ]);
}

function expandFontValue(value: string): ReadonlyMap<string, string> | null {
  const style = new CSSStyleDeclarationOracle();
  style.setProperty("font", value);
  const read = (name: string, fallback: string): string =>
    style.getPropertyValue(name) || fallback;
  const variant = read("font-variant", "normal");
  const result = new Map<string, string>([
    ["font-style", read("font-style", "normal")],
    ["font-variant-caps", variant],
    ["font-variant-ligatures", "normal"],
    ["font-variant-numeric", "normal"],
    ["font-variant-east-asian", "normal"],
    ["font-variant-alternates", "normal"],
    ["font-size-adjust", "none"],
    ["font-language-override", "normal"],
    ["font-kerning", "auto"],
    ["font-optical-sizing", "auto"],
    ["font-feature-settings", "normal"],
    ["font-variation-settings", "normal"],
    ["font-variant-position", "normal"],
    ["font-variant-emoji", "normal"],
    ["font-weight", read("font-weight", "normal")],
    ["font-stretch", "normal"],
    ["font-size", read("font-size", "")],
    ["line-height", read("line-height", "normal")],
    ["font-family", read("font-family", "")],
  ]);
  if (result.get("font-size") === "" || result.get("font-family") === "") return null;
  return result;
}

function expandBackgroundValue(value: string): ReadonlyMap<string, string> | null {
  try {
    if (csstree.lexer.matchType("color", value).error !== null) return null;
  } catch {
    return null;
  }
  return new Map([
    ["background-image", "initial"],
    ["background-position-x", "initial"],
    ["background-position-y", "initial"],
    ["background-size", "initial"],
    ["background-repeat", "initial"],
    ["background-attachment", "initial"],
    ["background-origin", "initial"],
    ["background-clip", "initial"],
    ["background-color", value.trim()],
  ]);
}

const typedObjectShorthandFields: Readonly<
  Record<string, Readonly<Record<string, string>>>
> = {
  "list-style": {
    "list-style-position": "position",
    "list-style-image": "image",
    "list-style-type": "listStyleType",
  },
  outline: {
    "outline-color": "color",
    "outline-style": "style",
    "outline-width": "width",
  },
  "text-decoration": {
    "text-decoration-line": "line",
    "text-decoration-thickness": "thickness",
    "text-decoration-style": "style",
    "text-decoration-color": "color",
  },
};

function expandTypedObjectValue(name: string, value: string): ReadonlyMap<string, string> | null {
  const fields = typedObjectShorthandFields[name];
  if (!fields) return null;
  const declaration = parseTypedDeclaration(name, value);
  if (declaration?.property !== name) return null;
  const shorthandValue = declaration.value;
  if (typeof shorthandValue !== "object" || shorthandValue === null) return null;
  const valueRecord = shorthandValue as Record<string, unknown>;
  const result = new Map<string, string>();
  for (const [longhand, field] of Object.entries(fields)) {
    if (!Object.hasOwn(valueRecord, field)) return null;
    const serialized = serializeTypedLonghand(longhand, valueRecord[field]);
    if (serialized === null) return null;
    result.set(longhand, serialized);
  }
  return result;
}

function expandHighRiskValue(name: string, value: string): ReadonlyMap<string, string> | null {
  switch (name) {
    case "background": return expandBackgroundValue(value);
    case "overflow": return expandOverflowValue(value);
    case "border-radius": return expandBorderRadiusValue(value);
    case "font": return expandFontValue(value);
    case "animation": return expandAnimationValue(value);
    case "transition": return expandTransitionValue(value);
    case "container": return expandContainerValue(value);
    case "white-space": return expandWhiteSpaceValue(value);
    default: return expandTypedObjectValue(name, value);
  }
}

function expandHighRiskShorthand(
  name: string,
  parsed: ParsedPropertyValue,
  important: boolean,
): DeclarationRecord[] | null {
  const observable = expandHighRiskValue(name, parsed.observableValue);
  const safe = expandHighRiskValue(name, parsed.safeValue);
  if (!observable || !safe) return null;
  if ([...observable.keys()].join("\0") !== [...safe.keys()].join("\0")) return null;
  return [...observable].map(([longhand, observableValue]) => ({
    name: longhand,
    observableValue,
    safeValue: safe.get(longhand) ?? observableValue,
    pendingSubstitution: false,
    important,
    pendingGroup: null,
  }));
}

function expandTwoValueShorthand(
  name: string,
  parsed: ParsedPropertyValue,
  important: boolean,
): DeclarationRecord[] | null {
  if (!twoValueShorthandNames.has(name)) return null;
  const longhands = getShorthandLonghands(name);
  if (!longhands || longhands.length !== 2) return null;
  const observable = splitTopLevelWhitespace(parsed.observableValue);
  const safe = splitTopLevelWhitespace(parsed.safeValue);
  if (
    observable.length < 1 || observable.length > 2 ||
    safe.length < 1 || safe.length > 2 ||
    !observable[0] || !safe[0]
  ) {
    return null;
  }
  return longhands.map((longhand, index) => ({
    name: longhand,
    observableValue: observable[index] ?? observable[0] ?? "",
    safeValue: safe[index] ?? safe[0] ?? "",
    pendingSubstitution: false,
    important,
    pendingGroup: null,
  }));
}

function expandSlashPairShorthand(
  name: string,
  parsed: ParsedPropertyValue,
  important: boolean,
): DeclarationRecord[] | null {
  if (!slashPairShorthandNames.has(name)) return null;
  const longhands = getShorthandLonghands(name);
  if (!longhands || longhands.length !== 2) return null;
  const observable = splitTopLevelDelimiter(parsed.observableValue, "/");
  const safe = splitTopLevelDelimiter(parsed.safeValue, "/");
  if (
    observable.length < 1 || observable.length > 2 ||
    safe.length < 1 || safe.length > 2 ||
    !observable[0] || !safe[0]
  ) {
    return null;
  }
  return longhands.map((longhand, index) => ({
    name: longhand,
    observableValue: observable[index] || (index === 1 ? "auto" : observable[0] ?? ""),
    safeValue: safe[index] || (index === 1 ? "auto" : safe[0] ?? ""),
    pendingSubstitution: false,
    important,
    pendingGroup: null,
  }));
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

export function expandStaticFourSide(
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
export function expandStaticShorthand(
  name: string,
  parsed: ParsedPropertyValue,
  important: boolean,
): DeclarationRecord[] | null {
  const longhands = getShorthandLonghands(name);
  if (!longhands || longhands.length === 0 || parsed.pendingSubstitution) return null;

  if (
    cssWideKeywords.has(parsed.observableValue) &&
    cssWideKeywords.has(parsed.safeValue)
  ) {
    return longhands.map(longhand => ({
      name: longhand,
      observableValue: parsed.observableValue,
      safeValue: parsed.safeValue,
      pendingSubstitution: false,
      important,
      pendingGroup: null,
    }));
  }

  const highRiskExpansion = expandHighRiskShorthand(name, parsed, important);
  if (highRiskExpansion) return highRiskExpansion;
  const twoValueExpansion = expandTwoValueShorthand(name, parsed, important);
  if (twoValueExpansion) return twoValueExpansion;
  const slashPairExpansion = expandSlashPairShorthand(name, parsed, important);
  if (slashPairExpansion) return slashPairExpansion;

  const observableStyle = new CSSStyleDeclarationOracle();
  const safeStyle = new CSSStyleDeclarationOracle();
  observableStyle.setProperty(name, parsed.observableValue);
  safeStyle.setProperty(name, parsed.safeValue);

  const fallbackValues: Readonly<Record<string, string>> = name === "border"
    ? {
        "border-image-source": "none",
        "border-image-slice": "100%",
        "border-image-width": "1",
        "border-image-outset": "0",
        "border-image-repeat": "stretch",
      }
    : name === "background"
      ? {
          "background-position-x": "0%",
          "background-position-y": "0%",
        }
    : {};
  const orderedLonghands = name === "border"
    ? [
        "border-top-width",
        "border-right-width",
        "border-bottom-width",
        "border-left-width",
        "border-top-style",
        "border-right-style",
        "border-bottom-style",
        "border-left-style",
        "border-top-color",
        "border-right-color",
        "border-bottom-color",
        "border-left-color",
        "border-image-source",
        "border-image-slice",
        "border-image-width",
        "border-image-outset",
        "border-image-repeat",
      ]
    : longhands;

  const records: DeclarationRecord[] = [];
  for (const longhand of orderedLonghands) {
    if (!longhands.includes(longhand)) return null;
    const observableValue = observableStyle.getPropertyValue(longhand) || fallbackValues[longhand] || "";
    const safeValue = safeStyle.getPropertyValue(longhand) || fallbackValues[longhand] || "";
    if (observableValue === "" || safeValue === "") return null;
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
