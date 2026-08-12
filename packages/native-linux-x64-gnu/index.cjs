"use strict";

const { readdirSync } = require("node:fs");
const path = require("node:path");

const artifacts = readdirSync(__dirname).filter(entry => entry.endsWith(".node"));
if (artifacts.length !== 1) {
  const error = new Error(
    `SheetOM native platform package must contain exactly one addon; found ${artifacts.length}.`,
  );
  error.name = "SheetOMNativeBindingError";
  error.code = "SHEETOM_NATIVE_PACKAGE_INVALID";
  throw error;
}

module.exports = require(path.join(__dirname, artifacts[0]));
