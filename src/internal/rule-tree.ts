/** Owns child-list identity and parent propagation for every live rule node. */
export class RuleTree<Node extends object, Sheet> {
  readonly #children = new WeakMap<Node, Node[]>();
  readonly #setParentage: (
    node: Node,
    parentRule: Node | null,
    parentStyleSheet: Sheet | null,
  ) => void;
  readonly #parentStyleSheet: (node: Node) => Sheet | null;

  constructor(
    setParentage: (
      node: Node,
      parentRule: Node | null,
      parentStyleSheet: Sheet | null,
    ) => void,
    parentStyleSheet: (node: Node) => Sheet | null,
  ) {
    this.#setParentage = setParentage;
    this.#parentStyleSheet = parentStyleSheet;
  }

  createChildren(owner: Node): Node[] {
    const children: Node[] = [];
    this.#children.set(owner, children);
    return children;
  }

  children(owner: Node): Node[] {
    return this.#children.get(owner) ?? [];
  }

  replace(owner: Node, children: Node[]): void {
    const current = this.#children.get(owner);
    if (!current) return;

    for (const child of current) {
      this.attach(child, null, null);
    }
    current.splice(0, current.length, ...children);
    for (const child of children) {
      this.attach(child, owner, this.#parentStyleSheet(owner));
    }
  }

  attach(node: Node, parentRule: Node | null, parentStyleSheet: Sheet | null): void {
    this.#setParentage(node, parentRule, parentStyleSheet);
    for (const child of this.#children.get(node) ?? []) {
      this.attach(child, node, parentStyleSheet);
    }
  }
}
