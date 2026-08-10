#![deny(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

#[cfg(panic = "abort")]
compile_error!("sheetom-native must be compiled with panic=unwind");

use napi_derive::napi;
use sheetom_core::{
    canonicalize_declaration_block as canonicalize, inspect_property, normalize_media_text,
    normalize_selector_text, normalize_supports_text, parse_container_prelude,
    parse_counter_style_descriptor, parse_counter_style_descriptors, parse_counter_style_name,
    parse_recovered_rule_tree, parse_rule_tree, parse_scope_prelude, parse_stylesheet_tree,
    scan_top_level_rules, DeclarationContext, DeclarationState, MutationOutcome, ENGINE_REVISION,
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
    pub fn new(context: Option<String>) -> napi::Result<Self> {
        let context = match context.as_deref() {
            None | Some("style") => DeclarationContext::Style,
            Some("font-face") => DeclarationContext::FontFace,
            Some(context) => {
                return Err(napi::Error::from_reason(format!(
                    "SHEETOM_DECLARATION_CONTEXT: unsupported declaration context {context}"
                )))
            }
        };
        Ok(Self {
            state: DeclarationState::new_with_context(context),
        })
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

/// Parses exactly one rule and returns an owned, parser-independent JSON DTO.
#[napi]
pub fn parse_rule_tree_json(source: String) -> napi::Result<String> {
    let parsed =
        parse_rule_tree(&source).map_err(|error| napi::Error::from_reason(error.to_string()))?;
    serde_json::to_string(&parsed).map_err(|error| napi::Error::from_reason(error.to_string()))
}

/// Parses exactly one rule with browser-style declaration recovery.
#[napi]
pub fn parse_recovered_rule_tree_json(source: String) -> napi::Result<String> {
    let parsed = parse_recovered_rule_tree(&source)
        .map_err(|error| napi::Error::from_reason(error.to_string()))?;
    serde_json::to_string(&parsed).map_err(|error| napi::Error::from_reason(error.to_string()))
}

#[napi]
pub fn normalize_selector(source: String) -> napi::Result<String> {
    normalize_selector_text(&source).map_err(|error| napi::Error::from_reason(error.to_string()))
}

#[napi]
pub fn normalize_media(source: String) -> napi::Result<String> {
    normalize_media_text(&source).map_err(|error| napi::Error::from_reason(error.to_string()))
}

#[napi]
pub fn normalize_supports(source: String) -> napi::Result<String> {
    normalize_supports_text(&source).map_err(|error| napi::Error::from_reason(error.to_string()))
}

#[napi]
pub fn parse_container_prelude_json(source: String) -> napi::Result<String> {
    let parsed = parse_container_prelude(&source)
        .map_err(|error| napi::Error::from_reason(error.to_string()))?;
    serde_json::to_string(&parsed).map_err(|error| napi::Error::from_reason(error.to_string()))
}

#[napi]
pub fn parse_scope_prelude_json(source: String) -> napi::Result<String> {
    let parsed = parse_scope_prelude(&source)
        .map_err(|error| napi::Error::from_reason(error.to_string()))?;
    serde_json::to_string(&parsed).map_err(|error| napi::Error::from_reason(error.to_string()))
}

#[napi]
pub fn parse_counter_style_descriptor_value(
    name: String,
    value: String,
) -> napi::Result<Option<String>> {
    inspect_property("--sheetom-counter-style", &value)
        .map_err(|error| napi::Error::from_reason(error.to_string()))?;
    Ok(parse_counter_style_descriptor(&name, &value))
}

#[napi]
pub fn parse_counter_style_descriptors_json(source: String) -> napi::Result<String> {
    canonicalize(&source).map_err(|error| napi::Error::from_reason(error.to_string()))?;
    serde_json::to_string(&parse_counter_style_descriptors(&source))
        .map_err(|error| napi::Error::from_reason(error.to_string()))
}

#[napi]
pub fn parse_counter_style_name_json(source: String) -> napi::Result<Option<String>> {
    inspect_property("--sheetom-counter-style-name", &source)
        .map_err(|error| napi::Error::from_reason(error.to_string()))?;
    parse_counter_style_name(&source)
        .map(|parsed| serde_json::to_string(&parsed))
        .transpose()
        .map_err(|error| napi::Error::from_reason(error.to_string()))
}

/// Parses a stylesheet and returns owned, parser-independent JSON DTOs.
#[napi]
pub fn parse_stylesheet_tree_json(source: String, error_recovery: bool) -> napi::Result<String> {
    let parsed = parse_stylesheet_tree(&source, error_recovery)
        .map_err(|error| napi::Error::from_reason(error.to_string()))?;
    serde_json::to_string(&parsed).map_err(|error| napi::Error::from_reason(error.to_string()))
}

/// Scans exact top-level CSS rule source without exposing native parser nodes.
#[napi]
pub fn scan_top_level_rules_json(source: String) -> napi::Result<String> {
    let rules = scan_top_level_rules(&source)
        .map_err(|error| napi::Error::from_reason(error.to_string()))?;
    serde_json::to_string(&rules).map_err(|error| napi::Error::from_reason(error.to_string()))
}
