import { CSSStyleDeclaration as CSSStyleDeclarationOracle } from "cssstyle";

import { chromiumShorthandLonghands } from "../chromium-properties.js";
import type {
  DeclarationRecord,
  ParsedPropertyValue,
  PendingSubstitutionGroup,
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

export function isFourSideShorthand(name: string): boolean {
  return fourSideShorthandNames.has(name);
}

export function getShorthandLonghands(name: string): readonly string[] | null {
  if (!Object.hasOwn(chromiumShorthandLonghands, name)) return null;
  return chromiumShorthandLonghands[name] ?? null;
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

  const pendingGroup: PendingSubstitutionGroup = {
    shorthand: name,
    observableValue: parsed.observableValue,
    safeValue: parsed.safeValue,
  };
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
      pendingGroup,
    });
  }
  return records;
}
