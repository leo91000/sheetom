import {
  definitionSyntax,
  type DSNode,
  type DSNodeMultiplier,
  type DSNodeType,
  type DSNodeTypeOpts,
} from "css-tree";

export interface WebrefSyntaxSample {
  value: string;
  branch: string;
}

export interface WebrefSyntaxIssue {
  kind: string;
  path: string;
  [detail: string]: unknown;
}

export interface WebrefDefinitions {
  properties: Record<string, { syntax: string; [field: string]: unknown }>;
  types: Record<string, { syntax: string; [field: string]: unknown }>;
  functions: Record<string, { syntax: string; [field: string]: unknown }>;
}

export interface WebrefSyntaxSamplerOptions {
  definitions: WebrefDefinitions;
  property: string;
  syntax: string;
  fallbackValue?: (property: string) => string | null | undefined;
  terminalValues?: Record<string, string[]>;
  maximumDepth?: number;
  maximumSamplesPerNode?: number;
}

const primitiveValues = new Map<string, readonly string[]>([
  ["angle", ["45deg", "0deg", "-45deg"]],
  ["custom-ident", ["sheetom-ident"]],
  ["dashed-ident", ["--sheetom-ident"]],
  ["dimension", ["1px", "0px", "-1px"]],
  ["flex", ["1fr", "0fr"]],
  ["hash-token", ["#sheetom"]],
  ["hex-color", ["#123456"]],
  ["ident", ["sheetom-ident"]],
  ["length", ["1px", "0px", "-1px"]],
  ["number", ["1", "0", "-1", "1.5"]],
  ["number-token", ["1", "0", "-1", "1.5"]],
  ["percentage", ["10%", "0%", "-10%"]],
  ["rect()", ["rect(0, 0, 0, 0)"]],
  ["size-keyword", ["auto", "min-content", "max-content"]],
  ["string", ["\"sheetom\""]],
  ["time", ["1s", "0s", "-1s"]],
  ["url-token", ["url(\"sheetom.css\")"]],
]);

const recursiveValues = new Map<string, string>([
  ["color", "red"],
]);

function joinCss(parts: readonly string[], separator = " "): string {
  return parts
    .filter(Boolean)
    .join(separator)
    .replace(/\(\s+/g, "(")
    .replace(/\s+\)/g, ")")
    .replace(/\s+,/g, ",")
    .replace(/,\s*/g, ", ")
    .replace(/\s*\/\s*/g, " / ")
    .trim();
}

function numericPrefix(value: string): number | null {
  const match = /^(-?(?:\d+(?:\.\d*)?|\.\d+))/.exec(value);
  return match ? Number(match[1]) : null;
}

function satisfiesRange(value: string, range: DSNodeTypeOpts | null): boolean {
  if (!range) return true;
  const numeric = numericPrefix(value);
  if (numeric === null) return true;
  if (typeof range.min === "number" && numeric < range.min) return false;
  if (typeof range.max === "number" && numeric > range.max) return false;
  return true;
}

function deduplicate(samples: readonly WebrefSyntaxSample[]): WebrefSyntaxSample[] {
  const seen = new Set<string>();
  const output: WebrefSyntaxSample[] = [];
  for (const sample of samples) {
    const key = `${sample.branch}\0${sample.value}`;
    if (seen.has(key)) continue;
    seen.add(key);
    output.push(sample);
  }
  return output;
}

function pairwiseProduct(
  sampleSets: readonly WebrefSyntaxSample[][],
  path: string,
  reverse = false,
): WebrefSyntaxSample[] {
  if (sampleSets.some(samples => samples.length === 0)) return [];
  const baseline = sampleSets.map(samples => samples[0]!);
  const candidates = [baseline];
  for (let index = 0; index < sampleSets.length; index += 1) {
    for (const sample of sampleSets[index]!.slice(1)) {
      const candidate = [...baseline];
      candidate[index] = sample;
      candidates.push(candidate);
    }
  }
  if (reverse && sampleSets.length > 1) candidates.push([...baseline].reverse());
  return candidates.map((samples, index) => ({
    value: joinCss(samples.map(sample => sample.value)),
    branch: `${path}/combination:${index}`,
  }));
}

