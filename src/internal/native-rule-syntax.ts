import { nativeBinding } from "./native-binding.js";

export interface NativeContainerPrelude {
  conditionText: string;
  name: string;
  query: string;
}

export interface NativeScopePrelude {
  start: string | null;
  end: string | null;
}

export function normalizeNativeSelector(source: string): string | null {
  return callNative(() => nativeBinding.normalizeSelector(source));
}

export function normalizeNativeMedia(source: string): string | null {
  return callNative(() => nativeBinding.normalizeMedia(source));
}

export function normalizeNativeSupports(source: string): string | null {
  return callNative(() => nativeBinding.normalizeSupports(source));
}

export function parseNativeContainerPrelude(source: string): NativeContainerPrelude | null {
  const parsed = callNative(() => nativeBinding.parseContainerPreludeJson(source));
  if (parsed === null) return null;
  try {
    const value: unknown = JSON.parse(parsed);
    if (typeof value !== "object" || value === null) return null;
    const candidate = value as Partial<NativeContainerPrelude>;
    if (
      typeof candidate.conditionText !== "string"
      || typeof candidate.name !== "string"
      || typeof candidate.query !== "string"
    ) return null;
    return candidate as NativeContainerPrelude;
  } catch {
    return null;
  }
}

export function parseNativeScopePrelude(source: string): NativeScopePrelude | null {
  const parsed = callNative(() => nativeBinding.parseScopePreludeJson(source));
  if (parsed === null) return null;
  try {
    const value: unknown = JSON.parse(parsed);
    if (typeof value !== "object" || value === null) return null;
    const candidate = value as Partial<NativeScopePrelude>;
    if (
      candidate.start !== null && typeof candidate.start !== "string"
      || candidate.end !== null && typeof candidate.end !== "string"
    ) return null;
    return candidate as NativeScopePrelude;
  } catch {
    return null;
  }
}

function callNative(operation: () => string): string | null {
  try {
    return operation();
  } catch {
    return null;
  }
}
