import { transform, transformStyleAttribute } from "lightningcss";
import * as csstree from "css-tree";

import {
  chromiumShorthandLonghands,
  chromiumSupportedProperties,
} from "../chromium-properties.js";
import type {
  AcceptedPropertyValue,
  ParsedPropertyValue,
} from "./declaration-block.js";
import { serializeObservableValue } from "./observable-value-codec.js";
import { matchesShorthandCapability } from "./shorthand-capabilities.js";
import {
  matchesMeasuredValueCapability,
  rejectsMeasuredValueCapability,
} from "./value-capabilities.js";

const encoder = new TextEncoder();
const decoder = new TextDecoder();
const cssWideKeywords = new Set(["initial", "inherit", "unset", "revert", "revert-layer"]);
const paddingLonghands = [
  "padding-top",
  "padding-right",
  "padding-bottom",
  "padding-left",
] as const;
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
  if (found) {
    const details = value.value;
    if (!isUnknownRecord(details)) return { found: true, valid: false };
    const fallback = details.fallback;
    if (Array.isArray(fallback) && fallback.some(item =>
      isToken(item, "semicolon") || isToken(item, "delim", "!")
    )) {
      return { found: true, valid: false };
    }
  }
  const cssFunction = functionValue(value);
  if (cssFunction && ["var", "env"].includes(cssFunction.name)) {
    found = true;
  }
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

