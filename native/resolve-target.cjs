"use strict";

const { readFileSync } = require("node:fs");

const NATIVE_TARGETS = Object.freeze([
  { triple: "aarch64-apple-darwin", target: "darwin-arm64" },
  { triple: "x86_64-apple-darwin", target: "darwin-x64" },
  { triple: "aarch64-pc-windows-msvc", target: "win32-arm64-msvc" },
  { triple: "i686-pc-windows-msvc", target: "win32-ia32-msvc" },
  { triple: "x86_64-pc-windows-msvc", target: "win32-x64-msvc" },
  { triple: "aarch64-unknown-linux-gnu", target: "linux-arm64-gnu" },
  { triple: "armv7-unknown-linux-gnueabihf", target: "linux-arm-gnueabihf" },
  { triple: "powerpc64le-unknown-linux-gnu", target: "linux-ppc64-gnu" },
  { triple: "s390x-unknown-linux-gnu", target: "linux-s390x-gnu" },
  { triple: "x86_64-unknown-linux-gnu", target: "linux-x64-gnu" },
  { triple: "aarch64-unknown-linux-musl", target: "linux-arm64-musl" },
  { triple: "armv7-unknown-linux-musleabihf", target: "linux-arm-musleabihf" },
  { triple: "x86_64-unknown-linux-musl", target: "linux-x64-musl" },
].map(entry => Object.freeze({
  ...entry,
  artifact: `sheetom-native.${entry.target}.node`,
  packageName: `@sheetom/native-${entry.target}`,
})));

const SUPPORTED_TARGETS = new Set(NATIVE_TARGETS.map(entry => entry.target));
const TARGET_BY_TRIPLE = new Map(NATIVE_TARGETS.map(entry => [entry.triple, entry]));
const TARGET_BY_NAME = new Map(NATIVE_TARGETS.map(entry => [entry.target, entry]));

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
  {
    platform = process.platform,
    arch = process.arch,
    armVersion = process.config?.variables?.arm_version,
  } = {},
  dependencies = {},
) {
  if (arch === "arm" && armVersion !== undefined) {
    const normalizedArmVersion = Number(armVersion);
    if (!Number.isInteger(normalizedArmVersion) || normalizedArmVersion < 7) return null;
  }
  let target;
  if (platform === "linux") {
    const libc = detectLinuxLibc(dependencies);
    if (!libc) return null;
    const abi = arch === "arm" ? (libc === "musl" ? "musleabihf" : "gnueabihf") : libc;
    target = `linux-${arch}-${abi}`;
  } else if (platform === "win32") {
    target = `win32-${arch}-msvc`;
  } else {
    target = `${platform}-${arch}`;
  }

  return SUPPORTED_TARGETS.has(target) ? target : null;
}

module.exports = {
  detectLinuxLibc,
  NATIVE_TARGETS,
  resolveTarget,
  SUPPORTED_TARGETS,
  TARGET_BY_NAME,
  TARGET_BY_TRIPLE,
};
