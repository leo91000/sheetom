import * as csstree from "css-tree";

function matchesProperty(name: string, value: string): boolean {
  try {
    return csstree.lexer.matchProperty(name, value).error === null;
  } catch {
    return false;
  }
}

function matchesContrastColor(value: string): boolean {
  let ast: csstree.CssNode;
  try {
    ast = csstree.parse(value, { context: "value" });
  } catch {
    return false;
  }
  if (ast.type !== "Value") return false;
  const children = ast.children.toArray();
  if (children.length !== 1 || children[0]?.type !== "Function") return false;
  const fn = children[0];
  if (fn.name.toLowerCase() !== "contrast-color") return false;
  const argument = csstree.generate({ type: "Value", children: fn.children });
  try {
    return csstree.lexer.matchType("color", argument).error === null;
  } catch {
    return false;
  }
}

function matchesContentCapability(value: string): boolean {
  if (/\bleader\s*\(/i.test(value)) return false;
  if (/\btarget-(?:counter|counters|text)\s*\(\s*url\s*\(/i.test(value)) {
    return false;
  }
  return matchesProperty("content", value);
}

/** Fills measured browser grammar gaps without accepting arbitrary unparsed values. */
export function matchesMeasuredValueCapability(name: string, value: string): boolean {
  if (name === "content") return matchesContentCapability(value);
  if (name === "color" && /^\s*contrast-color\s*\(/i.test(value)) {
    return matchesContrastColor(value);
  }
  return matchesProperty(name, value);
}

/** Rejects measured parser false positives before a typed parser can accept them. */
export function rejectsMeasuredValueCapability(name: string, value: string): boolean {
  if (name === "content") return !matchesContentCapability(value);
  if (name === "z-index" && /^\s*calc\s*\(/i.test(value)) {
    let dimensional = false;
    csstree.tokenize(value, type => {
      const tokenName = csstree.tokenNames[type];
      if (tokenName === "dimension-token" || tokenName === "percentage-token") {
        dimensional = true;
      }
    });
    return dimensional || !matchesProperty(name, value);
  }
  return false;
}
