#![deny(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

#[cfg(panic = "abort")]
compile_error!("sheetom-native must be compiled with panic=unwind");

use napi_derive::napi;
use sheetom_core::{canonicalize_declaration_block as canonicalize, ENGINE_REVISION};

/// Identifies the exact parser baseline compiled into the native addon.
#[napi]
pub fn native_engine_revision() -> &'static str {
    ENGINE_REVISION
}

/// Exercises the native string boundary while the engine runs in shadow mode.
///
/// This deliberately accepts and returns owned strings. Lightning CSS AST nodes
/// never cross Node-API and therefore cannot be deserialized back into Rust.
#[napi]
pub fn canonicalize_declaration_block(source: String) -> napi::Result<String> {
    canonicalize(&source).map_err(|error| napi::Error::from_reason(error.to_string()))
}
