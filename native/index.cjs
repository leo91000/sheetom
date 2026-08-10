"use strict";

const { existsSync } = require("node:fs");
const path = require("node:path");
const { resolveTarget } = require("./resolve-target.cjs");

function bindingError(code, message, cause) {
  const error = new Error(message, cause ? { cause } : undefined);
  error.name = "SheetOMNativeBindingError";
  error.code = code;
  return error;
}

const target = resolveTarget();
if (!target) {
  throw bindingError(
    "SHEETOM_NATIVE_PLATFORM_UNSUPPORTED",
    `SheetOM has no native binding for ${process.platform}/${process.arch}.`,
  );
}

const bindingPath = `./sheetom-native.${target}.node`;
const absoluteBindingPath = path.join(__dirname, bindingPath);
if (!existsSync(absoluteBindingPath)) {
  throw bindingError(
    "SHEETOM_NATIVE_BINDING_MISSING",
    `SheetOM's ${target} native binding is missing from ${absoluteBindingPath}. ` +
      "Reinstall the package for this exact operating system, CPU, and libc.",
  );
}

try {
  module.exports = require(bindingPath);
} catch (cause) {
  throw bindingError(
    "SHEETOM_NATIVE_BINDING_LOAD_FAILED",
    `SheetOM found but could not load its ${target} native binding at ${absoluteBindingPath}.`,
    cause,
  );
}
