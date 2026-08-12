import type { EngineBinding } from "./engine-binding.js";
import { nativeEngineBinding } from "./native-binding.js";

/** The synchronous root package uses the native adapter exclusively. */
export const engineBinding: EngineBinding = nativeEngineBinding;
