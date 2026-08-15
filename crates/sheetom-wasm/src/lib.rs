#![deny(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use js_sys::{Array, Error as JsError};
use sheetom_core::{
    canonicalize_declaration_block_with_limits as canonicalize, inspect_property_with_limits,
    normalize_media_text_with_limits, normalize_selector_text_with_limits,
    normalize_supports_text_with_limits, parse_container_prelude_with_limits,
    parse_counter_style_descriptor, parse_counter_style_descriptors, parse_counter_style_name,
    parse_recovered_rule_tree_with_limits, parse_recovered_single_rule_tree_with_limits,
    parse_scope_prelude_with_limits, parse_stylesheet_tree_with_limits,
    scan_top_level_rules_with_limits, serialize_css_identifier, serialize_font_family_setter,
    serialize_parsed_rule_json, serialize_parsed_rules_json, DeclarationContext,
    DeclarationMutation, DeclarationMutationResult, DeclarationState, EngineError, MutationOutcome,
    ResourceLimits,
};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct WasmDeclarationState {
    state: DeclarationState,
}

#[wasm_bindgen]
impl WasmDeclarationState {
    #[wasm_bindgen(constructor)]
    pub fn new(
        context: Option<String>,
        max_stylesheet_bytes: Option<u32>,
        max_declaration_value_bytes: Option<u32>,
        max_nesting_depth: Option<u32>,
        max_rules: Option<u32>,
        max_declarations_per_block: Option<u32>,
    ) -> Result<Self, JsValue> {
        let context = match context.as_deref() {
            None | Some("style") => DeclarationContext::Style,
            Some("font-face") => DeclarationContext::FontFace,
            Some("function") => DeclarationContext::Function,
            Some(context) => {
                return Err(js_error(format!(
                    "SHEETOM_DECLARATION_CONTEXT: unsupported declaration context {context}"
                )))
            }
        };
        Ok(Self {
            state: DeclarationState::new_with_context_and_limits(
                context,
                resource_limits(
                    max_stylesheet_bytes,
                    max_declaration_value_bytes,
                    max_nesting_depth,
                    max_rules,
                    max_declarations_per_block,
                ),
            ),
        })
    }

    #[wasm_bindgen(getter)]
    pub fn length(&self) -> u32 {
        u32::try_from(self.state.len()).unwrap_or(u32::MAX)
    }

    pub fn item(&self, index: u32) -> String {
        self.state.item(index as usize).to_owned()
    }

    #[wasm_bindgen(js_name = getPropertyValue)]
    pub fn get_property_value(&self, name: String) -> String {
        self.state.get_property_value(&name)
    }

    #[wasm_bindgen(js_name = getPropertyPriority)]
    pub fn get_property_priority(&self, name: String) -> String {
        self.state.get_property_priority(&name).to_owned()
    }

    #[wasm_bindgen(js_name = setProperty)]
    pub fn set_property(
        &mut self,
        name: String,
        value: String,
        priority: String,
        reserved_nesting_depth: Option<u32>,
    ) -> Result<String, JsValue> {
        self.state
            .set_property_checked_with_reserved_depth(
                &name,
                &value,
                &priority,
                reserved_nesting_depth.unwrap_or(0) as usize,
            )
            .map(|outcome| mutation_outcome_name(outcome).to_owned())
            .map_err(engine_error)
    }

