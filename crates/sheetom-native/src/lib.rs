#![deny(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

#[cfg(panic = "abort")]
compile_error!("sheetom-native must be compiled with panic=unwind");

use napi_derive::napi;
use sheetom_core::{
    canonicalize_declaration_block as canonicalize, DeclarationState, MutationOutcome,
    ENGINE_REVISION,
};

#[napi]
pub struct NativeDeclarationState {
    state: DeclarationState,
}

impl Default for NativeDeclarationState {
    fn default() -> Self {
        Self {
            state: DeclarationState::new(),
        }
    }
}

#[napi]
impl NativeDeclarationState {
    #[napi(constructor)]
    pub fn new() -> Self {
        Self::default()
    }

    #[napi(getter)]
    pub fn length(&self) -> u32 {
        u32::try_from(self.state.len()).unwrap_or(u32::MAX)
    }

    #[napi]
    pub fn item(&self, index: u32) -> String {
        self.state.item(index as usize).to_owned()
    }

    #[napi]
    pub fn get_property_value(&self, name: String) -> String {
        self.state.get_property_value(&name)
    }

    #[napi]
    pub fn get_property_priority(&self, name: String) -> String {
        self.state.get_property_priority(&name).to_owned()
    }

    #[napi]
    pub fn set_property(&mut self, name: String, value: String, priority: String) -> String {
        mutation_outcome_name(self.state.set_property(&name, &value, &priority)).to_owned()
    }

    #[napi]
    pub fn remove_property(&mut self, name: String) -> String {
        self.state.remove_property(&name)
    }

    #[napi]
    pub fn serialize_longhands(&self) -> String {
        self.state.serialize_longhands()
    }

    #[napi]
    pub fn replace_css_text(&mut self, source: String) {
        self.state.replace_css_text(&source);
    }

    #[napi]
    pub fn clear(&mut self) {
        self.state.clear();
    }

    #[napi(getter)]
    pub fn css_text(&self) -> String {
        self.state.css_text()
    }

    #[napi]
    pub fn serialize_safe(&self) -> String {
        self.state.serialize_safe()
    }

    #[napi]
    pub fn serialize_formatted(&self, safe: bool, indent: String, separator: String) -> String {
        self.state.serialize_formatted(safe, &indent, &separator)
    }
}

fn mutation_outcome_name(outcome: MutationOutcome) -> &'static str {
    match outcome {
        MutationOutcome::Applied => "applied",
        MutationOutcome::InvalidName => "invalid-name",
        MutationOutcome::InvalidPriority => "invalid-priority",
        MutationOutcome::InvalidValue => "invalid-value",
        MutationOutcome::UnsupportedShorthand => "unsupported-shorthand",
    }
}

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
