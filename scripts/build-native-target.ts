import { execFileSync } from "node:child_process";
import { access, copyFile, mkdir, stat } from "node:fs/promises";
import { createRequire } from "node:module";
import path from "node:path";
import { fileURLToPath } from "node:url";

const repositoryRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const require = createRequire(import.meta.url);
const { TARGET_BY_TRIPLE } = require("../native/resolve-target.cjs");
const target = process.argv[2];

const targets = new Map([
  ["aarch64-apple-darwin", { platform: "darwin", arch: "arm64", suffix: "darwin-arm64" }],
  [
    "x86_64-apple-darwin",
    {
      platform: "darwin",
      arch: "x64",
      buildArchitectures: ["arm64", "x64"],
      suffix: "darwin-x64",
    },
  ],
  [
    "aarch64-pc-windows-msvc",
    {
      platform: "win32",
      arch: "arm64",
      buildArchitectures: ["arm64", "x64"],
      suffix: "win32-arm64-msvc",
    },
  ],
  [
    "i686-pc-windows-msvc",
    {
      platform: "win32",
      arch: "ia32",
      buildArchitectures: ["x64"],
      suffix: "win32-ia32-msvc",
    },
  ],
  ["x86_64-pc-windows-msvc", { platform: "win32", arch: "x64", suffix: "win32-x64-msvc" }],
  ["aarch64-unknown-linux-gnu", { platform: "linux", arch: "arm64", suffix: "linux-arm64-gnu", napiCross: true }],
  ["armv7-unknown-linux-gnueabihf", { platform: "linux", arch: "arm", suffix: "linux-arm-gnueabihf", napiCross: true }],
  ["powerpc64le-unknown-linux-gnu", { platform: "linux", arch: "ppc64", suffix: "linux-ppc64-gnu", napiCross: true }],
  ["s390x-unknown-linux-gnu", { platform: "linux", arch: "s390x", suffix: "linux-s390x-gnu", napiCross: true }],
  ["x86_64-unknown-linux-gnu", { platform: "linux", arch: "x64", suffix: "linux-x64-gnu", napiCross: true }],
  ["aarch64-unknown-linux-musl", { platform: "linux", arch: "arm64", suffix: "linux-arm64-musl" }],
  ["armv7-unknown-linux-musleabihf", { platform: "linux", arch: "arm", suffix: "linux-arm-musleabihf" }],
  ["x86_64-unknown-linux-musl", { platform: "linux", arch: "x64", suffix: "linux-x64-musl" }],
]);

const configuration = targets.get(target);
if (!configuration) {
  throw new Error(`Unsupported native build target: ${target ?? "<missing>"}`);
}
const buildArchitectures = configuration.buildArchitectures ?? [configuration.arch];
if (
  process.platform !== configuration.platform ||
  !buildArchitectures.includes(process.arch)
) {
  throw new Error(
    `Target ${target} must be built on ${configuration.platform}/` +
      `${buildArchitectures.join(" or ")}, ` +
      `not ${process.platform}/${process.arch}`,
  );
}
if (
  configuration.suffix.endsWith("-musl") &&
  process.report?.getReport?.()?.header?.glibcVersionRuntime
) {
  throw new Error(`Target ${target} must be built inside a musl runtime`);
}

const arguments_ = [
  path.join(repositoryRoot, "node_modules", "@napi-rs", "cli", "cli.mjs"),
  "build",
  "--platform",
  "--release",
  "--package",
  "sheetom-native",
  "--manifest-path",
  "crates/sheetom-native/Cargo.toml",
  "--output-dir",
  "native",
  "--no-js",
  "--dts",
  "sheetom-native.d.ts",
  "--target",
  target,
];
if (configuration.napiCross) arguments_.push("--use-napi-cross");

execFileSync(process.execPath, arguments_, { cwd: repositoryRoot, stdio: "inherit" });

const artifact = path.join(
  repositoryRoot,
  "native",
  `sheetom-native.${configuration.suffix}.node`,
);
await access(artifact);
const artifactSize = (await stat(artifact)).size;
if (artifactSize < 1_000_000) {
  throw new Error(`Native artifact is unexpectedly small: ${artifact} (${artifactSize} bytes)`);
}

const targetMetadata = TARGET_BY_TRIPLE.get(target);
if (!targetMetadata || targetMetadata.artifact !== path.basename(artifact)) {
  throw new Error(`Native package registry is inconsistent for build target ${target}`);
}
const packageDirectory = path.join(
  repositoryRoot,
  "packages",
  `native-${targetMetadata.target}`,
);
await mkdir(packageDirectory, { recursive: true });
await copyFile(artifact, path.join(packageDirectory, targetMetadata.artifact));

console.log(`Built ${path.basename(artifact)} (${artifactSize} bytes).`);
