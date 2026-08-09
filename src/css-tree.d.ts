import "css-tree";

declare module "css-tree" {
  export const tokenNames: readonly string[];

  export function tokenize(
    source: string,
    onToken: (type: number, start: number, end: number) => void,
  ): void;
}