function repeatedSamples(
  samples: readonly WebrefSyntaxSample[],
  node: DSNodeMultiplier,
  path: string,
): WebrefSyntaxSample[] {
  const positiveMinimum = Math.max(1, node.min);
  const unbounded = node.max === 0 || node.max === Infinity;
  const counts = new Set([positiveMinimum]);
  if (unbounded) {
    counts.add(Math.max(2, positiveMinimum + 1));
  } else if (node.max > positiveMinimum) {
    counts.add(positiveMinimum + 1);
    counts.add(node.max);
  }

  const separator = node.comma ? ", " : " ";
  const output: WebrefSyntaxSample[] = [];
  for (const [index, sample] of samples.entries()) {
    output.push({
      value: Array.from({ length: positiveMinimum }, () => sample.value).join(separator),
      branch: `${path}/count:${positiveMinimum}/value:${index}`,
    });
  }
  for (const count of [...counts].slice(1)) {
    output.push({
      value: Array.from({ length: count }, () => samples[0]!.value).join(separator),
      branch: `${path}/count:${count}`,
    });
  }
  if (node.min === 0) output.push({ value: "", branch: `${path}/count:0` });
  return output;
}

function anyOrderSamples(
  sampleSets: readonly WebrefSyntaxSample[][],
  path: string,
): WebrefSyntaxSample[] {
  const nonEmpty = sampleSets.map(samples => samples.filter(sample => sample.value));
  const output: WebrefSyntaxSample[] = [];
  for (const [termIndex, samples] of nonEmpty.entries()) {
    for (const [sampleIndex, sample] of samples.entries()) {
      output.push({
        value: sample.value,
        branch: `${path}/subset:${termIndex}/value:${sampleIndex}`,
      });
    }
  }

  for (let left = 0; left < nonEmpty.length; left += 1) {
    const leftSamples = nonEmpty[left];
    if (!leftSamples || leftSamples.length === 0) continue;
    for (let right = left + 1; right < nonEmpty.length; right += 1) {
      const rightSamples = nonEmpty[right];
      if (!rightSamples || rightSamples.length === 0) continue;
      output.push({
        value: joinCss([leftSamples[0]!.value, rightSamples[0]!.value]),
        branch: `${path}/subset:${left},${right}`,
      });
      output.push({
        value: joinCss([rightSamples[0]!.value, leftSamples[0]!.value]),
        branch: `${path}/subset:${right},${left}`,
      });
    }
  }

  if (nonEmpty.length > 1 && nonEmpty.every(samples => samples.length > 0)) {
    output.push(...pairwiseProduct(nonEmpty, `${path}/all`, true));
  }
  return output;
}

/**
 * Expands a Webref Value Definition Syntax into deterministic representative
 * values. The returned samples cover every reachable alternative without
 * constructing the full Cartesian product of independent grammar branches.
 */
