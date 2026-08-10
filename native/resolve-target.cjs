"use strict";

const { readFileSync } = require("node:fs");

const SUPPORTED_TARGETS = new Set([
  "darwin-arm64",
  "darwin-x64",
  "linux-arm64-gnu",
  "linux-arm64-musl",
  "linux-x64-gnu",
  "linux-x64-musl",
  "win32-arm64-msvc",
  "win32-x64-msvc",
]);

function detectLinuxLibc({ report = process.report, readFile = readFileSync } = {}) {
  try {
    const runtimeReport = report?.getReport?.();
    if (runtimeReport?.header?.glibcVersionRuntime) return "gnu";
    if (
      runtimeReport?.sharedObjects?.some(
        filename => filename.includes("libc.musl-") || filename.includes("ld-musl-"),
      )
    ) {
      return "musl";
    }
  } catch {
    // Some Node-compatible runtimes expose an incomplete process.report.
  }

  try {
    return readFile("/usr/bin/ldd", "utf8").includes("musl") ? "musl" : "gnu";
  } catch {
    return null;
  }
}

function resolveTarget(
  { platform = process.platform, arch = process.arch } = {},
  dependencies = {},
) {
  let target;
  if (platform === "linux") {
    const libc = detectLinuxLibc(dependencies);
    if (!libc) return null;
    target = `linux-${arch}-${libc}`;
  } else if (platform === "win32") {
    target = `win32-${arch}-msvc`;
  } else {
    target = `${platform}-${arch}`;
  }

  return SUPPORTED_TARGETS.has(target) ? target : null;
}

module.exports = { detectLinuxLibc, resolveTarget, SUPPORTED_TARGETS };
