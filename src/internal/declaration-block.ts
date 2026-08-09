import type { SheetOMDiagnosticCode } from "../diagnostics.js";

export interface ParsedPropertyValue {
  observableValue: string;
  safeValue: string;
  pendingSubstitution: boolean;
}

export interface AcceptedPropertyValue extends ParsedPropertyValue {
  representation: {
    kind: "typed" | "grammar" | "pending-substitution";
    declaration: unknown;
  };
}

export interface PendingSubstitutionGroup {
  shorthand: string;
  observableValue: string;
  safeValue: string;
}

export interface DeclarationRecord extends ParsedPropertyValue {
  name: string;
  important: boolean;
  pendingGroup: PendingSubstitutionGroup | null;
}

export interface ParsedDeclaration {
  name: string;
  value: string;
  important: boolean;
}

export interface DeclarationBlockCodec {
  normalizeName(name: string): string;
  parseValue(name: string, value: string): AcceptedPropertyValue | null;
  shorthandLonghands(name: string): readonly string[] | null;
  staticShorthandNames(): readonly string[];
  expandFourSide(
    name: string,
    parsed: AcceptedPropertyValue,
    important: boolean,
  ): DeclarationRecord[] | null;
  expandShorthand(
    name: string,
    parsed: AcceptedPropertyValue,
    important: boolean,
  ): DeclarationRecord[] | null;
  synthesizeShorthand(
    name: string,
    records: readonly DeclarationRecord[],
    safe: boolean,
  ): string | null;
  serializeIdentifier(value: string): string;
  normalizeIndex(value: unknown): number;
  isFourSideShorthand(name: string): boolean;
}

export type ReportDeclarationDiagnostic = (
  code: SheetOMDiagnosticCode,
  property: string,
  input: string,
) => void;

/** Owns ordered declaration records and all atomic mutation decisions. */
export class DeclarationBlock {
  readonly #codec: DeclarationBlockCodec;
  readonly #reportDiagnostic: ReportDeclarationDiagnostic;
  readonly #records: DeclarationRecord[] = [];

  constructor(
    codec: DeclarationBlockCodec,
    reportDiagnostic: ReportDeclarationDiagnostic,
  ) {
    this.#codec = codec;
    this.#reportDiagnostic = reportDiagnostic;
  }

  replace(declarations: ParsedDeclaration[] | null): void {
    if (!declarations) {
      this.#records.splice(0);
      return;
    }

    const winners = new Map<
      string,
      DeclarationRecord & { sourceIndex: number; subIndex: number }
    >();
    let sourceIndex = 0;

    const consider = (record: DeclarationRecord, subIndex: number): void => {
      const existing = winners.get(record.name);
      if (existing?.important && !record.important) return;
      winners.set(record.name, { ...record, sourceIndex, subIndex });
    };

    for (const declaration of declarations) {
      const name = this.#codec.normalizeName(declaration.name);
      const propertyValue = this.#codec.parseValue(name, declaration.value);
      if (!propertyValue) {
        sourceIndex += 1;
        continue;
      }

      const longhands = this.#codec.shorthandLonghands(name);
      if (propertyValue.pendingSubstitution && longhands) {
        const pendingGroup: PendingSubstitutionGroup = {
          shorthand: name,
          observableValue: propertyValue.observableValue,
          safeValue: propertyValue.safeValue,
        };
        for (let index = 0; index < longhands.length; index += 1) {
          const longhand = longhands[index];
          if (!longhand) continue;
          consider({
            name: longhand,
            observableValue: "",
            safeValue: "",
            pendingSubstitution: true,
            important: declaration.important,
            pendingGroup,
          }, index);
        }
        sourceIndex += 1;
        continue;
      }

      const expansion = this.#codec.expandFourSide(
        name,
        propertyValue,
        declaration.important,
      ) ?? this.#codec.expandShorthand(
        name,
        propertyValue,
        declaration.important,
      );
      if (expansion) {
        for (let index = 0; index < expansion.length; index += 1) {
          const record = expansion[index];
          if (record) consider(record, index);
        }
        sourceIndex += 1;
        continue;
      }

      if (longhands) {
        sourceIndex += 1;
        continue;
      }

