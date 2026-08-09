let internalConstructionDepth = 0;

export function constructInternally<T>(factory: () => T): T {
  internalConstructionDepth += 1;
  try {
    return factory();
  } finally {
    internalConstructionDepth -= 1;
  }
}

export function assertInternalConstructor(name: string): void {
  if (internalConstructionDepth > 0) return;
  throw new TypeError(`Failed to construct '${name}': Illegal constructor`);
}
