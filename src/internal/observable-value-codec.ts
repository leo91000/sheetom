import {
  TokenType,
  tokenize,
  type CSSToken,
} from "@csstools/css-tokenizer";
import { CSSStyleDeclaration as CSSStyleDeclarationOracle } from "cssstyle";

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

  for (const token of tokens) {
    if (token[0] === TokenType.EOF) continue;
    if (token[0] === TokenType.Comment) {
      removedComment = true;
      continue;
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
  };
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
  if (name === "font-family") return safeValue;
  if (!recovered.recovered) return input.trim();

  const oracle = new CSSStyleDeclarationOracle();
  oracle.setProperty(name, recovered.closed);
  return oracle.getPropertyValue(name) || recovered.closed;
}