    #[wasm_bindgen(js_name = applyMutations)]
    pub fn apply_mutations(
        &mut self,
        kinds: Array,
        properties: Array,
        values: Array,
        priorities: Array,
        reserved_nesting_depth: Option<u32>,
    ) -> Result<Array, JsValue> {
        let mutation_count = kinds.length();
        if properties.length() != mutation_count
            || values.length() != mutation_count
            || priorities.length() != mutation_count
        {
            return Err(js_error(
                "SHEETOM_DECLARATION_MUTATION: column lengths must match".to_owned(),
            ));
        }
        let mut native_mutations = Vec::with_capacity(mutation_count as usize);
        for index in 0..mutation_count {
            let kind = kinds.get(index).as_f64().ok_or_else(|| {
                js_error("SHEETOM_DECLARATION_MUTATION: operation code must be a number".to_owned())
            })?;
            let property = required_array_string(&properties, index, "property")?;
            match kind as u8 {
                0 if kind == 0.0 => native_mutations.push(DeclarationMutation::Set {
                    property,
                    value: required_array_string(&values, index, "value")?,
                    priority: required_array_string(&priorities, index, "priority")?,
                }),
                1 if kind == 1.0 => native_mutations.push(DeclarationMutation::Remove { property }),
                _ => {
                    return Err(js_error(format!(
                        "SHEETOM_DECLARATION_MUTATION: unsupported operation code {kind}"
                    )))
                }
            }
        }
        let results = self
            .state
            .apply_mutations_checked_with_reserved_depth(
                native_mutations,
                reserved_nesting_depth.unwrap_or(0) as usize,
            )
            .map_err(engine_error)?;
        let output = Array::new_with_length(results.len() as u32);
        for (index, result) in results.into_iter().enumerate() {
            let value = match result {
                DeclarationMutationResult::Set(outcome) => {
                    mutation_outcome_name(outcome).to_owned()
                }
                DeclarationMutationResult::Remove(value) => value,
            };
            output.set(index as u32, JsValue::from_str(&value));
        }
        Ok(output)
    }

    #[wasm_bindgen(js_name = removeProperty)]
    pub fn remove_property(&mut self, name: String) -> String {
        self.state.remove_property(&name)
    }

    #[wasm_bindgen(js_name = replaceCssText)]
    pub fn replace_css_text(
        &mut self,
        source: String,
        reserved_nesting_depth: Option<u32>,
    ) -> Result<(), JsValue> {
        self.state
            .replace_css_text_checked_with_reserved_depth(
                &source,
                reserved_nesting_depth.unwrap_or(0) as usize,
            )
            .map_err(engine_error)
    }

    #[wasm_bindgen(getter, js_name = cssText)]
    pub fn css_text(&self) -> String {
        self.state.css_text()
    }

    #[wasm_bindgen(js_name = serializeFormatted)]
    pub fn serialize_formatted(
        &self,
        safe: bool,
        indent: String,
        separator: String,
    ) -> Result<String, JsValue> {
        self.state
            .serialize_formatted(safe, &indent, &separator)
            .map_err(engine_error)
    }

    #[wasm_bindgen(js_name = serializeFormattedResilient)]
    pub fn serialize_formatted_resilient(
        &self,
        safe: bool,
        indent: String,
        separator: String,
    ) -> Result<Array, JsValue> {
        let (serialized, issues) = self
            .state
            .serialize_formatted_resilient(safe, &indent, &separator)
            .map_err(engine_error)?;
        let output = Array::new();
        output.push(&JsValue::from_str(&serialized));
        for issue in issues {
            output.push(&JsValue::from_str(&issue.shorthand));
            output.push(&JsValue::from_str(&issue.conflicting_longhands.join(",")));
        }
        Ok(output)
    }
}

#[wasm_bindgen(js_name = engineAbiIdentity)]
pub fn engine_abi_identity() -> String {
    env!("SHEETOM_ENGINE_ABI_IDENTITY").to_owned()
}

#[wasm_bindgen(js_name = normalizeSelector)]
pub fn normalize_selector(
    source: String,
    max_stylesheet_bytes: Option<u32>,
    max_declaration_value_bytes: Option<u32>,
    max_nesting_depth: Option<u32>,
    max_rules: Option<u32>,
    max_declarations_per_block: Option<u32>,
) -> Result<String, JsValue> {
    normalize_selector_text_with_limits(
        &source,
        resource_limits(
            max_stylesheet_bytes,
            max_declaration_value_bytes,
            max_nesting_depth,
            max_rules,
            max_declarations_per_block,
        ),
    )
    .map_err(engine_error)
}

#[wasm_bindgen(js_name = normalizeMedia)]
pub fn normalize_media(
    source: String,
    max_stylesheet_bytes: Option<u32>,
    max_declaration_value_bytes: Option<u32>,
    max_nesting_depth: Option<u32>,
    max_rules: Option<u32>,
    max_declarations_per_block: Option<u32>,
) -> Result<String, JsValue> {
    normalize_media_text_with_limits(
        &source,
        resource_limits(
            max_stylesheet_bytes,
            max_declaration_value_bytes,
            max_nesting_depth,
            max_rules,
            max_declarations_per_block,
        ),
    )
    .map_err(engine_error)
}

