import * as csstree from "css-tree";

import { chromiumShorthandLonghands } from "../chromium-properties.js";
import overrides from "./shorthand-runtime-overrides.json" with { type: "json" };

function readPairs(values: string[][]): Array<readonly [string, string]> {
  const result: Array<readonly [string, string]> = [];
  for (const value of values) {
    const property = value[0];
    const input = value[1];
    if (property === undefined || input === undefined || value.length !== 2) {
      throw new Error("Invalid shorthand runtime override pair");
    }
    result.push([property, input]);
  }
  return result;
}

const literalOverrides = readPairs(overrides.literal);
const canonicalInputOverrides = readPairs(overrides.canonicalInputs);
const serializedLiteralOverrideAliases = readPairs(overrides.serializedAliases);
const measuredLonghandOrders: Readonly<Record<string, readonly string[]>> =
  overrides.longhandOrders;

function normalizeOverrideValue(value: string): string | null {
  try {
    return csstree.generate(csstree.parse(value, { context: "value" }));
  } catch {
    return null;
  }
}

function indexOverrides(values: ReadonlyArray<readonly [string, string]>): ReadonlyMap<string, string> {
  const result = new Map<string, string>();
  for (const [property, input] of values) {
    const normalized = normalizeOverrideValue(input);
    if (normalized === null) throw new Error(`Invalid shorthand runtime override: ${property}`);
    result.set(`${property}\0${normalized}`, input);
  }
  return result;
}

const acceptedLiteralOverrides = indexOverrides([
  ...literalOverrides,
  ...serializedLiteralOverrideAliases,
]);
const canonicalInputs = indexOverrides(canonicalInputOverrides);

export function matchesMeasuredShorthandOverride(name: string, value: string): boolean {
  const normalized = normalizeOverrideValue(value.trim());
  if (normalized === null) return false;
  return acceptedLiteralOverrides.has(`${name}\0${normalized}`);
}

export function getMatchingShorthandCanonicalInput(
  name: string,
  value: string,
): string | null {
  const normalized = normalizeOverrideValue(value.trim());
  if (normalized === null) return null;
  return canonicalInputs.get(`${name}\0${normalized}`) ?? null;
}

export function getShorthandRuntimeItems(name: string): readonly string[] | null {
  return measuredLonghandOrders[name] ?? chromiumShorthandLonghands[name] ?? null;
}
