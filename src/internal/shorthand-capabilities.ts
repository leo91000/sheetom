import shorthandCapabilities from "../../compatibility/shorthand-capabilities.json" with { type: "json" };
import * as csstree from "css-tree";

function normalizeCapabilityValue(value: string): string | null {
  try {
    return csstree.generate(csstree.parse(value, { context: "value" }));
  } catch {
    return null;
  }
}

const acceptedCases = new Map(
  shorthandCapabilities.cases.map(capability => [
    `${capability.property}\0${normalizeCapabilityValue(capability.input)}`,
    capability.input,
  ] as const),
);
const longhandOrders = new Map(
  shorthandCapabilities.cases.map(capability => [
    capability.property,
    capability.chromium.items,
  ] as const),
);

export function matchesShorthandCapability(name: string, value: string): boolean {
  return getMatchingShorthandCapabilityInput(name, value) !== null;
}

export function getMatchingShorthandCapabilityInput(
  name: string,
  value: string,
): string | null {
  const normalized = normalizeCapabilityValue(value.trim());
  if (normalized === null) return null;
  return acceptedCases.get(`${name}\0${normalized}`) ?? null;
}

export function getShorthandCapabilityItems(name: string): readonly string[] | null {
  return longhandOrders.get(name) ?? null;
}
