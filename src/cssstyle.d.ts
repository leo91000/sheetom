declare module "cssstyle" {
  export class CSSStyleDeclaration {
    readonly length: number;
    cssText: string;
    item(index: number): string;
    getPropertyValue(name: string): string;
    setProperty(name: string, value: string, priority?: string): void;
    removeProperty(name: string): string;
    [Symbol.iterator](): Iterator<string>;
  }
}
