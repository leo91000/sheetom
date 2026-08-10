import { execFileSync } from "node:child_process";
import { access, stat } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const repositoryRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const target = process.argv[2];

const targets = new Map([
  ["aarch64-apple-darwin", { platform: "darwin", arch: "arm64", suffix: "darwin-arm64" }],
  ["x86_64-apple-darwin", { platform: "darwin", arch: "x64", suffix: "darwin-x64" }],
  ["aarch64-pc-windows-msvc", { platform: "win32", arch: "arm64", suffix: "win32-arm64-msvc" }],
  ["x86_64-pc-windows-msvc", { platform: "win32", arch: "x64", suffix: "win32-x64-msvc" }],
  ["aarch64-unknown-linux-gnu", { platform: "linux", arch: "arm64", suffix: "linux-arm64-gnu", napiCross: true }],
  ["x86_64-unknown-linux-gnu", { platform: "linux", arch: "x64", suffix: "linux-x64-gnu", napiCross: true }],
  ["aarch64-unknown-linux-musl", { platform: "linux", arch: "arm64", suffix: "linux-arm64-musl" }],
  ["x86_64-unknown-linux-musl", { platform: "linux", arch: "x64", suffix: "linux-x64-musl" }],
]);

const configuration = targets.get(target);
if (!configuration) {
  throw new Error(`Unsupported native build target: ${target ?? "<missing>"}`);
}
if (process.platform !== configuration.platform || process.arch !== configuration.arch) {
  throw new Error(
    `Target ${target} must be built on ${configuration.platform}/${configuration.arch}, ` +
      `not ${process.platform}/${process.arch}`,
  );
}

execFileSync("rustup", ["target", "add", target], { cwd: repositoryRoot, stdio: "inherit" });

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

console.log(`Built ${path.basename(artifact)} (${artifactSize} bytes).`);
