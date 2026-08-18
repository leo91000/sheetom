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
    ...arguments_: [number, number, number, number, number, string?]
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
    createDeclarationState: (context, ...arguments_) => new addon.NativeDeclarationState(
      context,
      ...arguments_,
    ),
  };
}

export const nativeEngineBinding = loadBinding();
