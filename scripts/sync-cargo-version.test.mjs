import assert from "node:assert/strict";
import test from "node:test";

import {
  replaceCargoLockVersions,
  replaceCargoPackageVersion,
} from "./sync-cargo-version.mjs";

test("release versioning updates only the Cargo package version", () => {
  const source = `[package]
name = "sheetom-core"
version = "0.1.0-rc.5"
edition.workspace = true

[dependencies]
example = { version = "1.2.3" }
`;

  assert.equal(
    replaceCargoPackageVersion(source, "0.1.0-rc.6"),
    source.replace('version = "0.1.0-rc.5"', 'version = "0.1.0-rc.6"'),
  );
});

test("release versioning rejects ambiguous package metadata", () => {
  assert.throws(
    () => replaceCargoPackageVersion("[dependencies]\nexample = \"1\"\n", "0.1.0"),
    /no \[package\] section/u,
  );
  assert.throws(
    () => replaceCargoPackageVersion("[package]\nname = \"sheetom\"\n", "0.1.0"),
    /exactly one version/u,
  );
});

test("release versioning updates both workspace entries in Cargo.lock", () => {
  const source = `[[package]]
name = "dependency"
version = "1.2.3"

[[package]]
name = "sheetom-core"
version = "0.1.0-rc.5"
dependencies = ["dependency"]

[[package]]
name = "sheetom-native"
version = "0.1.0-rc.5"
dependencies = ["sheetom-core"]
`;
  const updated = replaceCargoLockVersions(source, "0.1.0-rc.6");
  assert.match(updated, /name = "dependency"\nversion = "1\.2\.3"/u);
  assert.equal(updated.match(/version = "0\.1\.0-rc\.6"/gu)?.length, 2);
});
