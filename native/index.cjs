"use strict";

const { resolveTarget, TARGET_BY_NAME } = require("./resolve-target.cjs");

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

const metadata = TARGET_BY_NAME.get(target);
if (!metadata) {
  throw bindingError(
    "SHEETOM_NATIVE_PLATFORM_UNSUPPORTED",
    `SheetOM has no native package metadata for ${target}.`,
  );
}

try {
  module.exports = require(metadata.packageName);
} catch (cause) {
  if (cause?.code === "MODULE_NOT_FOUND" && cause.message?.includes(metadata.packageName)) {
    throw bindingError(
      "SHEETOM_NATIVE_BINDING_MISSING",
      `SheetOM requires ${metadata.packageName} for ${target}. ` +
        "Reinstall without omitting optional dependencies for this exact operating system, CPU, and libc.",
      cause,
    );
  }
  throw bindingError(
    "SHEETOM_NATIVE_BINDING_LOAD_FAILED",
    `SheetOM found but could not load ${metadata.packageName} for ${target}.`,
    cause,
  );
}
