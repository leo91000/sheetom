import {
  TokenType,
  tokenize,
  type CSSToken,
} from "@csstools/css-tokenizer";

import { chromiumShorthandLonghands } from "../chromium-properties.js";

export type ObservableValueCategory =
  | "typed"
  | "pending-substitution"
  | "custom";

interface ObservableValueInput {
  name: string;
  input: string;
  safeValue: string;
  category: ObservableValueCategory;
}

const closingToken = new Map<string, { token: string; text: string }>([
  [TokenType.Function, { token: TokenType.CloseParen, text: ")" }],
  [TokenType.OpenParen, { token: TokenType.CloseParen, text: ")" }],
  [TokenType.OpenSquare, { token: TokenType.CloseSquare, text: "]" }],
  [TokenType.OpenCurly, { token: TokenType.CloseCurly, text: "}" }],
]);

function escapedString(value: string, quote: string): string {
  return value
    .replace(/\\/g, "\\\\")
    .replace(new RegExp(quote, "g"), `\\${quote}`);
}

function retainedToken(token: CSSToken): string {
  if (token[0] === TokenType.Comment || token[0] === TokenType.EOF) return "";
  const details = token[4];
  if (
    token[0] === TokenType.Ident &&
    token[1].endsWith("\\") &&
    details &&
    "value" in details &&
    `${details.value}`.endsWith("�")
  ) {
    return `${token[1].slice(0, -1)}�`;
  }
  if (
    token[0] === TokenType.URL &&
    token[1].endsWith("\\") &&
    details &&
    "value" in details
  ) {
    return `url(${details.value})`;
  }
  if (
    token[0] === TokenType.String &&
    token[1].endsWith("\\") &&
    details &&
    "value" in details
  ) {
    const quote = token[1][0] ?? '"';
    return `${quote}${escapedString(`${details.value}`, quote)}${quote}`;
  }
  return token[1];
}

function recoverTokenText(input: string): {
  closed: string;
  recovered: boolean;
  retained: string;
  stringValue: string | null;
} {
  let parseError = false;
  const tokens = tokenize(
    { css: input },
    { onParseError: () => {
      parseError = true;
    } },
  );
  const stack: Array<{ token: string; text: string }> = [];
  let retained = "";
  let closed = "";
  let removedComment = false;
  let stringValue: string | null = null;
  let significantTokenCount = 0;

  for (const token of tokens) {
    if (token[0] === TokenType.EOF) continue;
    if (token[0] === TokenType.Comment) {
      removedComment = true;
      continue;
    }

    if (token[0] !== TokenType.Whitespace) {
      significantTokenCount += 1;
      const details = token[4];
      if (token[0] === TokenType.String && details && "value" in details) {
        stringValue = `${details.value}`;
      }
    }

    const text = retainedToken(token);
    retained += text;
    closed += text;

    const closing = closingToken.get(token[0]);
    if (closing) {
      stack.push(closing);
      continue;
    }
    if (stack.at(-1)?.token === token[0]) stack.pop();

    if (token[0] === TokenType.URL && !text.endsWith(")")) closed += ")";
    if (token[0] !== TokenType.String) continue;
    const quote = token[1][0];
    if (quote && !text.endsWith(quote)) closed += quote;
  }

  for (let index = stack.length - 1; index >= 0; index -= 1) {
    closed += stack[index]?.text ?? "";
  }
  return {
    closed: closed.trim(),
    recovered: parseError || removedComment || stack.length > 0,
    retained: retained.trim(),
    stringValue: significantTokenCount === 1 ? stringValue : null,
  };
}

const genericFontFamilies = new Set([
  "serif",
  "sans-serif",
  "monospace",
  "cursive",
  "fantasy",
  "system-ui",
  "ui-serif",
  "ui-sans-serif",
  "ui-monospace",
  "ui-rounded",
  "math",
  "fangsong",
  "emoji",
]);

function serializeRecoveredFontFamily(value: string): string {
  const identifier = /^-?(?:[_a-zA-Z\u0080-\u{10ffff}])(?:[-_a-zA-Z0-9\u0080-\u{10ffff}])*$/u;
  if (identifier.test(value) && !genericFontFamilies.has(value.toLowerCase())) {
    return value;
  }
  return `"${escapedString(value, '"')}"`;
}

function byteFromHex(value: string): number {
  return Number.parseInt(value, 16);
}

function alphaFromByte(value: number): string {
  const rounded = Math.round((value / 255) * 1000) / 1000;
  return `${rounded}`;
}

