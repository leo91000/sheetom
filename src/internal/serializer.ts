export type RuleSerializationPlan<Node> =
  | { kind: "raw"; cssText: string }
  | { kind: "declarations"; declarations: string }
  | {
      kind: "block";
      header: string;
      declarations?: string;
      children?: readonly Node[];
    };

type SerializationFrame<Node> =
  | { kind: "node"; node: Node; depth: number; child: boolean }
  | { kind: "declarations"; value: string; depth: number }
  | { kind: "text"; value: string };

/** Formats reparsable CSS from immutable descriptions of live rule state. */
export class Serializer<Node> {
  readonly #describe: (node: Node) => RuleSerializationPlan<Node>;

  constructor(describe: (node: Node) => RuleSerializationPlan<Node>) {
    this.#describe = describe;
  }

  serialize(node: Node): string {
    const chunks: string[] = [];
    const pending: SerializationFrame<Node>[] = [
      { kind: "node", node, depth: 0, child: false },
    ];
    while (pending.length > 0) {
      const frame = pending.pop();
      if (!frame) continue;
      if (frame.kind === "text") {
        chunks.push(frame.value);
        continue;
      }
      if (frame.kind === "declarations") {
        appendIndented(chunks, frame.value, frame.depth);
        continue;
      }

      const plan = this.#describe(frame.node);
      if (plan.kind === "raw") {
        const value = frame.child ? plan.cssText.trimEnd() : plan.cssText;
        appendIndented(chunks, value, frame.depth);
        if (!frame.child) chunks.push("\n");
        continue;
      }
      if (plan.kind === "declarations") {
        const value = frame.child ? plan.declarations.trimEnd() : plan.declarations;
        if (frame.child) appendIndented(chunks, value, frame.depth);
        else chunks.push(value, value === "" ? "" : "\n");
        continue;
      }

      chunks.push("  ".repeat(frame.depth), `${plan.header} {\n`);
      const children = plan.children ?? [];
      const contentCount = (plan.declarations ? 1 : 0) + children.length;
      if (contentCount === 0) {
        appendIndented(chunks, "}", frame.depth);
        if (!frame.child) chunks.push("\n");
        continue;
      }

      pending.push({
        kind: "text",
        value: `\n${"  ".repeat(frame.depth)}}${frame.child ? "" : "\n"}`,
      });
      for (let index = children.length - 1; index >= 0; index -= 1) {
        const child = children[index];
        if (!child) continue;
        pending.push({ kind: "node", node: child, depth: frame.depth + 1, child: true });
        if (index > 0 || plan.declarations) {
          pending.push({ kind: "text", value: "\n" });
        }
      }
      if (plan.declarations) {
        pending.push({ kind: "declarations", value: plan.declarations, depth: frame.depth });
      }
    }
    return chunks.join("");
  }
}

function appendIndented(chunks: string[], value: string, depth: number): void {
  const indentation = "  ".repeat(depth);
  chunks.push(indentation, value.replaceAll("\n", `\n${indentation}`));
}
