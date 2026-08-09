import {
  TokenType,
  tokenize,
  type CSSToken,
} from "@csstools/css-tokenizer";

const trivia = new Set<string>([
  TokenType.Whitespace,
  TokenType.Comment,
  TokenType.CDO,
  TokenType.CDC,
]);

const closingFor = new Map<string, string>([
  [TokenType.Function, TokenType.CloseParen],
  [TokenType.OpenParen, TokenType.CloseParen],
  [TokenType.OpenSquare, TokenType.CloseSquare],
  [TokenType.OpenCurly, TokenType.CloseCurly],
]);

/** Consumes top-level CSS Syntax rules and retains their exact UTF-16 spans. */
export function scanTopLevelRules(css: string): string[] {
  const tokens = tokenize({ css });
  const rules: string[] = [];
  let index = 0;

  while (index < tokens.length) {
    const token = tokens[index];
    if (!token || token[0] === TokenType.EOF) break;
    if (trivia.has(token[0]) || token[0] === TokenType.Semicolon) {
      index += 1;
      continue;
    }
    if (token[0] === TokenType.CloseCurly) {
      index += 1;
      continue;
    }

    const start = token[2];
    const stack: string[] = [];
    let end = token[3];
    let foundBoundary = false;

    for (; index < tokens.length; index += 1) {
      const current = tokens[index];
      if (!current || current[0] === TokenType.EOF) break;
      end = current[3];

      const closing = closingFor.get(current[0]);
      if (closing) {
        stack.push(closing);
        continue;
      }

      const expected = stack.at(-1);
      if (expected && current[0] === expected) {
        stack.pop();
        if (current[0] === TokenType.CloseCurly && stack.length === 0) {
          index += 1;
          foundBoundary = true;
          break;
        }
        continue;
      }

      if (current[0] === TokenType.Semicolon && stack.length === 0) {
        index += 1;
        foundBoundary = true;
        break;
      }
    }

    const sourceEnd = foundBoundary ? end + 1 : css.length;
    const rule = css.slice(start, sourceEnd).trim();
    if (rule !== "") rules.push(rule);
  }

  return rules;
}
