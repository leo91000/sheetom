import { nativeBinding } from "./native-binding.js";

export interface NativeCounterStyleDescriptor {
  name: string;
  value: string;
}

export interface NativeCounterStyleName {
  name: string;
  serialized: string;
}

export function parseNativeCounterStyleName(source: string): NativeCounterStyleName | null {
  try {
    const encoded = nativeBinding.parseCounterStyleNameJson(source);
    if (encoded === null) return null;
    const parsed: unknown = JSON.parse(encoded);
    if (typeof parsed !== "object" || parsed === null) return null;
    const candidate = parsed as Partial<NativeCounterStyleName>;
    if (typeof candidate.name !== "string" || typeof candidate.serialized !== "string") return null;
    return candidate as NativeCounterStyleName;
  } catch {
    return null;
  }
}

export function serializeNativeIdentifier(value: string): string {
  return nativeBinding.serializeIdentifierValue(value);
}

export function serializeNativeFontFamily(value: string): string {
  return nativeBinding.serializeFontFamilyValue(value);
}

export function parseNativeCounterStyleDescriptor(
  name: string,
  value: string,
): string | null {
  return nativeBinding.parseCounterStyleDescriptorValue(name, value);
}

export function parseNativeCounterStyleDescriptors(
  source: string,
): NativeCounterStyleDescriptor[] {
  try {
    const parsed: unknown = JSON.parse(nativeBinding.parseCounterStyleDescriptorsJson(source));
    if (!Array.isArray(parsed)) return [];
    const descriptors: NativeCounterStyleDescriptor[] = [];
    for (const value of parsed) {
      if (typeof value !== "object" || value === null) continue;
      const candidate = value as Partial<NativeCounterStyleDescriptor>;
      if (typeof candidate.name !== "string" || typeof candidate.value !== "string") continue;
      descriptors.push(candidate as NativeCounterStyleDescriptor);
    }
    return descriptors;
  } catch {
    return [];
  }
}
