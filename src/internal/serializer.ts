export type RuleSerializationPlan<Node> =
  | { kind: "raw"; cssText: string }
  | { kind: "declarations"; declarations: string }
  | {
      kind: "block";
      header: string;
      declarations?: string;
      children?: readonly Node[];
    };

/** Formats reparsable CSS from immutable descriptions of live rule state. */
export class Serializer<Node> {
  readonly #describe: (node: Node) => RuleSerializationPlan<Node>;

  constructor(describe: (node: Node) => RuleSerializationPlan<Node>) {
    this.#describe = describe;
  }

  serialize(node: Node): string {
    const plan = this.#describe(node);
    if (plan.kind === "raw") return `${plan.cssText}\n`;
    if (plan.kind === "declarations") {
      return `${plan.declarations}${plan.declarations === "" ? "" : "\n"}`;
    }

    const contents: string[] = [];
    if (plan.declarations) contents.push(plan.declarations);
    for (const child of plan.children ?? []) {
      contents.push(indent(this.serialize(child).trimEnd()));
    }
    if (contents.length === 0) return `${plan.header} {\n}\n`;
    return `${plan.header} {\n${contents.join("\n")}\n}\n`;
  }
}

function indent(value: string): string {
  return value.split("\n").map(line => `  ${line}`).join("\n");
}
