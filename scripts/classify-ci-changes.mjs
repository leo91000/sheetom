import { execFileSync } from "node:child_process";
import { pathToFileURL } from "node:url";

const OUTPUT_NAMES = [
    "browser",
    "docs",
    "native",
    "package",
    "performance",
    "quality",
    "vendor",
    "wasm",
];

const isDocumentationPath = filePath =>
    filePath.endsWith(".md") ||
    filePath.startsWith("docs/") ||
    ["CONTEXT.md", "LICENSE", "SECURITY.md", "SUPPORT.md"].includes(filePath);

const isReleaseMetadataPath = filePath => filePath.startsWith(".changeset/");

const isNativePath = filePath =>
    filePath === "engine-abi.json" ||
    filePath === "Cargo.lock" ||
    filePath === "Cargo.toml" ||
    filePath === "rust-toolchain.toml" ||
    filePath.startsWith("crates/") ||
    filePath.startsWith("fuzz/") ||
    filePath.startsWith("native/") ||
    filePath.startsWith("packages/native-") ||
    filePath.startsWith("vendor/cssparser/") ||
    filePath.startsWith("vendor/lightningcss/") ||
    filePath === "scripts/check-native-public-corpus.mjs" ||
    filePath === "scripts/engine-abi.mjs" ||
    filePath === "scripts/generate-native-property-catalog.mjs" ||
    filePath === "scripts/native-crash-worker.mjs" ||
    filePath === "scripts/public-crash-worker.mjs" ||
    filePath.startsWith("scripts/sync-cargo-version") ||
    filePath.startsWith("scripts/build-native-") ||
    filePath.startsWith("scripts/collect-native-") ||
    filePath.startsWith("scripts/install-local-native-") ||
    filePath.startsWith("scripts/native-package-") ||
    filePath.startsWith("scripts/pack-native-") ||
    filePath.startsWith("scripts/sync-native-") ||
    filePath.startsWith("scripts/test-native-");

const isVendorPath = filePath =>
    filePath === "Cargo.lock" ||
    filePath === "Cargo.toml" ||
    filePath === "package.json" ||
    filePath === "rust-toolchain.toml" ||
    filePath.startsWith(".cargo/") ||
    filePath.startsWith("vendor/cssparser/") ||
    filePath.startsWith("vendor/lightningcss/");

const isWasmPath = filePath =>
    filePath === "engine-abi.json" ||
    filePath === "Cargo.lock" ||
    filePath === "Cargo.toml" ||
    filePath === "package.json" ||
    filePath === "package-lock.json" ||
    filePath === "rust-toolchain.toml" ||
    filePath.startsWith("crates/sheetom-core/") ||
    filePath.startsWith("crates/sheetom-wasm/") ||
    filePath.startsWith("packages/wasm/") ||
    filePath.startsWith("src/") ||
    filePath.startsWith("vendor/cssparser/") ||
    filePath.startsWith("vendor/lightningcss/") ||
    filePath.startsWith("scripts/build-wasm-") ||
    filePath.startsWith("scripts/finalize-wasm-") ||
    filePath.startsWith("scripts/sync-wasm-") ||
    filePath.startsWith("scripts/test-wasm-");

const isAutomationPath = filePath =>
    filePath.startsWith(".github/") ||
    filePath === "scripts/classify-ci-changes.mjs" ||
    filePath === "scripts/classify-ci-changes.test.mjs";

const isBrowserPath = filePath =>
    filePath.startsWith("src/") ||
    filePath.startsWith("tests/browser/") ||
    filePath.startsWith("tests/conformance/") ||
    filePath.startsWith("compatibility/") ||
    filePath.startsWith("conformance/") ||
    filePath.startsWith("scripts/browser-") ||
    filePath === "scripts/generate-chromium-properties.mjs" ||
    filePath === "scripts/generate-native-grammar-inventory.mjs" ||
    filePath.startsWith("scripts/generate-property-value-") ||
    filePath === "scripts/generate-webref-property-branches.mjs" ||
    filePath === "scripts/lib/webref-syntax-samples.mjs" ||
    filePath === "scripts/check-property-value-matrix.mjs" ||
    filePath.startsWith("scripts/generate-shorthand-") ||
    ["package.json", "package-lock.json", "vitest.config.ts"].includes(filePath);

const isPackagePath = filePath =>
    filePath.startsWith("src/") ||
    filePath === "scripts/verify-release.mjs" ||
    filePath.startsWith("scripts/test-package") ||
    filePath.startsWith("scripts/test-tarball") ||
    filePath.startsWith("packages/native-") ||
    filePath.startsWith("packages/wasm/") ||
    filePath.startsWith("scripts/build-wasm-") ||
    filePath.startsWith("scripts/finalize-wasm-") ||
    filePath.startsWith("scripts/sync-wasm-") ||
    filePath.startsWith("scripts/test-wasm-") ||
    ["package.json", "package-lock.json", "tsdown.config.ts", "tsconfig.json"].includes(filePath);

const isPerformancePath = filePath =>
    filePath.startsWith("src/") ||
    filePath.startsWith("crates/sheetom-core/") ||
    filePath.startsWith("crates/sheetom-native/src/") ||
    filePath.startsWith("vendor/cssparser/src/") ||
    filePath.startsWith("vendor/lightningcss/src/") ||
    filePath.startsWith("scripts/benchmark") ||
    filePath.startsWith("scripts/compare-benchmarks") ||
    ["package.json", "package-lock.json", "tsdown.config.ts"].includes(filePath);

const fullClassification = () => Object.fromEntries(OUTPUT_NAMES.map(name => [name, true]));

export function classifyPaths(filePaths, { forceFull = false } = {}) {
    if (forceFull || filePaths.length === 0 || filePaths.some(isAutomationPath)) {
        return fullClassification();
    }

    const knownPaths = filePaths.filter(
        filePath => isDocumentationPath(filePath) ||
            isReleaseMetadataPath(filePath) ||
            isNativePath(filePath),
    );
    const hasUnknownPath = knownPaths.length !== filePaths.length;
    const native = filePaths.some(isNativePath);
    const quality = hasUnknownPath;

    return {
        browser: filePaths.some(isBrowserPath),
        docs: quality || filePaths.some(isDocumentationPath),
        native,
        package: native || filePaths.some(isPackagePath),
        performance: filePaths.some(isPerformancePath),
        quality,
        vendor: filePaths.some(isVendorPath),
        wasm: filePaths.some(isWasmPath),
    };
}

function changedPaths(base, head) {
    if (!base || /^0+$/u.test(base)) {
        return [];
    }

    return execFileSync("git", ["diff", "--name-only", base, head], { encoding: "utf8" })
        .split("\n")
        .filter(Boolean);
}

function run() {
    const forceFull = process.env.SHEETOM_CI_FORCE_FULL === "1";
    const filePaths = changedPaths(process.env.SHEETOM_CI_BASE, process.env.SHEETOM_CI_HEAD ?? "HEAD");
    const classification = classifyPaths(filePaths, { forceFull });

    for (const name of OUTPUT_NAMES) {
        console.log(`${name}=${classification[name]}`);
    }
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
    run();
}