export function generateWebrefSyntaxSamples({
  definitions,
  property,
  syntax,
  fallbackValue,
  terminalValues = {},
  maximumDepth = 32,
  maximumSamplesPerNode = 2048,
}: WebrefSyntaxSamplerOptions): {
  samples: WebrefSyntaxSample[];
  issues: WebrefSyntaxIssue[];
} {
  const syntaxCache = new Map<string, DSNode>();
  const issues: WebrefSyntaxIssue[] = [];

  function parse(source: string): DSNode {
    let ast = syntaxCache.get(source);
    if (!ast) {
      ast = definitionSyntax.parse(source);
      syntaxCache.set(source, ast);
    }
    return ast;
  }

  function limited(
    samples: readonly WebrefSyntaxSample[],
    path: string,
  ): WebrefSyntaxSample[] {
    const unique = deduplicate(samples);
    if (unique.length <= maximumSamplesPerNode) return unique;
    issues.push({
      kind: "sample-budget",
      path,
      produced: unique.length,
      maximum: maximumSamplesPerNode,
    });
    return unique.slice(0, maximumSamplesPerNode);
  }

  function primitiveSamples(
    node: DSNodeType,
    path: string,
  ): WebrefSyntaxSample[] | null {
    const values = primitiveValues.get(node.name);
    if (!values) return null;
    return values
      .filter(value => satisfiesRange(value, node.opts))
      .map((value, index) => ({
        value,
        branch: `${path}/primitive:${node.name}:${index}`,
      }));
  }

  function expand(
    node: DSNode,
    path: string,
    stack: ReadonlySet<string>,
    depth: number,
  ): WebrefSyntaxSample[] {
    if (depth > maximumDepth) {
      issues.push({ kind: "depth-budget", path, maximum: maximumDepth });
      return [];
    }

    switch (node.type) {
      case "Keyword":
        return [{ value: node.name, branch: `${path}/keyword:${node.name}` }];
      case "String":
        return [{
          value: node.value.slice(1, -1),
          branch: `${path}/literal:${node.value.slice(1, -1)}`,
        }];
      case "Function":
        return [{ value: `${node.name}(`, branch: `${path}/function:${node.name}` }];
      case "Token":
        return [{ value: node.value, branch: `${path}/token:${node.value}` }];
      case "Comma":
        return [{ value: ",", branch: `${path}/comma` }];
      case "Type": {
        const terminal = terminalValues[node.name];
        if (terminal) {
          return terminal
            .filter(value => satisfiesRange(value, node.opts))
            .map((value, index) => ({
              value,
              branch: `${path}/terminal:${node.name}:${index}`,
            }));
        }
        const primitive = primitiveSamples(node, path);
        if (primitive) return primitive;
        const key = `type:${node.name}`;
        if (stack.has(key)) {
          const recursive = recursiveValues.get(node.name);
          if (recursive) {
            return [{ value: recursive, branch: `${path}/${key}:recursive` }];
          }
          issues.push({ kind: "cycle", path, reference: key });
          return [];
        }
        const definition = definitions.types[node.name] ?? definitions.functions[node.name];
        if (!definition?.syntax) {
          issues.push({ kind: "missing-definition", path, reference: key });
          return [];
        }
        const nested = expand(
          parse(definition.syntax),
          `${path}/${key}`,
          new Set([...stack, key]),
          depth + 1,
        );
        return limited(nested, path);
      }
      case "Property": {
        const key = `property:${node.name}`;
        if (stack.has(key)) {
          const fallback = fallbackValue?.(node.name);
          if (fallback) {
            return [{ value: fallback, branch: `${path}/${key}:fallback` }];
          }
          issues.push({ kind: "cycle", path, reference: key });
          return [];
        }
        const definition = definitions.properties[node.name];
        if (!definition?.syntax) {
          const fallback = fallbackValue?.(node.name);
          if (fallback) {
            return [{ value: fallback, branch: `${path}/${key}:fallback` }];
          }
          issues.push({ kind: "missing-definition", path, reference: key });
          return [];
        }
        const nested = expand(
          parse(definition.syntax),
          `${path}/${key}`,
          new Set([...stack, key]),
          depth + 1,
        );
        return limited(nested, path);
      }
      case "Multiplier": {
        const child = expand(node.term, `${path}/multiplier`, stack, depth + 1)
          .filter(sample => sample.value);
        if (child.length === 0) {
          if (node.min === 0) {
            return [{ value: "", branch: `${path}/omitted` }];
          }
          return [];
        }
        return limited(repeatedSamples(child, node, path), path);
      }
      case "Group": {
        const sampleSets = node.terms.map((term, index) =>
          expand(term, `${path}/term:${index}`, stack, depth + 1));
        if (node.combinator === "|") {
          return limited(sampleSets.flat(), path);
        }
        if (node.combinator === "||") {
          return limited(anyOrderSamples(sampleSets, path), path);
        }
        if (node.combinator === "&&") {
          return limited(pairwiseProduct(sampleSets, path, true), path);
        }
        return limited(pairwiseProduct(sampleSets, path), path);
      }
      default:
        issues.push({ kind: "unknown-node", path, nodeType: node.type });
        return [];
    }
  }

  const rootKey = `property:${property}`;
  const samples = limited(
    expand(parse(syntax), rootKey, new Set([rootKey]), 0)
      .filter(sample => sample.value),
    rootKey,
  );
  return { samples, issues };
}
