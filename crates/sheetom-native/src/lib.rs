#![deny(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

#[cfg(panic = "abort")]
compile_error!("sheetom-native must be compiled with panic=unwind");

use napi_derive::napi;
use sheetom_core::{
    canonicalize_declaration_block_with_limits as canonicalize, inspect_property_with_limits,
    normalize_media_text_with_limits, normalize_selector_text_with_limits,
    normalize_supports_text_with_limits, parse_container_prelude_with_limits,
    parse_counter_style_descriptor, parse_counter_style_descriptors, parse_counter_style_name,
    parse_recovered_rule_tree_with_limits, parse_recovered_single_rule_tree_with_limits,
    parse_rule_tree_with_limits, parse_scope_prelude_with_limits,
    parse_stylesheet_tree_with_limits, scan_top_level_rules_with_limits, serialize_css_identifier,
    serialize_font_family_setter, DeclarationContext, DeclarationState, MutationOutcome,
    ResourceLimits, ENGINE_REVISION,
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
    pub fn new(
        context: Option<String>,
        max_stylesheet_bytes: Option<u32>,
        max_declaration_value_bytes: Option<u32>,
        max_nesting_depth: Option<u32>,
        max_rules: Option<u32>,
        max_declarations_per_block: Option<u32>,
    ) -> napi::Result<Self> {
        let context = match context.as_deref() {
            None | Some("style") => DeclarationContext::Style,
            Some("font-face") => DeclarationContext::FontFace,
            Some("function") => DeclarationContext::Function,
            Some(context) => {
                return Err(napi::Error::from_reason(format!(
                    "SHEETOM_DECLARATION_CONTEXT: unsupported declaration context {context}"
                )))
            }
        };
        let limits = resource_limits(
            max_stylesheet_bytes,
            max_declaration_value_bytes,
            max_nesting_depth,
            max_rules,
            max_declarations_per_block,
        );
        Ok(Self {
            state: DeclarationState::new_with_context_and_limits(context, limits),
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
    pub fn set_property(
        &mut self,
        name: String,
        value: String,
        priority: String,
        reserved_nesting_depth: Option<u32>,
    ) -> napi::Result<String> {
        self.state
            .set_property_checked_with_reserved_depth(
                &name,
                &value,
                &priority,
                reserved_nesting_depth.unwrap_or(0) as usize,
            )
            .map(|outcome| mutation_outcome_name(outcome).to_owned())
            .map_err(|error| napi::Error::from_reason(error.to_string()))
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
    pub fn replace_css_text(
        &mut self,
        source: String,
        reserved_nesting_depth: Option<u32>,
    ) -> napi::Result<()> {
        self.state
            .replace_css_text_checked_with_reserved_depth(
                &source,
                reserved_nesting_depth.unwrap_or(0) as usize,
            )
            .map_err(|error| napi::Error::from_reason(error.to_string()))
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

fn resource_limits(
    max_stylesheet_bytes: Option<u32>,
    max_declaration_value_bytes: Option<u32>,
    max_nesting_depth: Option<u32>,
    max_rules: Option<u32>,
    max_declarations_per_block: Option<u32>,
) -> ResourceLimits {
    let defaults = ResourceLimits::default();
    ResourceLimits {
        max_stylesheet_bytes: max_stylesheet_bytes
            .map_or(defaults.max_stylesheet_bytes, |value| value as usize),
        max_declaration_value_bytes: max_declaration_value_bytes
            .map_or(defaults.max_declaration_value_bytes, |value| value as usize),
        max_nesting_depth: max_nesting_depth
            .map_or(defaults.max_nesting_depth, |value| value as usize),
        max_rules: max_rules.map_or(defaults.max_rules, |value| value as usize),
        max_declarations_per_block: max_declarations_per_block
            .map_or(defaults.max_declarations_per_block, |value| value as usize),
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
pub fn canonicalize_declaration_block(
    source: String,
    max_stylesheet_bytes: Option<u32>,
    max_declaration_value_bytes: Option<u32>,
    max_nesting_depth: Option<u32>,
    max_rules: Option<u32>,
    max_declarations_per_block: Option<u32>,
) -> napi::Result<String> {
    let limits = resource_limits(
        max_stylesheet_bytes,
        max_declaration_value_bytes,
        max_nesting_depth,
        max_rules,
        max_declarations_per_block,
    );
    canonicalize(&source, limits).map_err(|error| napi::Error::from_reason(error.to_string()))
}

/// Parses exactly one rule and returns an owned, parser-independent JSON DTO.
#[napi]
pub fn parse_rule_tree_json(
    source: String,
    max_stylesheet_bytes: Option<u32>,
    max_declaration_value_bytes: Option<u32>,
    max_nesting_depth: Option<u32>,
    max_rules: Option<u32>,
    max_declarations_per_block: Option<u32>,
) -> napi::Result<String> {
    let limits = resource_limits(
        max_stylesheet_bytes,
        max_declaration_value_bytes,
        max_nesting_depth,
        max_rules,
        max_declarations_per_block,
    );
    let parsed = parse_rule_tree_with_limits(&source, limits)
        .map_err(|error| napi::Error::from_reason(error.to_string()))?;
    serde_json::to_string(&parsed).map_err(|error| napi::Error::from_reason(error.to_string()))
}

/// Parses one exact outer rule with browser-style recovery inside its block.
#[napi]
pub fn parse_recovered_single_rule_tree_json(
    source: String,
    max_stylesheet_bytes: Option<u32>,
    max_declaration_value_bytes: Option<u32>,
    max_nesting_depth: Option<u32>,
    max_rules: Option<u32>,
    max_declarations_per_block: Option<u32>,
) -> napi::Result<String> {
    let limits = resource_limits(
        max_stylesheet_bytes,
        max_declaration_value_bytes,
        max_nesting_depth,
        max_rules,
        max_declarations_per_block,
    );
    let parsed = parse_recovered_single_rule_tree_with_limits(&source, limits)
        .map_err(|error| napi::Error::from_reason(error.to_string()))?;
    serde_json::to_string(&parsed).map_err(|error| napi::Error::from_reason(error.to_string()))
}

/// Parses exactly one rule with browser-style declaration recovery.
#[napi]
pub fn parse_recovered_rule_tree_json(
    source: String,
    max_stylesheet_bytes: Option<u32>,
    max_declaration_value_bytes: Option<u32>,
    max_nesting_depth: Option<u32>,
    max_rules: Option<u32>,
    max_declarations_per_block: Option<u32>,
) -> napi::Result<String> {
    let limits = resource_limits(
        max_stylesheet_bytes,
        max_declaration_value_bytes,
        max_nesting_depth,
        max_rules,
        max_declarations_per_block,
    );
    let parsed = parse_recovered_rule_tree_with_limits(&source, limits)
        .map_err(|error| napi::Error::from_reason(error.to_string()))?;
    serde_json::to_string(&parsed).map_err(|error| napi::Error::from_reason(error.to_string()))
}

#[napi]
pub fn normalize_selector(
    source: String,
    max_stylesheet_bytes: Option<u32>,
    max_declaration_value_bytes: Option<u32>,
    max_nesting_depth: Option<u32>,
    max_rules: Option<u32>,
    max_declarations_per_block: Option<u32>,
) -> napi::Result<String> {
    let limits = resource_limits(
        max_stylesheet_bytes,
        max_declaration_value_bytes,
        max_nesting_depth,
        max_rules,
        max_declarations_per_block,
    );
    normalize_selector_text_with_limits(&source, limits)
        .map_err(|error| napi::Error::from_reason(error.to_string()))
}

#[napi]
pub fn normalize_media(
    source: String,
    max_stylesheet_bytes: Option<u32>,
    max_declaration_value_bytes: Option<u32>,
    max_nesting_depth: Option<u32>,
    max_rules: Option<u32>,
    max_declarations_per_block: Option<u32>,
) -> napi::Result<String> {
    let limits = resource_limits(
        max_stylesheet_bytes,
        max_declaration_value_bytes,
        max_nesting_depth,
        max_rules,
        max_declarations_per_block,
    );
    normalize_media_text_with_limits(&source, limits)
        .map_err(|error| napi::Error::from_reason(error.to_string()))
}

#[napi]
pub fn normalize_supports(
    source: String,
    max_stylesheet_bytes: Option<u32>,
    max_declaration_value_bytes: Option<u32>,
    max_nesting_depth: Option<u32>,
    max_rules: Option<u32>,
    max_declarations_per_block: Option<u32>,
) -> napi::Result<String> {
    let limits = resource_limits(
        max_stylesheet_bytes,
        max_declaration_value_bytes,
        max_nesting_depth,
        max_rules,
        max_declarations_per_block,
    );
    normalize_supports_text_with_limits(&source, limits)
        .map_err(|error| napi::Error::from_reason(error.to_string()))
}

#[napi]
pub fn parse_container_prelude_json(
    source: String,
    max_stylesheet_bytes: Option<u32>,
    max_declaration_value_bytes: Option<u32>,
    max_nesting_depth: Option<u32>,
    max_rules: Option<u32>,
    max_declarations_per_block: Option<u32>,
) -> napi::Result<String> {
    let limits = resource_limits(
        max_stylesheet_bytes,
        max_declaration_value_bytes,
        max_nesting_depth,
        max_rules,
        max_declarations_per_block,
    );
    let parsed = parse_container_prelude_with_limits(&source, limits)
        .map_err(|error| napi::Error::from_reason(error.to_string()))?;
    serde_json::to_string(&parsed).map_err(|error| napi::Error::from_reason(error.to_string()))
}

#[napi]
pub fn parse_scope_prelude_json(
    source: String,
    max_stylesheet_bytes: Option<u32>,
    max_declaration_value_bytes: Option<u32>,
    max_nesting_depth: Option<u32>,
    max_rules: Option<u32>,
    max_declarations_per_block: Option<u32>,
) -> napi::Result<String> {
    let limits = resource_limits(
        max_stylesheet_bytes,
        max_declaration_value_bytes,
        max_nesting_depth,
        max_rules,
        max_declarations_per_block,
    );
    let parsed = parse_scope_prelude_with_limits(&source, limits)
        .map_err(|error| napi::Error::from_reason(error.to_string()))?;
    serde_json::to_string(&parsed).map_err(|error| napi::Error::from_reason(error.to_string()))
}

#[napi]
pub fn parse_counter_style_descriptor_value(
    name: String,
    value: String,
    max_stylesheet_bytes: Option<u32>,
    max_declaration_value_bytes: Option<u32>,
    max_nesting_depth: Option<u32>,
    max_rules: Option<u32>,
    max_declarations_per_block: Option<u32>,
) -> napi::Result<Option<String>> {
    let limits = resource_limits(
        max_stylesheet_bytes,
        max_declaration_value_bytes,
        max_nesting_depth,
        max_rules,
        max_declarations_per_block,
    );
    inspect_property_with_limits("--sheetom-counter-style", &value, limits)
        .map_err(|error| napi::Error::from_reason(error.to_string()))?;
    Ok(parse_counter_style_descriptor(&name, &value))
}

#[napi]
pub fn parse_counter_style_descriptors_json(
    source: String,
    max_stylesheet_bytes: Option<u32>,
    max_declaration_value_bytes: Option<u32>,
    max_nesting_depth: Option<u32>,
    max_rules: Option<u32>,
    max_declarations_per_block: Option<u32>,
) -> napi::Result<String> {
    let limits = resource_limits(
        max_stylesheet_bytes,
        max_declaration_value_bytes,
        max_nesting_depth,
        max_rules,
        max_declarations_per_block,
    );
    canonicalize(&source, limits).map_err(|error| napi::Error::from_reason(error.to_string()))?;
    serde_json::to_string(&parse_counter_style_descriptors(&source))
        .map_err(|error| napi::Error::from_reason(error.to_string()))
}

#[napi]
pub fn parse_counter_style_name_json(
    source: String,
    max_stylesheet_bytes: Option<u32>,
    max_declaration_value_bytes: Option<u32>,
    max_nesting_depth: Option<u32>,
    max_rules: Option<u32>,
    max_declarations_per_block: Option<u32>,
) -> napi::Result<Option<String>> {
    let limits = resource_limits(
        max_stylesheet_bytes,
        max_declaration_value_bytes,
        max_nesting_depth,
        max_rules,
        max_declarations_per_block,
    );
    inspect_property_with_limits("--sheetom-counter-style-name", &source, limits)
        .map_err(|error| napi::Error::from_reason(error.to_string()))?;
    parse_counter_style_name(&source)
        .map(|parsed| serde_json::to_string(&parsed))
        .transpose()
        .map_err(|error| napi::Error::from_reason(error.to_string()))
}

#[napi]
pub fn serialize_identifier_value(value: String) -> String {
    serialize_css_identifier(&value)
}

#[napi]
pub fn serialize_font_family_value(value: String) -> String {
    serialize_font_family_setter(&value)
}

/// Parses a stylesheet and returns owned, parser-independent JSON DTOs.
#[napi]
pub fn parse_stylesheet_tree_json(
    source: String,
    error_recovery: bool,
    max_stylesheet_bytes: Option<u32>,
    max_declaration_value_bytes: Option<u32>,
    max_nesting_depth: Option<u32>,
    max_rules: Option<u32>,
    max_declarations_per_block: Option<u32>,
) -> napi::Result<String> {
    let limits = resource_limits(
        max_stylesheet_bytes,
        max_declaration_value_bytes,
        max_nesting_depth,
        max_rules,
        max_declarations_per_block,
    );
    let parsed = parse_stylesheet_tree_with_limits(&source, error_recovery, limits)
        .map_err(|error| napi::Error::from_reason(error.to_string()))?;
    serde_json::to_string(&parsed).map_err(|error| napi::Error::from_reason(error.to_string()))
}

/// Scans exact top-level CSS rule source without exposing native parser nodes.
#[napi]
pub fn scan_top_level_rules_json(
    source: String,
    max_stylesheet_bytes: Option<u32>,
    max_declaration_value_bytes: Option<u32>,
    max_nesting_depth: Option<u32>,
    max_rules: Option<u32>,
    max_declarations_per_block: Option<u32>,
) -> napi::Result<String> {
    let limits = resource_limits(
        max_stylesheet_bytes,
        max_declaration_value_bytes,
        max_nesting_depth,
        max_rules,
        max_declarations_per_block,
    );
    let rules = scan_top_level_rules_with_limits(&source, limits)
        .map_err(|error| napi::Error::from_reason(error.to_string()))?;
    serde_json::to_string(&rules).map_err(|error| napi::Error::from_reason(error.to_string()))
}
