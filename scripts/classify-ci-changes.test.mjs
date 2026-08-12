import assert from "node:assert/strict";
import test from "node:test";

import { classifyPaths } from "./classify-ci-changes.mjs";

const none = {
    browser: false,
    docs: false,
    native: false,
    package: false,
    performance: false,
    quality: false,
    vendor: false,
};

test("documentation changes run only documentation validation", () => {
    assert.deepEqual(classifyPaths(["README.md", "docs/api.md"]), { ...none, docs: true });
});

test("vendored source changes rebuild native validation and package artifacts", () => {
    assert.deepEqual(classifyPaths(["vendor/cssparser/src/parser.rs", "vendor/lightningcss/src/lib.rs", "fuzz/fuzz_targets/declaration_block.rs"]), {
        ...none,
        native: true,
        package: true,
        performance: true,
        vendor: true,
    });
});

test("native engine changes rebuild native validation and package artifacts", () => {
    assert.deepEqual(classifyPaths(["crates/sheetom-native/src/lib.rs"]), {
        ...none,
        native: true,
        package: true,
        performance: true,
    });
});

test("the generated native property catalog does not trigger browser jobs", () => {
    assert.deepEqual(
        classifyPaths([
            "scripts/generate-native-property-catalog.mjs",
            "crates/sheetom-core/src/generated/chromium_properties.rs",
        ]),
        { ...none, native: true, package: true, performance: true },
    );
});

test("native packaging scripts rebuild the native matrix and package artifact", () => {
    for (const filePath of [
        "scripts/collect-native-artifacts.mjs",
        "scripts/pack-native-packages.mjs",
        "packages/native-linux-x64-gnu/package.json",
    ]) {
        assert.deepEqual(classifyPaths([filePath]), {
            ...none,
            native: true,
            package: true,
        });
    }
});

test("release version synchronization rebuilds native package evidence", () => {
    assert.deepEqual(classifyPaths(["scripts/sync-cargo-version.mjs"]), {
        ...none,
        native: true,
        package: true,
    });
});

test("the generated Engine ABI Identity rebuilds native and package artifacts", () => {
    assert.deepEqual(classifyPaths(["engine-abi.json", "scripts/engine-abi.mjs"]), {
        ...none,
        native: true,
        package: true,
    });
});

test("native and public crash workers run the subprocess safety gate", () => {
    assert.deepEqual(
        classifyPaths(["scripts/native-crash-worker.mjs", "scripts/public-crash-worker.mjs"]),
        { ...none, native: true, package: true },
    );
});

test("browser-backed grammar generators still run browser validation", () => {
    assert.deepEqual(classifyPaths([
        "scripts/generate-native-grammar-inventory.mjs",
        "scripts/generate-property-value-observations.mjs",
        "scripts/check-property-value-matrix.mjs",
    ]), {
        ...none,
        browser: true,
        docs: true,
        quality: true,
        vendor: false,
    });
});

test("Webref grammar sampling reruns browser evidence without native or package matrices", () => {
    assert.deepEqual(classifyPaths([
        "scripts/generate-webref-property-branches.mjs",
        "scripts/lib/webref-syntax-samples.mjs",
    ]), {
        ...none,
        browser: true,
        docs: true,
        quality: true,
        vendor: false,
    });
});

test("public runtime changes run every relevant JavaScript gate", () => {
    assert.deepEqual(classifyPaths(["src/index.ts"]), {
        browser: true,
        docs: true,
        native: false,
        package: true,
        performance: true,
        quality: true,
        vendor: false,
    });
});

test("workflow and classifier changes fail safe to the complete matrix", () => {
    assert.deepEqual(classifyPaths([".github/workflows/ci.yml"]), {
        browser: true,
        docs: true,
        native: true,
        package: true,
        performance: true,
        quality: true,
        vendor: true,
    });
});

test("scheduled runs force the complete matrix", () => {
    assert.deepEqual(classifyPaths(["README.md"], { forceFull: true }), {
        browser: true,
        docs: true,
        native: true,
        package: true,
        performance: true,
        quality: true,
        vendor: true,
    });
});

test("root Rust and package command changes keep vendored tests enabled", () => {
    for (const filePath of ["Cargo.toml", "Cargo.lock", "rust-toolchain.toml", "package.json"]) {
        assert.equal(classifyPaths([filePath]).vendor, true, filePath);
    }
});