#[wasm_bindgen(js_name = normalizeSupports)]
pub fn normalize_supports(
    source: String,
    max_stylesheet_bytes: Option<u32>,
    max_declaration_value_bytes: Option<u32>,
    max_nesting_depth: Option<u32>,
    max_rules: Option<u32>,
    max_declarations_per_block: Option<u32>,
) -> Result<String, JsValue> {
    normalize_supports_text_with_limits(
        &source,
        resource_limits(
            max_stylesheet_bytes,
            max_declaration_value_bytes,
            max_nesting_depth,
            max_rules,
            max_declarations_per_block,
        ),
    )
    .map_err(engine_error)
}

#[wasm_bindgen(js_name = parseContainerPreludeJson)]
pub fn parse_container_prelude_json(
    source: String,
    max_stylesheet_bytes: Option<u32>,
    max_declaration_value_bytes: Option<u32>,
    max_nesting_depth: Option<u32>,
    max_rules: Option<u32>,
    max_declarations_per_block: Option<u32>,
) -> Result<String, JsValue> {
    let parsed = parse_container_prelude_with_limits(
        &source,
        resource_limits(
            max_stylesheet_bytes,
            max_declaration_value_bytes,
            max_nesting_depth,
            max_rules,
            max_declarations_per_block,
        ),
    )
    .map_err(engine_error)?;
    serde_json::to_string(&parsed).map_err(json_error)
}

#[wasm_bindgen(js_name = parseScopePreludeJson)]
pub fn parse_scope_prelude_json(
    source: String,
    max_stylesheet_bytes: Option<u32>,
    max_declaration_value_bytes: Option<u32>,
    max_nesting_depth: Option<u32>,
    max_rules: Option<u32>,
    max_declarations_per_block: Option<u32>,
) -> Result<String, JsValue> {
    let parsed = parse_scope_prelude_with_limits(
        &source,
        resource_limits(
            max_stylesheet_bytes,
            max_declaration_value_bytes,
            max_nesting_depth,
            max_rules,
            max_declarations_per_block,
        ),
    )
    .map_err(engine_error)?;
    serde_json::to_string(&parsed).map_err(json_error)
}

#[wasm_bindgen(js_name = parseCounterStyleDescriptorValue)]
pub fn parse_counter_style_descriptor_value(
    name: String,
    value: String,
    max_stylesheet_bytes: Option<u32>,
    max_declaration_value_bytes: Option<u32>,
    max_nesting_depth: Option<u32>,
    max_rules: Option<u32>,
    max_declarations_per_block: Option<u32>,
) -> Result<Option<String>, JsValue> {
    inspect_property_with_limits(
        "--sheetom-counter-style",
        &value,
        resource_limits(
            max_stylesheet_bytes,
            max_declaration_value_bytes,
            max_nesting_depth,
            max_rules,
            max_declarations_per_block,
        ),
    )
    .map_err(engine_error)?;
    Ok(parse_counter_style_descriptor(&name, &value))
}

#[wasm_bindgen(js_name = parseCounterStyleDescriptorsJson)]
pub fn parse_counter_style_descriptors_json(
    source: String,
    max_stylesheet_bytes: Option<u32>,
    max_declaration_value_bytes: Option<u32>,
    max_nesting_depth: Option<u32>,
    max_rules: Option<u32>,
    max_declarations_per_block: Option<u32>,
) -> Result<String, JsValue> {
    canonicalize(
        &source,
        resource_limits(
            max_stylesheet_bytes,
            max_declaration_value_bytes,
            max_nesting_depth,
            max_rules,
            max_declarations_per_block,
        ),
    )
    .map_err(engine_error)?;
    serde_json::to_string(&parse_counter_style_descriptors(&source)).map_err(json_error)
}

#[wasm_bindgen(js_name = parseCounterStyleNameJson)]
pub fn parse_counter_style_name_json(
    source: String,
    max_stylesheet_bytes: Option<u32>,
    max_declaration_value_bytes: Option<u32>,
    max_nesting_depth: Option<u32>,
    max_rules: Option<u32>,
    max_declarations_per_block: Option<u32>,
) -> Result<Option<String>, JsValue> {
    inspect_property_with_limits(
        "--sheetom-counter-style-name",
        &source,
        resource_limits(
            max_stylesheet_bytes,
            max_declaration_value_bytes,
            max_nesting_depth,
            max_rules,
            max_declarations_per_block,
        ),
    )
    .map_err(engine_error)?;
    parse_counter_style_name(&source)
        .map(|parsed| serde_json::to_string(&parsed))
        .transpose()
        .map_err(json_error)
}

