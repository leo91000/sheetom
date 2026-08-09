import { CSSStyleDeclaration as CSSStyleDeclarationOracle } from "cssstyle";
import * as csstree from "css-tree";
import {
  transformStyleAttribute,
  type ReturnedDeclaration,
} from "lightningcss";

import { chromiumShorthandLonghands } from "../chromium-properties.js";
import type {
  AcceptedPropertyValue,
  DeclarationRecord,
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
const repeatedPairShorthandNames = new Set([
  "animation-range",
  "background-position",
  "border-spacing",
  "contain-intrinsic-size",
  "interest-delay",
  "mask-position",
  "-webkit-mask-position",
  "place-content",
  "place-items",
  "place-self",
  "timeline-trigger-activation-range",
  "timeline-trigger-active-range",
]);
const borderLikeShorthandNames = new Set([
  "border",
  "border-block",
  "border-block-end",
  "border-block-start",
  "border-bottom",
  "border-inline",
  "border-inline-end",
  "border-inline-start",
  "border-left",
  "border-right",
  "border-top",
  "column-rule",
  "row-rule",
  "rule",
  "-webkit-border-after",
  "-webkit-border-before",
  "-webkit-border-end",
  "-webkit-border-start",
  "-webkit-column-rule",
]);
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

export type StaticShorthandCodecId =
  | "four-side"
  | "two-value"
  | "slash-pair"
  | "animation"
  | "background"
  | "border"
  | "border-image"
  | "border-radius"
  | "columns"
  | "container"
  | "flex-flow"
  | "font"
  | "grid"
  | "layered"
  | "offset"
  | "overflow"
  | "repeated-pair"
  | "scroll-timeline"
  | "text-wrap"
  | "transition"
  | "typed-object"
  | "white-space"
  | "legacy-cssstyle";

export interface StaticShorthandDefinition {
  name: string;
  longhands: readonly string[];
  codec: StaticShorthandCodecId;
}

const namedCodecIds: Readonly<Record<string, StaticShorthandCodecId>> = {
  animation: "animation",
  "-webkit-animation": "animation",
  background: "background",
  mask: "layered",
  "-webkit-mask": "layered",
  "border-image": "border-image",
  "-webkit-mask-box-image": "border-image",
  offset: "offset",
  "border-radius": "border-radius",
  "-webkit-border-radius": "border-radius",
  container: "container",
  font: "font",
  overflow: "overflow",
  transition: "transition",
  "-webkit-transition": "transition",
  "list-style": "typed-object",
  outline: "typed-object",
  "text-decoration": "typed-object",
  "white-space": "white-space",
};

function codecIdFor(name: string): StaticShorthandCodecId {
  if (fourSideShorthandNames.has(name)) return "four-side";
  if (twoValueShorthandNames.has(name)) return "two-value";
  if (slashPairShorthandNames.has(name)) return "slash-pair";
  if (borderLikeShorthandNames.has(name)) return "border";
  if (repeatedPairShorthandNames.has(name)) return "repeated-pair";
  if (["grid-area", "grid-template"].includes(name)) return "grid";
  if (name === "columns") return "columns";
  if (name === "text-wrap") return "text-wrap";
  if (name === "scroll-timeline") return "scroll-timeline";
  if (name === "flex-flow") return "flex-flow";
  return namedCodecIds[name] ?? "legacy-cssstyle";
}

const staticShorthandDefinitions = staticShorthandNames.map(name => ({
  name,
  longhands: chromiumShorthandLonghands[name] ?? [],
  codec: codecIdFor(name),
}));

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

export function getStaticShorthandDefinitions(): readonly StaticShorthandDefinition[] {
  return staticShorthandDefinitions;
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

function uniformSuffixValue(
  records: readonly DeclarationRecord[],
  suffix: string,
  safe: boolean,
): string | null {
  const values = records
    .filter(record => record.name.endsWith(suffix))
    .map(record => safe ? record.safeValue : record.observableValue);
  if (values.length === 0 || values.some(value => value !== values[0])) return null;
  return values[0] ?? null;
}

function synthesizeBorderLike(
  name: string,
  records: readonly DeclarationRecord[],
  safe: boolean,
): string | null {
  if (!borderLikeShorthandNames.has(name)) return null;
  const componentRecords = name === "border"
    ? records.filter(record => !record.name.startsWith("border-image-"))
    : records;
  if (name === "border") {
    for (const residual of [
      "border-image-source",
      "border-image-slice",
      "border-image-width",
      "border-image-outset",
      "border-image-repeat",
    ]) {
      if (!isAllowedResidual("border", residual, records, safe)) return null;
    }
  }
  const width = uniformSuffixValue(componentRecords, "-width", safe);
  const style = uniformSuffixValue(componentRecords, "-style", safe);
  const color = uniformSuffixValue(componentRecords, "-color", safe);
  if (width === null || style === null || color === null) return null;
  const components = [width];
  if (style !== "none") components.push(style);
  if (color !== "currentcolor") components.push(color);
  return components.join(" ");
}

function parallelLonghandLists(
  records: readonly DeclarationRecord[],
  properties: readonly string[],
  safe: boolean,
): string[][] | null {
  const lists = properties.map(property => {
    const value = recordValue(records, property, safe);
    return value === null ? [] : splitTopLevelDelimiter(value, ",");
  });
  const length = lists[0]?.length ?? 0;
  if (length === 0 || lists.some(list => list.length !== length)) return null;
  return lists;
}

function synthesizeBackground(
  records: readonly DeclarationRecord[],
  safe: boolean,
): string | null {
  const properties = [
    "background-image",
    "background-position-x",
    "background-position-y",
    "background-size",
    "background-repeat",
    "background-attachment",
    "background-origin",
    "background-clip",
  ] as const;
  const lists = parallelLonghandLists(records, properties, safe);
  const color = recordValue(records, "background-color", safe);
  if (!lists || color === null) return null;
  const layers: string[] = [];
  for (let index = 0; index < (lists[0]?.length ?? 0); index += 1) {
    const values = lists.map(list => list[index] ?? "initial");
    const image = values[0] ?? "initial";
    const x = values[1] ?? "initial";
    const y = values[2] ?? "initial";
    const size = values[3] ?? "initial";
    const repeat = values[4] ?? "initial";
    const attachment = values[5] ?? "initial";
    const origin = values[6] ?? "initial";
    const clip = values[7] ?? "initial";
    const components: string[] = [];
    if (image !== "initial") components.push(image);
    if (x !== "initial" || y !== "initial" || size !== "initial") {
      if (x === "initial" || y === "initial") return null;
      components.push(`${x} ${y}`);
      if (size !== "initial") components.push(`/ ${size}`);
    }
    if (repeat !== "initial") components.push(repeat);
    if (attachment !== "initial") components.push(attachment);
    if (origin !== "initial" || clip !== "initial") {
      if (origin === "initial" || clip === "initial") return null;
      components.push(origin);
      if (clip !== origin) components.push(clip);
    }
    if (index === (lists[0]?.length ?? 0) - 1 && color !== "initial") {
      components.push(color);
    }
    if (components.length === 0) return null;
    layers.push(components.join(" "));
  }
  return layers.join(", ");
}

function synthesizeMask(
  records: readonly DeclarationRecord[],
  safe: boolean,
): string | null {
  const properties = [
    "mask-image",
    "-webkit-mask-position-x",
    "-webkit-mask-position-y",
    "mask-size",
    "mask-repeat",
    "mask-origin",
    "mask-clip",
    "mask-composite",
    "mask-mode",
  ] as const;
  const lists = parallelLonghandLists(records, properties, safe);
  if (!lists) return null;
  const layers: string[] = [];
  for (let index = 0; index < (lists[0]?.length ?? 0); index += 1) {
    const [image, x, y, size, repeat, origin, clip, composite, mode] =
      lists.map(list => list[index] ?? "");
    if (!image || !x || !y || !size || !repeat || !origin || !clip || !composite || !mode) {
      return null;
    }
    const components = [image];
    if (x !== "0%" || y !== "0%" || size !== "auto") {
      components.push(`${x} ${y}`);
      if (size !== "auto") components.push(`/ ${size}`);
    }
    if (repeat !== "repeat") components.push(repeat);
    if (origin !== "border-box" || clip !== "border-box") {
      components.push(origin);
      if (clip !== origin) components.push(clip);
    }
    if (composite !== "add") components.push(composite);
    if (mode !== "match-source") components.push(mode);
    layers.push(components.join(" "));
  }
  return layers.join(", ");
}

function synthesizeBorderImage(
  name: string,
  records: readonly DeclarationRecord[],
  safe: boolean,
): string | null {
  if (name === "-webkit-mask-box-image") return null;
  if (name !== "border-image") return null;
  const source = recordValue(records, "border-image-source", safe);
  const slice = recordValue(records, "border-image-slice", safe);
  const width = recordValue(records, "border-image-width", safe);
  const outset = recordValue(records, "border-image-outset", safe);
  const repeat = recordValue(records, "border-image-repeat", safe);
  if (!source || !slice || !width || !outset || !repeat) return null;
  if (
    source === "none" && slice === "100%" && width === "1" &&
    outset === "0" && repeat === "stretch"
  ) return "none";
  return `${source} ${slice} / ${width} / ${outset} ${repeat}`;
}

function synthesizeStructuralShorthand(
  name: string,
  records: readonly DeclarationRecord[],
  safe: boolean,
): string | null {
  const border = synthesizeBorderLike(name, records, safe);
  if (border) return border;
  const value = (longhand: string): string | null => recordValue(records, longhand, safe);

  if (name === "grid-area") {
    const values = [
      value("grid-row-start"),
      value("grid-column-start"),
      value("grid-row-end"),
      value("grid-column-end"),
    ];
    if (values.some(component => component === null)) return null;
    if (values.every(component => component === "auto")) return "auto";
    return values.join(" / ");
  }
  if (name === "grid-template") {
    const rows = value("grid-template-rows");
    const columns = value("grid-template-columns");
    const areas = value("grid-template-areas");
    if (rows === null || columns === null || areas === null) return null;
    if (rows === "none" && columns === "none") return "none";
    if (areas !== "none") {
      const areaRows = splitTopLevelWhitespace(areas);
      const rowSizes = splitTopLevelWhitespace(rows);
      if (areaRows.length !== rowSizes.length) return null;
      return `${areaRows.map((area, index) => `${area} ${rowSizes[index]}`).join(" ")} / ${columns}`;
    }
    return `${rows} / ${columns}`;
  }
  if (name === "columns") {
    const width = value("column-width");
    const count = value("column-count");
    if (
      width === null || count === null ||
      value("column-height") !== "auto" || value("column-wrap") !== "auto"
    ) return null;
    if (width === "auto" && count === "auto") return "auto";
    if (width === "auto") return count;
    if (count === "auto") return width;
    return `${width} ${count}`;
  }
  if (name === "text-wrap") {
    const mode = value("text-wrap-mode");
    const style = value("text-wrap-style");
    if (mode === null || style === null) return null;
    if (style === "initial") return mode;
    if (mode === "wrap") return style;
    return `${mode} ${style}`;
  }
  if (name === "scroll-timeline") {
    const timelineName = value("scroll-timeline-name");
    const axis = value("scroll-timeline-axis");
    if (timelineName === null || axis === null) return null;
    return axis === "block" ? timelineName : `${timelineName} ${axis}`;
  }
  if (name === "offset") {
    const position = value("offset-position");
    const path = value("offset-path");
    const distance = value("offset-distance");
    const rotate = value("offset-rotate");
    const anchor = value("offset-anchor");
    if (!position || !path || !distance || !rotate || !anchor) return null;
    if (
      position === "normal" && path === "none" && distance === "0px" &&
      rotate === "auto" && anchor === "auto"
    ) return "normal";
    const components: string[] = [];
    if (position !== "normal") components.push(position);
    if (path !== "none") components.push(path);
    if (distance !== "0px") components.push(distance);
    if (rotate !== "auto") components.push(rotate);
    if (anchor !== "auto") components.push(`/ ${anchor}`);
    return components.length > 0 ? components.join(" ") : null;
  }
  if (name === "flex-flow") {
    const direction = value("flex-direction");
    const wrap = value("flex-wrap");
    if (direction === null || wrap === null) return null;
    if (wrap === "nowrap") return direction;
    if (direction === "row") return wrap;
    return `${direction} ${wrap}`;
  }
  if (repeatedPairShorthandNames.has(name)) {
    const longhands = getShorthandLonghands(name);
    if (!longhands || longhands.length !== 2) return null;
    const first = value(longhands[0] ?? "");
    const second = value(longhands[1] ?? "");
    if (first === null || second === null) return null;
    if (["background-position", "mask-position", "-webkit-mask-position"].includes(name)) {
      return `${first} ${second}`;
    }
    return first === second ? first : `${first} ${second}`;
  }
  return null;
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
    const background = synthesizeBackground(records, safe);
    if (background !== null) return background;
  }

  if (serializationName === "mask") return synthesizeMask(records, safe);
  const borderImage = synthesizeBorderImage(serializationName, records, safe);
  if (borderImage !== null) return borderImage;
  if (safe && serializationName === "offset") return null;

  const structural = synthesizeStructuralShorthand(serializationName, records, safe);
  if (structural !== null) return structural;

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

function matchesPropertyGrammar(property: string, value: string): boolean {
  try {
    return csstree.lexer.matchProperty(property, value).error === null;
  } catch {
    return false;
  }
}

function borderComponentValues(
  value: string,
  longhands: readonly string[],
): Readonly<Record<"width" | "style" | "color", string>> | null {
  const widthProperty = longhands.find(longhand => longhand.endsWith("-width"));
  const styleProperty = longhands.find(longhand => longhand.endsWith("-style"));
  const colorProperty = longhands.find(longhand => longhand.endsWith("-color"));
  if (!widthProperty || !styleProperty || !colorProperty) return null;

  const result = { width: "medium", style: "none", color: "currentcolor" };
  const components = splitTopLevelWhitespace(value);
  if (components.length === 0 || components.length > 3) return null;
  for (const component of components) {
    if (matchesPropertyGrammar(widthProperty, component)) {
      result.width = component;
      continue;
    }
    if (matchesPropertyGrammar(styleProperty, component)) {
      result.style = component;
      continue;
    }
    if (matchesPropertyGrammar(colorProperty, component)) {
      result.color = component;
      continue;
    }
    return null;
  }
  return result;
}

function orderedBorderLonghands(longhands: readonly string[]): string[] {
  const ranked = longhands.map((longhand, index) => ({
    longhand,
    index,
    rank: longhand.endsWith("-width")
      ? 0
      : longhand.endsWith("-style")
        ? 1
        : longhand.endsWith("-color")
          ? 2
          : 3,
  }));
  ranked.sort((left, right) => left.rank - right.rank || left.index - right.index);
  return ranked.map(entry => entry.longhand);
}

function expandBorderLikeValue(
  name: string,
  value: string,
): ReadonlyMap<string, string> | null {
  if (!borderLikeShorthandNames.has(name)) return null;
  const longhands = getShorthandLonghands(name);
  if (!longhands) return null;
  const components = borderComponentValues(value, longhands);
  if (!components) return null;
  const result = new Map<string, string>();
  for (const longhand of orderedBorderLonghands(longhands)) {
    if (longhand === "border-image-source") result.set(longhand, "none");
    else if (longhand === "border-image-slice") result.set(longhand, "100%");
    else if (longhand === "border-image-width") result.set(longhand, "1");
    else if (longhand === "border-image-outset") result.set(longhand, "0");
    else if (longhand === "border-image-repeat") result.set(longhand, "stretch");
    else if (longhand.endsWith("-width")) result.set(longhand, components.width);
    else if (longhand.endsWith("-style")) result.set(longhand, components.style);
    else if (longhand.endsWith("-color")) result.set(longhand, components.color);
    else return null;
  }
  return result;
}

function expandRepeatedPairValue(
  name: string,
  value: string,
): ReadonlyMap<string, string> | null {
  if (!repeatedPairShorthandNames.has(name)) return null;
  const longhands = getShorthandLonghands(name);
  if (!longhands || longhands.length !== 2) return null;
  const components = splitTopLevelWhitespace(value);
  if (components.length < 1 || components.length > 2 || !components[0]) return null;
  return new Map([
    [longhands[0] ?? "", components[0]],
    [longhands[1] ?? "", components[1] ?? components[0]],
  ]);
}

function expandGridAreaValue(value: string): ReadonlyMap<string, string> | null {
  const components = splitTopLevelDelimiter(value, "/");
  if (components.length < 1 || components.length > 4 || !components[0]) return null;
  const [rowStart, columnStart = "auto", rowEnd = "auto", columnEnd = "auto"] = components;
  if (!rowStart) return null;
  return new Map([
    ["grid-row-start", rowStart],
    ["grid-column-start", columnStart],
    ["grid-row-end", rowEnd],
    ["grid-column-end", columnEnd],
  ]);
}

function expandGridTemplateValue(value: string): ReadonlyMap<string, string> | null {
  if (value === "none") {
    return new Map([
      ["grid-template-rows", "none"],
      ["grid-template-columns", "none"],
      ["grid-template-areas", "none"],
    ]);
  }
  const components = splitTopLevelDelimiter(value, "/");
  if (components.length !== 2 || !components[0] || !components[1]) return null;
  if (components[0].includes('"') || components[0].includes("'")) return null;
  return new Map([
    ["grid-template-rows", components[0]],
    ["grid-template-columns", components[1]],
    ["grid-template-areas", "none"],
  ]);
}

function expandColumnsValue(value: string): ReadonlyMap<string, string> | null {
  const components = splitTopLevelWhitespace(value);
  if (components.length < 1 || components.length > 2) return null;
  let width = "auto";
  let count = "auto";
  for (const component of components) {
    if (component === "auto") continue;
    if (matchesPropertyGrammar("column-width", component)) {
      width = component;
      continue;
    }
    if (matchesPropertyGrammar("column-count", component)) {
      count = component;
      continue;
    }
    return null;
  }
  return new Map([
    ["column-width", width],
    ["column-count", count],
    ["column-height", "auto"],
    ["column-wrap", "auto"],
  ]);
}

function expandTextWrapValue(value: string): ReadonlyMap<string, string> | null {
  const components = splitTopLevelWhitespace(value);
  if (components.length < 1 || components.length > 2) return null;
  const modeKeywords = new Set(["wrap", "nowrap"]);
  const mode = components.find(component => modeKeywords.has(component)) ?? "wrap";
  const style = components.find(component => !modeKeywords.has(component)) ?? "initial";
  return new Map([
    ["text-wrap-mode", mode],
    ["text-wrap-style", style],
  ]);
}

function expandScrollTimelineValue(value: string): ReadonlyMap<string, string> | null {
  const components = splitTopLevelWhitespace(value);
  if (components.length < 1 || components.length > 2 || !components[0]) return null;
  const axes = new Set(["block", "inline", "x", "y"]);
  const axis = components.find(component => axes.has(component)) ?? "block";
  const name = components.find(component => !axes.has(component)) ?? "none";
  return new Map([
    ["scroll-timeline-name", name],
    ["scroll-timeline-axis", axis],
  ]);
}

function expandFlexFlowValue(value: string): ReadonlyMap<string, string> | null {
  const components = splitTopLevelWhitespace(value);
  if (components.length < 1 || components.length > 2) return null;
  const directions = new Set(["row", "row-reverse", "column", "column-reverse"]);
  const wraps = new Set(["nowrap", "wrap", "wrap-reverse"]);
  const direction = components.find(component => directions.has(component)) ?? "row";
  const wrap = components.find(component => wraps.has(component)) ?? "nowrap";
  if (!components.every(component => directions.has(component) || wraps.has(component))) return null;
  return new Map([
    ["flex-direction", direction],
    ["flex-wrap", wrap],
  ]);
}

function canonicalPositionPair(value: string): string {
  const components = splitTopLevelWhitespace(value);
  if (components.length === 1 && components[0] !== "auto") {
    return `${components[0]} ${components[0]}`;
  }
  return value;
}

function expandOffsetValue(value: string): ReadonlyMap<string, string> | null {
  if (value === "normal") {
    return new Map([
      ["offset-position", "normal"],
      ["offset-path", "none"],
      ["offset-distance", "0px"],
      ["offset-rotate", "auto"],
      ["offset-anchor", "auto"],
    ]);
  }
  const slash = splitTopLevelDelimiter(value, "/");
  if (slash.length > 2 || !slash[0]) return null;
  const components = splitTopLevelWhitespace(slash[0]);
  let path = "none";
  let distance = "0px";
  let rotate = "auto";
  const position: string[] = [];
  const rotation: string[] = [];
  for (const component of components) {
    if (path === "none" && matchesPropertyGrammar("offset-path", component)) {
      path = component;
      continue;
    }
    if (distance === "0px" && matchesPropertyGrammar("offset-distance", component)) {
      distance = component;
      continue;
    }
    if (["auto", "reverse"].includes(component) || matchesPropertyGrammar("rotate", component)) {
      rotation.push(component);
      continue;
    }
    position.push(component);
  }
  if (rotation.length > 0) rotate = rotation.join(" ");
  return new Map([
    ["offset-position", position.length > 0 ? position.join(" ") : "normal"],
    ["offset-path", path],
    ["offset-distance", distance],
    ["offset-rotate", rotate],
    ["offset-anchor", slash[1] ? canonicalPositionPair(slash[1]) : "auto"],
  ]);
}

function expandStructuralValue(name: string, value: string): ReadonlyMap<string, string> | null {
  const border = expandBorderLikeValue(name, value);
  if (border) return border;
  const pair = expandRepeatedPairValue(name, value);
  if (pair) return pair;
  if (name === "grid-area") return expandGridAreaValue(value);
  if (name === "grid-template") return expandGridTemplateValue(value);
  if (name === "columns") return expandColumnsValue(value);
  if (name === "text-wrap") return expandTextWrapValue(value);
  if (name === "scroll-timeline") return expandScrollTimelineValue(value);
  if (name === "flex-flow") return expandFlexFlowValue(value);
  if (name === "offset") return expandOffsetValue(value);
  return null;
}

function expandMappedValues(
  name: string,
  parsed: AcceptedPropertyValue,
  important: boolean,
  expand: (name: string, value: string) => ReadonlyMap<string, string> | null,
): DeclarationRecord[] | null {
  const observable = expand(name, parsed.observableValue);
  const safe = expand(
    name,
    name === "offset" ? parsed.observableValue : parsed.safeValue,
  );
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

function acceptedTypedValue(
  parsed: AcceptedPropertyValue,
  properties: readonly string[],
): Record<string, unknown> | unknown[] | null {
  if (parsed.representation.kind !== "typed") return null;
  const declaration = parsed.representation.declaration;
  if (typeof declaration !== "object" || declaration === null) return null;
  const record = declaration as Record<string, unknown>;
  if (typeof record.property !== "string" || !properties.includes(record.property)) return null;
  const value = record.value;
  if (typeof value !== "object" || value === null) return null;
  return value as Record<string, unknown> | unknown[];
}

function serializeTypedFields(
  definitions: readonly (readonly [string, unknown, string?])[],
): ReadonlyMap<string, string> | null {
  const result = new Map<string, string>();
  for (const [property, value, serializationProperty = property] of definitions) {
    const serialized = serializeTypedLonghand(serializationProperty, value);
    if (serialized === null) return null;
    result.set(property, serialized);
  }
  return result;
}

interface BackgroundLayerPresence {
  image: boolean;
  position: boolean;
  size: boolean;
  repeat: boolean;
  attachment: boolean;
  boxes: number;
  color: boolean;
}

function backgroundLayerPresence(value: string): BackgroundLayerPresence[] {
  const layers = splitTopLevelDelimiter(value, ",");
  return layers.map((layer, index) => {
    const slash = splitTopLevelDelimiter(layer, "/");
    const beforeSize = slash[0] ?? "";
    const components = splitTopLevelWhitespace(beforeSize);
    const allComponents = [
      ...components,
      ...splitTopLevelWhitespace(slash[1] ?? ""),
    ];
    const repeatKeywords = new Set([
      "repeat", "no-repeat", "repeat-x", "repeat-y", "space", "round",
    ]);
    const attachmentKeywords = new Set(["scroll", "fixed", "local"]);
    const boxKeywords = new Set(["border-box", "padding-box", "content-box"]);
    const isLast = index === layers.length - 1;
    return {
      image: components.some(component => matchesPropertyGrammar("background-image", component)) ||
        /(?:^|\s)(?:url|(?:repeating-)?(?:linear|radial|conic)-gradient|image-set|cross-fade)\(/i
          .test(beforeSize) || components.includes("none"),
      position: components.some(component =>
        matchesPropertyGrammar("background-position-x", component) ||
        matchesPropertyGrammar("background-position-y", component),
      ),
      size: slash.length === 2 && (slash[1]?.trim() ?? "") !== "",
      repeat: allComponents.some(component => repeatKeywords.has(component)),
      attachment: allComponents.some(component => attachmentKeywords.has(component)),
      boxes: allComponents.filter(component => boxKeywords.has(component)).length,
      color: isLast && allComponents.some(component =>
        matchesPropertyGrammar("background-color", component),
      ),
    };
  });
}

function applyBackgroundOmissions(
  values: ReadonlyMap<string, string>,
  input: string,
  concreteDefaults: boolean,
): ReadonlyMap<string, string> | null {
  const presence = backgroundLayerPresence(input);
  const result = new Map(values);
  const initialValues: Readonly<Record<string, string>> = {
    "background-image": "none",
    "background-position-x": "0%",
    "background-position-y": "0%",
    "background-size": "auto",
    "background-repeat": "repeat",
    "background-attachment": "scroll",
    "background-origin": "padding-box",
    "background-clip": "border-box",
  };
  const flags: Readonly<Record<string, (layer: BackgroundLayerPresence) => boolean>> = {
    "background-image": layer => layer.image,
    "background-position-x": layer => layer.position,
    "background-position-y": layer => layer.position,
    "background-size": layer => layer.size,
    "background-repeat": layer => layer.repeat,
    "background-attachment": layer => layer.attachment,
    "background-origin": layer => layer.boxes > 0,
    "background-clip": layer => layer.boxes > 0,
  };
  for (const [property, isPresent] of Object.entries(flags)) {
    const serialized = result.get(property);
    if (serialized === undefined) return null;
    const components = splitTopLevelDelimiter(serialized, ",");
    if (components.length !== presence.length) return null;
    result.set(
      property,
      components.map((component, index) =>
        presence[index] && isPresent(presence[index])
          ? component
          : concreteDefaults ? initialValues[property] ?? component : "initial",
      ).join(", "),
    );
  }
  if (!presence[presence.length - 1]?.color) {
    result.set(
      "background-color",
      concreteDefaults ? "rgba(0, 0, 0, 0)" : "initial",
    );
  }
  return result;
}

function expandLayeredTypedValue(
  name: string,
  parsed: AcceptedPropertyValue,
  safe = false,
): ReadonlyMap<string, string> | null {
  if (name !== "background" && name !== "mask") return null;
  const value = acceptedTypedValue(parsed, [name]);
  if (!Array.isArray(value) || value.length === 0) return null;
  const layers: Record<string, unknown>[] = [];
  for (const layer of value) {
    if (typeof layer !== "object" || layer === null) return null;
    layers.push(layer as Record<string, unknown>);
  }
  const fields = (field: string): unknown[] | null => {
    const values: unknown[] = [];
    for (const layer of layers) {
      if (!Object.hasOwn(layer, field)) return null;
      values.push(layer[field]);
    }
    return values;
  };
  const positions = layers.map(layer => layer.position);
  if (positions.some(position => typeof position !== "object" || position === null)) {
    return null;
  }
  const positionRecords = positions as Record<string, unknown>[];
  const positionX = positionRecords.map(position => position.x);
  const positionY = positionRecords.map(position => position.y);
  if (positionX.some(position => position === undefined) || positionY.some(position => position === undefined)) {
    return null;
  }

  if (name === "background") {
    const lastLayer = layers[layers.length - 1];
    if (!lastLayer) return null;
    const image = fields("image");
    const size = fields("size");
    const repeat = fields("repeat");
    const attachment = fields("attachment");
    const origin = fields("origin");
    const clip = fields("clip");
    if (!image || !size || !repeat || !attachment || !origin || !clip) return null;
    const serialized = serializeTypedFields([
      ["background-image", image],
      ["background-position-x", positionX],
      ["background-position-y", positionY],
      ["background-size", size],
      ["background-repeat", repeat],
      ["background-attachment", attachment],
      ["background-origin", origin],
      ["background-clip", clip],
      ["background-color", lastLayer.color],
    ]);
    const source = safe ? parsed.safeValue : parsed.observableValue;
    return serialized ? applyBackgroundOmissions(serialized, source, safe) : null;
  }

  const image = fields("image");
  const size = fields("size");
  const repeat = fields("repeat");
  const origin = fields("origin");
  const clip = fields("clip");
  const composite = fields("composite");
  const mode = fields("mode");
  if (!image || !size || !repeat || !origin || !clip || !composite || !mode) return null;
  return serializeTypedFields([
    ["mask-image", image],
    ["-webkit-mask-position-x", positionX, "background-position-x"],
    ["-webkit-mask-position-y", positionY, "background-position-y"],
    ["mask-size", size],
    ["mask-repeat", repeat],
    ["mask-origin", origin],
    ["mask-clip", clip],
    ["mask-composite", composite],
    ["mask-mode", mode],
  ]);
}

function expandBorderImageTypedValue(
  name: string,
  parsed: AcceptedPropertyValue,
): ReadonlyMap<string, string> | null {
  const isBorderImage = name === "border-image";
  const isMaskBoxImage = name === "-webkit-mask-box-image";
  if (!isBorderImage && !isMaskBoxImage) return null;
  if (isMaskBoxImage && parsed.observableValue === "none") {
    return new Map([
      ["-webkit-mask-box-image-source", "none"],
      ["-webkit-mask-box-image-slice", "initial"],
      ["-webkit-mask-box-image-width", "initial"],
      ["-webkit-mask-box-image-outset", "initial"],
      ["-webkit-mask-box-image-repeat", "initial"],
    ]);
  }
  const value = acceptedTypedValue(parsed, isBorderImage ? ["border-image"] : ["mask-box-image"]);
  if (Array.isArray(value) || value === null) return null;
  const prefix = isBorderImage ? "border-image" : "-webkit-mask-box-image";
  for (const field of ["source", "slice", "width", "outset", "repeat"]) {
    if (!Object.hasOwn(value, field)) return null;
  }
  return serializeTypedFields([
    [`${prefix}-source`, value.source],
    [`${prefix}-slice`, value.slice],
    [`${prefix}-width`, value.width],
    [`${prefix}-outset`, value.outset],
    [`${prefix}-repeat`, value.repeat],
  ]);
}

function expandGridTemplateTypedValue(
  parsed: AcceptedPropertyValue,
): ReadonlyMap<string, string> | null {
  const value = acceptedTypedValue(parsed, ["grid-template"]);
  if (Array.isArray(value) || value === null) return null;
  for (const field of ["rows", "columns", "areas"]) {
    if (!Object.hasOwn(value, field)) return null;
  }
  return serializeTypedFields([
    ["grid-template-rows", value.rows],
    ["grid-template-columns", value.columns],
    ["grid-template-areas", value.areas],
  ]);
}

function typedRecords(
  name: string,
  parsed: AcceptedPropertyValue,
  important: boolean,
): DeclarationRecord[] | null {
  const values = expandLayeredTypedValue(name, parsed) ??
    expandBorderImageTypedValue(name, parsed) ??
    (name === "grid-template" ? expandGridTemplateTypedValue(parsed) : null);
  if (!values) return null;
  const safeValues = expandLayeredTypedValue(name, parsed, true) ?? values;
  return [...values].map(([longhand, value]) => ({
    name: longhand,
    observableValue: value,
    safeValue: safeValues.get(longhand) ?? value,
    pendingSubstitution: false,
    important,
    pendingGroup: null,
  }));
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
  parsed: AcceptedPropertyValue,
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
  parsed: AcceptedPropertyValue,
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
  parsed: AcceptedPropertyValue,
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
  parsed: AcceptedPropertyValue,
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
  parsed: AcceptedPropertyValue,
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

  const typedExpansion = typedRecords(name, parsed, important);
  if (typedExpansion) return typedExpansion;

  const structuralExpansion = expandMappedValues(
    name,
    parsed,
    important,
    expandStructuralValue,
  );
  if (structuralExpansion) return structuralExpansion;

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
