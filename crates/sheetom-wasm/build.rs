use std::{env, fs, path::PathBuf};

fn main() {
    let manifest_directory = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap_or_default());
    let identity_path = manifest_directory.join("../../engine-abi.json");
    println!("cargo:rerun-if-changed={}", identity_path.display());
    let identity_source = fs::read_to_string(identity_path).unwrap_or_else(|error| {
        panic!("could not read the generated Engine ABI Identity: {error}")
    });
    let identity: serde_json::Value =
        serde_json::from_str(&identity_source).unwrap_or_else(|error| {
            panic!("could not parse the generated Engine ABI Identity: {error}")
        });
    println!("cargo:rustc-env=SHEETOM_ENGINE_ABI_IDENTITY={identity}");
}