#[wasm_bindgen(js_name = serializeIdentifierValue)]
pub fn serialize_identifier_value(value: String) -> String {
    serialize_css_identifier(&value)
}

#[wasm_bindgen(js_name = serializeFontFamilyValue)]
pub fn serialize_font_family_value(value: String) -> String {
    serialize_font_family_setter(&value)
}

#[wasm_bindgen(js_name = parseRecoveredRuleTreeJson)]
pub fn parse_recovered_rule_tree_json(
    source: String,
    max_stylesheet_bytes: Option<u32>,
    max_declaration_value_bytes: Option<u32>,
    max_nesting_depth: Option<u32>,
    max_rules: Option<u32>,
    max_declarations_per_block: Option<u32>,
) -> Result<String, JsValue> {
    let parsed = parse_recovered_rule_tree_with_limits(
        &source,
        resource_limits(
            max_stylesheet_bytes,
            max_declaration_value_bytes,
            max_nesting_depth,
            max_rules,
            max_declarations_per_block,
        ),
    )
    .map_err(engine_error)?;
    serialize_parsed_rule_json(&parsed).map_err(engine_error)
}

#[wasm_bindgen(js_name = parseRecoveredSingleRuleTreeJson)]
pub fn parse_recovered_single_rule_tree_json(
    source: String,
    max_stylesheet_bytes: Option<u32>,
    max_declaration_value_bytes: Option<u32>,
    max_nesting_depth: Option<u32>,
    max_rules: Option<u32>,
    max_declarations_per_block: Option<u32>,
) -> Result<String, JsValue> {
    let parsed = parse_recovered_single_rule_tree_with_limits(
        &source,
        resource_limits(
            max_stylesheet_bytes,
            max_declaration_value_bytes,
            max_nesting_depth,
            max_rules,
            max_declarations_per_block,
        ),
    )
    .map_err(engine_error)?;
    serialize_parsed_rule_json(&parsed).map_err(engine_error)
}

#[wasm_bindgen(js_name = parseStylesheetTreeJson)]
pub fn parse_stylesheet_tree_json(
    source: String,
    error_recovery: bool,
    max_stylesheet_bytes: Option<u32>,
    max_declaration_value_bytes: Option<u32>,
    max_nesting_depth: Option<u32>,
    max_rules: Option<u32>,
    max_declarations_per_block: Option<u32>,
) -> Result<String, JsValue> {
    let parsed = parse_stylesheet_tree_with_limits(
        &source,
        error_recovery,
        resource_limits(
            max_stylesheet_bytes,
            max_declaration_value_bytes,
            max_nesting_depth,
            max_rules,
            max_declarations_per_block,
        ),
    )
    .map_err(engine_error)?;
    serialize_parsed_rules_json(&parsed).map_err(engine_error)
}

#[wasm_bindgen(js_name = scanTopLevelRulesJson)]
pub fn scan_top_level_rules_json(
    source: String,
    max_stylesheet_bytes: Option<u32>,
    max_declaration_value_bytes: Option<u32>,
    max_nesting_depth: Option<u32>,
    max_rules: Option<u32>,
    max_declarations_per_block: Option<u32>,
) -> Result<String, JsValue> {
    let rules = scan_top_level_rules_with_limits(
        &source,
        resource_limits(
            max_stylesheet_bytes,
            max_declaration_value_bytes,
            max_nesting_depth,
            max_rules,
            max_declarations_per_block,
        ),
    )
    .map_err(engine_error)?;
    serde_json::to_string(&rules).map_err(json_error)
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

fn engine_error(error: EngineError) -> JsValue {
    js_error(error.to_string())
}

fn json_error(error: serde_json::Error) -> JsValue {
    js_error(error.to_string())
}

fn js_error(message: String) -> JsValue {
    JsError::new(&message).into()
}

fn required_array_string(array: &Array, index: u32, name: &str) -> Result<String, JsValue> {
    array.get(index).as_string().ok_or_else(|| {
        js_error(format!(
            "SHEETOM_DECLARATION_MUTATION: operation {name} must be a string"
        ))
    })
}