export function serializeIdentifier(value: string): string {
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

function canonicalizeLeadingDecimal(value: string): string {
  const replacements: Array<{ start: number; end: number; value: string }> = [];
  csstree.tokenize(value, (type, start, end) => {
    const tokenName = csstree.tokenNames[type];
    if (tokenName === undefined || ![
      "number-token",
      "percentage-token",
      "dimension-token",
    ].includes(tokenName)) return;
    const token = value.slice(start, end);
    const normalized = token.replace(/^([+-]?)\.(\d)/, "$10.$2");
    if (normalized !== token) replacements.push({ start, end, value: normalized });
  });

  let result = value;
  for (let index = replacements.length - 1; index >= 0; index -= 1) {
    const replacement = replacements[index];
    if (!replacement) continue;
    result = `${result.slice(0, replacement.start)}${replacement.value}${result.slice(replacement.end)}`;
  }
  return result;
}

function containsTopLevelDeclarationBoundary(value: string): boolean {
  let depth = 0;
  let found = false;
  csstree.tokenize(value, (type, start, end) => {
    const tokenName = csstree.tokenNames[type];
    if (tokenName === undefined) return;
    if (["function-token", "(-token", "[-token", "{-token"].includes(tokenName)) {
      depth += 1;
      return;
    }
    if ([")-token", "]-token", "}-token"].includes(tokenName)) {
      if (depth > 0) depth -= 1;
      return;
    }
    if (
      depth === 0 &&
      (
        tokenName === "semicolon-token" ||
        (tokenName === "delim-token" && value.slice(start, end) === "!")
      )
    ) {
      found = true;
    }
  });
  return found;
}

function canonicalizeBrowserValue(name: string, value: string): string {
  if (name === "-webkit-mask-box-image" && value === "") return "none";
  if (
    name === "-webkit-box-reflect" &&
    ["above", "below", "left", "right"].includes(value)
  ) {
    return `${value} 0px`;
  }
  return value;
}

export function parsePropertyValue(
  name: string,
  observableValue: string,
): AcceptedPropertyValue | null {
  if (name === "--") return null;
  if (!name.startsWith("--") && !chromiumSupportedProperties.has(name)) {
    return null;
  }
  if (containsTopLevelDeclarationBoundary(observableValue)) return null;
  const trimmedInput = observableValue.trim();
  if (!name.startsWith("--") && cssWideKeywords.has(trimmedInput)) {
    return {
      observableValue: trimmedInput,
      safeValue: trimmedInput,
      pendingSubstitution: false,
      representation: { kind: "grammar", declaration: null },
    };
  }
  let declaration: unknown;
  let declarationCount = 0;

  try {
    const result = transformStyleAttribute({
      code: encoder.encode(
        `${name.startsWith("--") ? serializeIdentifier(name) : name}: ${observableValue}`,
      ),
      visitor: {
        Declaration(candidate) {
          declarationCount += 1;
          declaration = candidate;
        },
      },
    });

    if (declarationCount !== 1 || !isUnknownRecord(declaration)) return null;
    if (!Object.hasOwn(declaration, "property")) return null;

    const analysis = analyzeSubstitutions(declaration);
    if (!analysis.valid) return null;
    const pendingSubstitution = analysis.found;
    if (
      !pendingSubstitution &&
      rejectsMeasuredValueCapability(name, observableValue)
    ) {
      return null;
    }
    const typedOrdinaryValue = declaration.property !== "unparsed" &&
      declaration.property !== "custom";
    if (
      !name.startsWith("--") &&
      !pendingSubstitution &&
      !typedOrdinaryValue &&
      !matchesMeasuredValueCapability(name, observableValue) &&
      !matchesShorthandCapability(name, observableValue)
    ) {
      return null;
    }

    const serialized = decoder.decode(result.code);
    const parsed = csstree.parse(serialized, {
      context: "declarationList",
      positions: true,
    });
    if (parsed.type !== "DeclarationList") return null;
    const serializedDeclarations = parsed.children.toArray();
    if (serializedDeclarations.length !== 1) return null;
    const serializedDeclaration = serializedDeclarations[0];
    if (serializedDeclaration?.type !== "Declaration") return null;
    if (serializedDeclaration.important) return null;

    const serializedName = csstree.ident.decode(serializedDeclaration.property);
    if (serializedName !== name) return null;
    const valueLocation = serializedDeclaration.value.loc;
    const safeValue = valueLocation
      ? serialized.slice(valueLocation.start.offset, valueLocation.end.offset)
      : csstree.generate(serializedDeclaration.value);

    const observableCategory = name.startsWith("--")
      ? "custom"
      : pendingSubstitution
        ? "pending-substitution"
        : "typed";
    const serializedObservableValue = serializeObservableValue({
      name,
      input: observableValue,
      safeValue: safeValue.trim(),
      category: observableCategory,
    });
    const trimmedObservableValue = canonicalizeBrowserValue(
      name,
      observableCategory === "typed"
        ? canonicalizeLeadingDecimal(serializedObservableValue)
        : serializedObservableValue,
    );
    const normalizedSafeValue = canonicalizeBrowserValue(name, safeValue.trim());
    return {
      observableValue:
        trimmedObservableValue === "0" && zeroLengthProperties.has(name)
          ? "0px"
          : trimmedObservableValue,
      safeValue: normalizedSafeValue,
      pendingSubstitution,
      representation: {
        kind: pendingSubstitution
          ? "pending-substitution"
          : typedOrdinaryValue
            ? "typed"
            : "grammar",
        declaration,
      },
    };
  } catch {
    return null;
  }
}
export function parseAtruleDescriptorValue(
  atrule: string,
  name: string,
  observableValue: string,
): ParsedPropertyValue | null {
  if (containsTopLevelDeclarationBoundary(observableValue)) return null;
  try {
    const match = csstree.lexer.matchAtruleDescriptor(
      atrule,
      name,
      observableValue,
    );
    if (match.error) return null;

    const prelude = atrule === "counter-style" ? " sheetom" : "";
    const result = transform({
      filename: "sheetom-descriptor.css",
      code: encoder.encode(`@${atrule}${prelude} { ${name}: ${observableValue}; }`),
    });
    const serializedSheet = decoder.decode(result.code);
    const parsed = csstree.parse(serializedSheet, { positions: true });
    if (parsed.type !== "StyleSheet") return null;
    const rule = parsed.children.first;
    if (rule?.type !== "Atrule" || !rule.block) return null;
    const declaration = rule.block.children.first;
    if (declaration?.type !== "Declaration") return null;
    const valueLocation = declaration.value.loc;
    const serializedValue = valueLocation
      ? serializedSheet.slice(valueLocation.start.offset, valueLocation.end.offset)
      : csstree.generate(declaration.value);
    return {
      observableValue: serializedValue,
      safeValue: serializedValue,
      pendingSubstitution: false,
    };
  } catch {
    return null;
  }
}
