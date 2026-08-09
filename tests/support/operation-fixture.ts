export interface BoundaryTag {
  $type: string;
  value?: string;
}

export interface FixtureOperation {
  op: string;
  target: string;
  handle?: string;
  args: Array<unknown | BoundaryTag>;
  observe?: string[];
}

export interface OperationFixture {
  schemaVersion: 1;
  id: string;
  operations: FixtureOperation[];
}

export interface FixtureAdapter {
  invoke(operation: FixtureOperation, target: unknown, args: unknown[]): unknown;
}

export type FixtureObservation = Record<string, unknown>;

function isBoundaryTag(value: unknown): value is BoundaryTag {
  return typeof value === "object" && value !== null && "$type" in value;
}

function decodeBoundaryValue(value: unknown): unknown {
  if (!isBoundaryTag(value)) return value;

  switch (value.$type) {
    case "undefined":
      return undefined;
    case "nan":
      return Number.NaN;
    case "positive-infinity":
      return Number.POSITIVE_INFINITY;
    case "negative-infinity":
      return Number.NEGATIVE_INFINITY;
    case "bigint":
      return BigInt(value.value ?? "0");
    case "symbol":
      return Symbol(value.value);
    case "throwing-string-coercion":
      return {
        toString(): never {
          throw new Error(value.value ?? "string coercion failed");
        },
      };
    default:
      throw new Error(`Unknown Boundary Value: ${value.$type}`);
  }
}

function encodeBoundaryValue(value: unknown): unknown {
  if (value === undefined) return { $type: "undefined" };
  if (typeof value === "number" && Number.isNaN(value)) return { $type: "nan" };
  if (value === Number.POSITIVE_INFINITY) return { $type: "positive-infinity" };
  if (value === Number.NEGATIVE_INFINITY) return { $type: "negative-infinity" };
  if (typeof value === "bigint") return { $type: "bigint", value: value.toString() };
  if (typeof value === "symbol") return { $type: "symbol", value: value.description };
  return value;
}

function observeTarget(
  observation: FixtureObservation,
  target: unknown,
  requested: string[],
): void {
  if (typeof target !== "object" || target === null) return;
  const record = target as Record<string, unknown>;

  if (requested.includes("cssText")) observation.cssText = record.cssText;
  if (requested.includes("length")) observation.length = record.length;
  if (requested.includes("serialize")) {
    const serialize = record.serialize;
    if (typeof serialize === "function") observation.serialize = serialize.call(target);
  }
  if (requested.includes("items")) {
    const length = typeof record.length === "number" ? record.length : 0;
    const item = record.item;
    observation.items = typeof item === "function"
      ? Array.from({ length }, (_, index) => item.call(target, index))
      : [];
  }
}

export async function runOperationFixture(
  fixture: OperationFixture,
  adapter: FixtureAdapter,
): Promise<FixtureObservation[]> {
  const handles = new Map<string, unknown>([["$root", null]]);
  const observations: FixtureObservation[] = [];

  for (const operation of fixture.operations) {
    if (!handles.has(operation.target)) {
      throw new Error(`Unknown fixture handle: ${operation.target}`);
    }

    const target = handles.get(operation.target);
    const args = operation.args.map(decodeBoundaryValue);
    const observation: FixtureObservation = {};
    let result: unknown;

    try {
      result = await adapter.invoke(operation, target, args);
      observation.exception = null;
    } catch (error) {
      observation.exception = {
        name: error instanceof Error ? error.name : "UnknownError",
      };
    }

    if (operation.handle && observation.exception === null) {
      handles.set(operation.handle, result);
    }

    const requested = operation.observe ?? [];
    if (requested.includes("return") && observation.exception === null) {
      observation.return = encodeBoundaryValue(result);
    }
    if (!requested.includes("exception")) delete observation.exception;
    observeTarget(observation, target, requested);
    observations.push(observation);
  }

  return observations;
}
