import type { EngineBinding } from "../../../src/internal/engine-binding.js";
import { takeEngineBinding } from "./binding-registry.js";

export const engineBinding: EngineBinding = takeEngineBinding(import.meta.url);
