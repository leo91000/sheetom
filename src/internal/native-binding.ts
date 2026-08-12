import { createRequire } from "node:module";
import path from "node:path";
import { fileURLToPath } from "node:url";

import {
  type EngineBinding,
  type EngineDeclarationStateHandle,
  validateEngineBindingIdentity,
} from "./engine-binding.js";

interface NativeAddonBinding extends Omit<EngineBinding, "createDeclarationState"> {
  NativeDeclarationState: new (
    context?: "style" | "font-face" | "function",
    ...budget: [number, number, number, number, number]
  ) => EngineDeclarationStateHandle;
}

function loadBinding(): EngineBinding {
  const moduleDirectory = path.dirname(fileURLToPath(import.meta.url));
  const packageRoot = path.basename(moduleDirectory) === "dist"
    ? path.dirname(moduleDirectory)
    : path.resolve(moduleDirectory, "../..");
  const require = createRequire(import.meta.url);
  const addon = require(path.join(packageRoot, "native", "index.cjs")) as NativeAddonBinding;
  validateEngineBindingIdentity(addon);
  return {
    ...addon,
    createDeclarationState: (context, ...budget) => new addon.NativeDeclarationState(
      context,
      ...budget,
    ),
  };
}

export const nativeEngineBinding = loadBinding();
