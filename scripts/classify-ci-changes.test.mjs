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
};

test("documentation changes run only documentation validation", () => {
    assert.deepEqual(classifyPaths(["README.md", "docs/api.md"]), { ...none, docs: true });
});

test("vendored source changes run only native validation", () => {
    assert.deepEqual(classifyPaths(["vendor/lightningcss/src/lib.rs"]), { ...none, native: true });
});

test("native bridge changes include the performance gate", () => {
    assert.deepEqual(classifyPaths(["crates/sheetom-native/src/lib.rs"]), {
        ...none,
        native: true,
        performance: true,
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
    });
});