      consider({
        name,
        observableValue: propertyValue.observableValue,
        safeValue: propertyValue.safeValue,
        pendingSubstitution: propertyValue.pendingSubstitution,
        important: declaration.important,
        pendingGroup: null,
      }, 0);
      sourceIndex += 1;
    }

    const records = [...winners.values()].sort((left, right) => {
      if (left.important !== right.important) return left.important ? 1 : -1;
      return left.sourceIndex - right.sourceIndex || left.subIndex - right.subIndex;
    });
    this.#records.splice(0, this.#records.length, ...records);
  }

  get length(): number {
    return this.#records.length;
  }

  item(index: unknown): string {
    return this.#records[this.#codec.normalizeIndex(index)]?.name ?? "";
  }

  getPropertyValue(name: string): string {
    const normalizedName = this.#codec.normalizeName(name);
    const shorthand = this.#shorthand(normalizedName, false);
    if (shorthand) return shorthand.value;
    return this.#records.find(record => record.name === normalizedName)?.observableValue ?? "";
  }

  getPropertyPriority(name: string): string {
    const normalizedName = this.#codec.normalizeName(name);
    const shorthand = this.#shorthand(normalizedName, false);
    if (shorthand) return shorthand.important ? "important" : "";
    return this.#records.find(record => record.name === normalizedName)?.important
      ? "important"
      : "";
  }

  setProperty(name: string, value: string, priority: string): void {
    const normalizedName = this.#codec.normalizeName(name);
    const normalizedPriority = priority.toLowerCase();
    if (normalizedPriority !== "" && normalizedPriority !== "important") {
      this.#reportDiagnostic("INVALID_PRIORITY", normalizedName, priority);
      return;
    }

    if (value === "") {
      this.removeProperty(normalizedName);
      return;
    }

    const parsed = this.#codec.parseValue(normalizedName, value);
    if (!parsed) {
      this.#reportDiagnostic("INVALID_PROPERTY_VALUE", normalizedName, value);
      return;
    }

    const important = normalizedPriority === "important";
    const longhands = this.#codec.shorthandLonghands(normalizedName);
    if (parsed.pendingSubstitution && longhands) {
      const pendingGroup: PendingSubstitutionGroup = {
        shorthand: normalizedName,
        observableValue: parsed.observableValue,
        safeValue: parsed.safeValue,
      };
      for (const longhand of longhands) {
        this.#commitRecord(
          longhand,
          { observableValue: "", safeValue: "", pendingSubstitution: true },
          important,
          pendingGroup,
        );
      }
      return;
    }

    const expansion = this.#codec.expandFourSide(normalizedName, parsed, important) ??
      this.#codec.expandShorthand(normalizedName, parsed, important);
    if (expansion) {
      for (const record of expansion) {
        this.#commitRecord(
          record.name,
          record,
          record.important,
          record.pendingGroup,
        );
      }
      return;
    }

    if (longhands) {
      this.#reportDiagnostic("UNSUPPORTED_SHORTHAND_VALUE", normalizedName, value);
      return;
    }

    this.#commitRecord(normalizedName, parsed, important);
  }

  removeProperty(name: string): string {
    const normalizedName = this.#codec.normalizeName(name);
    const longhands = this.#codec.shorthandLonghands(normalizedName);
    if (longhands) {
      const previousValue = this.getPropertyValue(normalizedName);
      const names = new Set([normalizedName, ...longhands]);
      for (let index = this.#records.length - 1; index >= 0; index -= 1) {
        const record = this.#records[index];
        if (record && names.has(record.name)) this.#records.splice(index, 1);
      }
      return previousValue;
    }

    const index = this.#records.findIndex(record => record.name === normalizedName);
    if (index === -1) return "";
    const [removed] = this.#records.splice(index, 1);
    return removed?.observableValue ?? "";
  }

  serialize(safe: boolean, indent: string, separator: string): string {
    const declarations: string[] = [];
    const writtenPendingGroups = new Set<PendingSubstitutionGroup>();
    const writtenStaticShorthands = new Set<string>();
    const recordsByName = new Map(
      this.#records.map(record => [record.name, record] as const),
    );
    const staticShorthandCandidates: Array<{
      name: string;
      longhands: readonly string[];
      value: string;
      important: boolean;
    }> = [];
    for (const name of this.#codec.staticShorthandNames()) {
      const longhands = this.#codec.shorthandLonghands(name);
      if (
        !longhands ||
        longhands.length > recordsByName.size ||
        longhands.some(longhand => !recordsByName.has(longhand))
      ) {
        continue;
      }

      const shorthand = this.#shorthand(name, safe, recordsByName);
      if (shorthand && shorthand.value !== "") {
        staticShorthandCandidates.push({ name, longhands, ...shorthand });
      }
    }
    staticShorthandCandidates.sort((left, right) =>
      right.longhands.length - left.longhands.length ||
      Number(left.name.startsWith("-")) - Number(right.name.startsWith("-")),
    );
    const staticShorthands = new Map<
      string,
      { name: string; value: string; important: boolean }
    >();
    const claimedLonghands = new Set<string>();
    for (const candidate of staticShorthandCandidates) {
      if (candidate.longhands.some(longhand => claimedLonghands.has(longhand))) continue;
      for (const longhand of candidate.longhands) {
        claimedLonghands.add(longhand);
        staticShorthands.set(longhand, candidate);
      }
    }

    for (const record of this.#records) {
      const pendingGroup = record.pendingGroup;
      if (pendingGroup) {
        const shorthand = this.#shorthand(pendingGroup.shorthand, safe);
        if (shorthand) {
          if (writtenPendingGroups.has(pendingGroup)) continue;
          declarations.push(
            `${indent}${pendingGroup.shorthand}: ${shorthand.value}${shorthand.important ? " !important" : ""};`,
          );
          writtenPendingGroups.add(pendingGroup);
          continue;
        }
      }

      let staticShorthandWritten = false;
      const staticShorthand = staticShorthands.get(record.name);
      if (staticShorthand) {
        if (!writtenStaticShorthands.has(staticShorthand.name)) {
          declarations.push(
            `${indent}${staticShorthand.name}: ${staticShorthand.value}${staticShorthand.important ? " !important" : ""};`,
          );
          writtenStaticShorthands.add(staticShorthand.name);
        }
        staticShorthandWritten = true;
      }
      if (staticShorthandWritten) continue;

      const name = record.name.startsWith("--")
        ? this.#codec.serializeIdentifier(record.name)
        : record.name;
      const value = safe ? record.safeValue : record.observableValue;
      declarations.push(
        `${indent}${name}: ${value}${record.important ? " !important" : ""};`,
      );
    }

    return declarations.join(separator);
  }

  #commitRecord(
    name: string,
    parsed: ParsedPropertyValue,
    important: boolean,
    pendingGroup: PendingSubstitutionGroup | null = null,
  ): void {
    const existing = this.#records.find(record => record.name === name);
    if (existing) {
      existing.observableValue = parsed.observableValue;
      existing.safeValue = parsed.safeValue;
      existing.pendingSubstitution = parsed.pendingSubstitution;
      existing.important = important;
      existing.pendingGroup = pendingGroup;
      return;
    }
    this.#records.push({
      name,
      observableValue: parsed.observableValue,
      safeValue: parsed.safeValue,
      pendingSubstitution: parsed.pendingSubstitution,
      important,
      pendingGroup,
    });
  }

  #shorthand(
    name: string,
    safe: boolean,
    recordsByName?: ReadonlyMap<string, DeclarationRecord>,
  ): { value: string; important: boolean } | null {
    const longhands = this.#codec.shorthandLonghands(name);
    if (!longhands) return null;
    const records = longhands.map(longhand =>
      recordsByName?.get(longhand) ??
      this.#records.find(record => record.name === longhand),
    );
    if (records.some(record => record === undefined)) return null;

    const first = records[0];
    if (!first || records.some(record => record?.important !== first.important)) return null;
    const pendingGroup = first.pendingGroup;
    if (
      pendingGroup?.shorthand === name &&
      records.every(record => record?.pendingGroup === pendingGroup)
    ) {
      return {
        value: safe ? pendingGroup.safeValue : pendingGroup.observableValue,
        important: first.important,
      };
    }
    if (records.some(record => record?.pendingGroup)) return null;
    if (this.#codec.isFourSideShorthand(name) && records.length === 4) {
      const [top, right, bottom, left] = records;
      if (!top || !right || !bottom || !left) return null;
      const values: [string, string, string, string] = [top, right, bottom, left]
        .map(record => safe ? record.safeValue : record.observableValue) as [
          string,
          string,
          string,
          string,
        ];
      return { value: compressFourSides(values), important: first.important };
    }

    const synthesized = this.#codec.synthesizeShorthand(
      name,
      records as DeclarationRecord[],
      safe,
    );
    if (synthesized !== null) {
      return { value: synthesized, important: first.important };
    }
    return null;
  }
}

function compressFourSides([top, right, bottom, left]: [string, string, string, string]): string {
  if (top === right && top === bottom && top === left) return top;
  if (top === bottom && right === left) return `${top} ${right}`;
  if (right === left) return `${top} ${right} ${bottom}`;
  return [top, right, bottom, left].join(" ");
}