function serializeHexColor(value: string): string | null {
  const match = /^#([\da-f]{3,8})$/i.exec(value);
  if (!match?.[1] || ![3, 4, 6, 8].includes(match[1].length)) return null;
  const expanded = match[1].length <= 4
    ? [...match[1]].map(character => `${character}${character}`).join("")
    : match[1];
  const red = byteFromHex(expanded.slice(0, 2));
  const green = byteFromHex(expanded.slice(2, 4));
  const blue = byteFromHex(expanded.slice(4, 6));
  if (expanded.length === 6) return `rgb(${red}, ${green}, ${blue})`;
  const alpha = byteFromHex(expanded.slice(6, 8));
  return `rgba(${red}, ${green}, ${blue}, ${alphaFromByte(alpha)})`;
}

function serializeRgbColor(value: string): string | null {
  const match = /^rgba?\(\s*([^)]*)\s*\)$/i.exec(value);
  if (!match?.[1]) return null;
  const commaSyntax = match[1].includes(",");
  const slashParts = match[1].split("/").map(part => part.trim());
  const colorParts = commaSyntax
    ? (slashParts[0] ?? "").split(",").map(part => part.trim())
    : (slashParts[0] ?? "").split(/\s+/).filter(Boolean);
  let alpha = slashParts[1];
  if (commaSyntax && colorParts.length === 4) alpha = colorParts.pop();
  if (colorParts.length !== 3) return null;
  const channel = (part: string): number | null => {
    const numeric = Number.parseFloat(part);
    if (!Number.isFinite(numeric)) return null;
    return part.endsWith("%")
      ? Math.round(Math.min(100, Math.max(0, numeric)) * 255 / 100)
      : Math.round(Math.min(255, Math.max(0, numeric)));
  };
  const channels = colorParts.map(channel);
  if (channels.some(component => component === null)) return null;
  const [red, green, blue] = channels;
  if (red === undefined || green === undefined || blue === undefined) return null;
  if (alpha === undefined) return `rgb(${red}, ${green}, ${blue})`;
  const numericAlpha = Number.parseFloat(alpha);
  if (!Number.isFinite(numericAlpha)) return null;
  const normalizedAlpha = alpha.endsWith("%")
    ? Math.min(100, Math.max(0, numericAlpha)) / 100
    : Math.min(1, Math.max(0, numericAlpha));
  return `rgba(${red}, ${green}, ${blue}, ${normalizedAlpha})`;
}

function serializeColor(value: string, safeValue: string): string {
  const direct = serializeRgbColor(value) ?? serializeHexColor(value);
  if (direct !== null) return direct;
  if (/^(?:hsl|hsla|hwb|lab|lch|oklab|oklch|color)\(/i.test(value)) {
    return serializeHexColor(safeValue) ?? value;
  }
  return /^[a-z-]+$/i.test(value) ? value.toLowerCase() : value;
}

function serializeIntegerCalculation(name: string, value: string): string | null {
  if (name !== "z-index") return null;
  const match = /^calc\(\s*([+-]?(?:\d+(?:\.\d*)?|\.\d+))\s*([+-])\s*([+-]?(?:\d+(?:\.\d*)?|\.\d+))\s*\)$/i.exec(value);
  if (!match?.[1] || !match[2] || !match[3]) return null;
  const left = Number.parseFloat(match[1]);
  const right = Number.parseFloat(match[3]);
  const result = match[2] === "+" ? left + right : left - right;
  if (!Number.isInteger(result)) return null;
  return `calc(${result})`;
}

function serializeTypedValue(
  name: string,
  input: string,
  safeValue: string,
  recovered: ReturnType<typeof recoverTokenText>,
): string {
  const shorthandLonghands = chromiumShorthandLonghands[name];
  if (shorthandLonghands && shorthandLonghands.length > 1 && !recovered.recovered) {
    return input;
  }
  if (name.endsWith("color") || name === "color") {
    return serializeColor(recovered.closed, safeValue);
  }
  const integerCalculation = serializeIntegerCalculation(name, recovered.closed);
  if (integerCalculation !== null) return integerCalculation;
  if (/^(?:calc|min|max|clamp)\(/i.test(recovered.closed)) {
    return safeValue.startsWith("calc(") ? safeValue : `calc(${safeValue})`;
  }
  if (/gradient\(/i.test(recovered.closed)) return recovered.closed;
  if (!recovered.recovered) {
    return safeValue;
  }
  if (/^url\(/i.test(recovered.closed) || /^['"]/.test(input)) return safeValue;
  if (input.includes("/*")) {
    return recovered.retained;
  }
  return recovered.closed;
}

/** Produces browser-facing text without changing acceptance or reparsable output. */
export function serializeObservableValue({
  name,
  input,
  safeValue,
  category,
}: ObservableValueInput): string {
  const recovered = recoverTokenText(input.trim());
  if (category !== "typed") return recovered.retained;
  if (name === "font-family" && recovered.stringValue !== null) {
    return serializeRecoveredFontFamily(recovered.stringValue);
  }
  if (name === "font-family") return safeValue;
  return serializeTypedValue(name, input.trim(), safeValue, recovered);
}
