#![cfg_attr(
    not(test),
    deny(clippy::expect_used, clippy::panic, clippy::unwrap_used)
)]

mod browser_longhand;
mod catalog;
mod counter_style;
mod declaration_state;
mod declaration_value;
mod extension_value;
mod font_face;
mod function_rule;
mod gap_rule;
mod geometric_value;
mod observable;
mod property_constraints;
mod recovered_value;
mod rules;
mod semantic_value;
mod shorthand;
mod substitution_value;
mod syntax;

pub(crate) use catalog::sheetom_parser_property_name;

pub use catalog::{
    CHROMIUM_BASELINE, INITIAL_VALUES_SOURCE_SHA256,
    SOURCE_SHA256 as PROPERTY_CATALOG_SOURCE_SHA256,
};
pub use counter_style::{
    parse_counter_style_descriptor, parse_counter_style_descriptors, parse_counter_style_name,
    ParsedCounterStyleDescriptor, ParsedCounterStyleName, COUNTER_STYLE_DESCRIPTORS,
};
pub use declaration_state::{
    DeclarationContext, DeclarationRecord, DeclarationState, MutationOutcome, ParsedDeclaration,
    PendingSubstitutionGroup,
};
#[doc(hidden)]
pub use declaration_value::{DeclarationValue, DeclarationValueKind};
#[doc(hidden)]
pub use extension_value::{
    CrossDimensionCalculationValue, IntegerCalculationValue, NamedPageSize, OffsetPositionValue,
    OffsetRotateDirection, OffsetRotateValue, PageLength, PageOrientation, PageSizeValue,
    SemanticExtensionValue,
};
#[doc(hidden)]
pub use gap_rule::GapRuleLonghandValue;
#[doc(hidden)]
pub use recovered_value::{
    recover_component_values, recover_component_values_with_limits, RecoveredBlockDelimiter,
    RecoveredClosure, RecoveredComponentKind, RecoveredComponentValue, RecoveredToken,
    RecoveredTokenKind, RecoveredTokenTermination, RecoveredValue, SourceSpan,
};
pub use rules::{
    normalize_media_text, normalize_media_text_with_limits, normalize_selector_text,
    normalize_selector_text_with_limits, normalize_supports_text,
    normalize_supports_text_with_limits, parse_container_prelude,
    parse_container_prelude_with_limits, parse_recovered_rule_tree,
    parse_recovered_rule_tree_with_limits, parse_recovered_single_rule_tree,
    parse_recovered_single_rule_tree_with_limits, parse_rule_tree, parse_rule_tree_with_limits,
    parse_scope_prelude, parse_scope_prelude_with_limits, parse_stylesheet_tree,
    parse_stylesheet_tree_with_limits, scan_top_level_rules, scan_top_level_rules_with_limits,
    serialize_font_family_setter, ParsedContainerPrelude, ParsedRule, ParsedScopePrelude,
};
#[doc(hidden)]
pub use semantic_value::{
    parse_semantic_property, parse_semantic_property_with_limits, parse_standard_semantic_property,
    parse_standard_semantic_property_with_limits, SemanticDeclaration, SemanticPropertyValue,
};
#[doc(hidden)]
pub use substitution_value::{
    analyze_recovered_substitutions, SemanticSubstitutionFunction, SemanticSubstitutionValue,
    SubstitutionFunctionKind,
};

#[doc(hidden)]
pub fn serialize_css_identifier(value: &str) -> String {
    syntax::serialize_identifier(value)
}

#[cfg(panic = "abort")]
compile_error!("sheetom-core must be compiled with panic=unwind");

use lightningcss::{
    declaration::DeclarationBlock,
    properties::{Property, PropertyId},
    stylesheet::{ParserOptions, PrinterOptions},
    traits::ToCss,
};
use std::{
    fmt::{Display, Formatter},
    panic::{catch_unwind, AssertUnwindSafe},
};

pub const ENGINE_REVISION: &str = "lightningcss-1.33.0-c6a0c3ce-sheetom.47";
pub const DEFAULT_MAX_STYLESHEET_BYTES: usize = 64 * 1024 * 1024;
pub const DEFAULT_MAX_DECLARATION_VALUE_BYTES: usize = 1024 * 1024;
pub const DEFAULT_MAX_NESTING_DEPTH: usize = 4096;
pub const DEFAULT_MAX_RULES: usize = 1_000_000;
pub const DEFAULT_MAX_DECLARATIONS_PER_BLOCK: usize = 100_000;

/// Per-sheet limits checked before parser allocation or CSSOM state mutation.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ResourceLimits {
    pub max_stylesheet_bytes: usize,
    pub max_declaration_value_bytes: usize,
    pub max_nesting_depth: usize,
    pub max_rules: usize,
    pub max_declarations_per_block: usize,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            max_stylesheet_bytes: DEFAULT_MAX_STYLESHEET_BYTES,
            max_declaration_value_bytes: DEFAULT_MAX_DECLARATION_VALUE_BYTES,
            max_nesting_depth: DEFAULT_MAX_NESTING_DEPTH,
            max_rules: DEFAULT_MAX_RULES,
            max_declarations_per_block: DEFAULT_MAX_DECLARATIONS_PER_BLOCK,
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum EngineError {
    InputLimitExceeded { actual: usize, limit: usize },
    DeclarationLimitExceeded { actual: usize, limit: usize },
    RuleLimitExceeded { actual: usize, limit: usize },
    NestingLimitExceeded { actual: usize, limit: usize },
    Parse(String),
    Serialize(String),
    UnexpectedPanic,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PropertyParseKind {
    Typed,
    SheetomTyped,
    Unparsed,
    Custom,
}

#[derive(Debug, PartialEq)]
pub struct PropertyInspection {
    pub kind: PropertyParseKind,
    pub canonical_value: String,
}

impl Display for EngineError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InputLimitExceeded { actual, limit } => write!(
                formatter,
                "SHEETOM_INPUT_LIMIT: CSS input is {actual} bytes; the limit is {limit} bytes"
            ),
            Self::DeclarationLimitExceeded { actual, limit } => write!(
                formatter,
                "SHEETOM_DECLARATION_LIMIT: declaration block has {actual} entries; the limit is {limit}"
            ),
            Self::RuleLimitExceeded { actual, limit } => write!(
                formatter,
                "SHEETOM_RULE_LIMIT: stylesheet has {actual} rules; the limit is {limit} rules"
            ),
            Self::NestingLimitExceeded { actual, limit } => write!(
                formatter,
                "SHEETOM_NESTING_LIMIT: CSS nesting depth is {actual}; the limit is {limit}"
            ),
            Self::Parse(message) => write!(formatter, "SHEETOM_PARSE_ERROR: {message}"),
            Self::Serialize(message) => write!(formatter, "SHEETOM_SERIALIZE_ERROR: {message}"),
            Self::UnexpectedPanic => formatter.write_str(
                "SHEETOM_NATIVE_PANIC: the CSS engine aborted the current operation safely",
            ),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct SafetyMetrics {
    declaration_count: usize,
    maximum_depth: usize,
}

fn scan_safety_metrics(source: &str) -> SafetyMetrics {
    let bytes = source.as_bytes();
    let mut metrics = SafetyMetrics::default();
    let mut depth = 0usize;
    let mut index = 0usize;
    let mut quote = None;
    let mut in_comment = false;
    let mut has_top_level_content = false;

    while index < bytes.len() {
        let byte = bytes[index];

        if in_comment {
            if byte == b'*' && bytes.get(index + 1) == Some(&b'/') {
                in_comment = false;
                index += 2;
                continue;
            }

            index += 1;
            continue;
        }

        if let Some(delimiter) = quote {
            if byte == b'\\' {
                index = (index + 2).min(bytes.len());
                continue;
            }

            if byte == delimiter || matches!(byte, b'\n' | b'\r' | b'\x0c') {
                quote = None;
            }

            index += 1;
            continue;
        }

        if byte == b'/' && bytes.get(index + 1) == Some(&b'*') {
            in_comment = true;
            index += 2;
            continue;
        }

        if matches!(byte, b'\'' | b'"') {
            quote = Some(byte);
            has_top_level_content |= depth == 0;
            index += 1;
            continue;
        }

        if byte == b'\\' {
            has_top_level_content |= depth == 0;
            index = (index + 2).min(bytes.len());
            continue;
        }

        if matches!(byte, b'(' | b'[' | b'{') {
            depth += 1;
            metrics.maximum_depth = metrics.maximum_depth.max(depth);
            has_top_level_content |= depth == 1;
            index += 1;
            continue;
        }

        if matches!(byte, b')' | b']' | b'}') {
            depth = depth.saturating_sub(1);
            has_top_level_content |= depth == 0;
            index += 1;
            continue;
        }

        if byte == b';' && depth == 0 {
            if has_top_level_content {
                metrics.declaration_count += 1;
                has_top_level_content = false;
            }
            index += 1;
            continue;
        }

        has_top_level_content |= depth == 0 && !byte.is_ascii_whitespace();
        index += 1;
    }

    if has_top_level_content {
        metrics.declaration_count += 1;
    }

    metrics
}

pub(crate) fn validate_declaration_value_input(
    value: &str,
    limits: ResourceLimits,
) -> Result<(), EngineError> {
    if value.len() > limits.max_declaration_value_bytes {
        return Err(EngineError::InputLimitExceeded {
            actual: value.len(),
            limit: limits.max_declaration_value_bytes,
        });
    }
    let metrics = scan_safety_metrics(value);
    if metrics.maximum_depth > limits.max_nesting_depth {
        return Err(EngineError::NestingLimitExceeded {
            actual: metrics.maximum_depth,
            limit: limits.max_nesting_depth,
        });
    }
    Ok(())
}

pub(crate) fn validate_declaration_block_input(
    source: &str,
    limits: ResourceLimits,
) -> Result<(), EngineError> {
    if source.len() > limits.max_stylesheet_bytes {
        return Err(EngineError::InputLimitExceeded {
            actual: source.len(),
            limit: limits.max_stylesheet_bytes,
        });
    }
    let metrics = scan_safety_metrics(source);
    if metrics.maximum_depth > limits.max_nesting_depth {
        return Err(EngineError::NestingLimitExceeded {
            actual: metrics.maximum_depth,
            limit: limits.max_nesting_depth,
        });
    }
    if metrics.declaration_count > limits.max_declarations_per_block {
        return Err(EngineError::DeclarationLimitExceeded {
            actual: metrics.declaration_count,
            limit: limits.max_declarations_per_block,
        });
    }
    Ok(())
}

fn run_guarded<T>(operation: impl FnOnce() -> Result<T, EngineError>) -> Result<T, EngineError> {
    match catch_unwind(AssertUnwindSafe(operation)) {
        Ok(result) => result,
        Err(_) => Err(EngineError::UnexpectedPanic),
    }
}

fn canonicalize_unchecked(source: &str) -> Result<String, EngineError> {
    let declarations = DeclarationBlock::parse_string(source, ParserOptions::default())
        .map_err(|error| EngineError::Parse(error.to_string()))?;

    declarations
        .to_css_string(PrinterOptions::default())
        .map_err(|error| EngineError::Serialize(error.to_string()))
}

fn inspect_property_unchecked<'i>(
    name: &'i str,
    value: &'i str,
) -> Result<PropertyInspection, EngineError> {
    if let Some(parser_name) = sheetom_parser_property_name(name) {
        let property = Property::parse_string(
            PropertyId::from(parser_name),
            value,
            ParserOptions::default(),
        )
        .map_err(|error| EngineError::Parse(error.to_string()))?;
        if matches!(property, Property::Unparsed(_) | Property::Custom(_)) {
            return Err(EngineError::Parse(format!(
                "invalid value for {name}: {value}"
            )));
        }
        let canonical_value = property
            .value_to_css_string(PrinterOptions::default())
            .map_err(|error| EngineError::Serialize(error.to_string()))?;
        return Ok(PropertyInspection {
            kind: PropertyParseKind::SheetomTyped,
            canonical_value,
        });
    }

    let property = Property::parse_string(PropertyId::from(name), value, ParserOptions::default())
        .map_err(|error| EngineError::Parse(error.to_string()))?;
    let kind = match property {
        Property::Unparsed(_) => PropertyParseKind::Unparsed,
        Property::Custom(_) => PropertyParseKind::Custom,
        _ => PropertyParseKind::Typed,
    };
    let canonical_value = property
        .value_to_css_string(PrinterOptions::default())
        .map_err(|error| EngineError::Serialize(error.to_string()))?;

    Ok(PropertyInspection {
        kind,
        canonical_value,
    })
}

#[doc(hidden)]
pub fn inspect_property<'i>(
    name: &'i str,
    value: &'i str,
) -> Result<PropertyInspection, EngineError> {
    inspect_property_with_limits(name, value, ResourceLimits::default())
}

#[doc(hidden)]
pub fn inspect_property_with_limits<'i>(
    name: &'i str,
    value: &'i str,
    limits: ResourceLimits,
) -> Result<PropertyInspection, EngineError> {
    validate_declaration_value_input(value, limits)?;

    run_guarded(|| inspect_property_unchecked(name, value))
}

#[doc(hidden)]
pub fn validate_static_property<'i>(
    name: &'i str,
    value: &'i str,
) -> Result<PropertyInspection, EngineError> {
    let inspection = inspect_property(name, value)?;
    if matches!(
        inspection.kind,
        PropertyParseKind::Typed | PropertyParseKind::SheetomTyped
    ) {
        return Ok(inspection);
    }

    Err(EngineError::Parse(format!(
        "invalid static value for {name}: {value}"
    )))
}

pub fn canonicalize_declaration_block(source: &str) -> Result<String, EngineError> {
    canonicalize_declaration_block_with_limits(source, ResourceLimits::default())
}

#[doc(hidden)]
pub fn canonicalize_declaration_block_with_limits(
    source: &str,
    limits: ResourceLimits,
) -> Result<String, EngineError> {
    validate_declaration_block_input(source, limits)?;

    run_guarded(|| canonicalize_unchecked(source))
}

#[doc(hidden)]
pub fn fuzz_declaration_block(source: &str) {
    let _ = canonicalize_declaration_block(source);
}

#[doc(hidden)]
pub fn fuzz_recovered_component_values(source: &str) {
    let Ok(recovered) = recover_component_values(source) else {
        return;
    };
    let _ = recovered.reparsable_css();
    let _ = analyze_recovered_substitutions(&recovered);
    for property in [
        "border-color",
        "color",
        "z-index",
        "offset-anchor",
        "offset-position",
        "offset-rotate",
        "size",
        "scrollbar-color",
        "width",
    ] {
        let _ = parse_semantic_property(property, source);
    }
}

#[cfg(test)]
mod tests {
    use super::{
        canonicalize_declaration_block, canonicalize_declaration_block_with_limits,
        inspect_property, inspect_property_with_limits, run_guarded, scan_safety_metrics,
        validate_static_property, EngineError, PropertyInspection, PropertyParseKind,
        ResourceLimits, SafetyMetrics, DEFAULT_MAX_DECLARATIONS_PER_BLOCK,
        DEFAULT_MAX_DECLARATION_VALUE_BYTES, DEFAULT_MAX_NESTING_DEPTH, DEFAULT_MAX_RULES,
        DEFAULT_MAX_STYLESHEET_BYTES, ENGINE_REVISION,
    };

    #[test]
    fn reports_the_vendored_engine_revision() {
        assert_eq!(ENGINE_REVISION, "lightningcss-1.33.0-c6a0c3ce-sheetom.47");
    }

    #[test]
    fn image_set_never_crosses_an_ast_boundary() {
        let css = canonicalize_declaration_block(
            "background: image-set(url(a.png) 1x, url(b.png) 2x) center/cover no-repeat red",
        )
        .expect("valid Chromium background should parse");

        assert!(css.contains("image-set("));
        assert!(css.contains("background:"));
    }

    #[test]
    fn distinguishes_vendored_and_sheetom_owned_grammars() {
        assert_eq!(
            inspect_property("background-position", "left 10px top 20px"),
            Ok(PropertyInspection {
                kind: PropertyParseKind::Typed,
                canonical_value: "10px 20px".into(),
            })
        );
        assert_eq!(
            inspect_property("animation", "auto ease 1s foo")
                .expect("automatic animation duration should parse")
                .kind,
            PropertyParseKind::Typed,
        );
        assert_eq!(
            inspect_property("row-rule", "2px dashed red")
                .expect("row-rule should use the SheetOM border grammar")
                .kind,
            PropertyParseKind::SheetomTyped,
        );
        assert_eq!(
            inspect_property("rule", "2px dashed red")
                .expect("rule should use the SheetOM border grammar")
                .kind,
            PropertyParseKind::SheetomTyped,
        );
        assert_eq!(
            inspect_property("font", "caption")
                .expect("system font should parse")
                .kind,
            PropertyParseKind::Typed,
        );
        for name in [
            "-webkit-border-after",
            "-webkit-border-before",
            "-webkit-border-end",
            "-webkit-border-start",
            "-webkit-column-rule",
            "column-rule",
        ] {
            assert_eq!(
                inspect_property(name, "2px dashed red")
                    .expect("border-like shorthand should use the SheetOM border grammar")
                    .kind,
                PropertyParseKind::SheetomTyped,
                "{name}"
            );
        }
        assert_eq!(
            inspect_property("-webkit-text-stroke", "2px red")
                .expect("text stroke should use width and color grammar")
                .kind,
            PropertyParseKind::SheetomTyped,
        );
        assert_eq!(
            inspect_property("grid-gap", "1px 2px")
                .expect("legacy grid gap should use gap grammar")
                .kind,
            PropertyParseKind::SheetomTyped,
        );
    }

    #[test]
    fn rejects_invalid_sheetom_owned_grammar() {
        assert!(matches!(
            inspect_property("row-rule", "2px dashed solid red"),
            Err(EngineError::Parse(_))
        ));
        assert!(matches!(
            inspect_property("rule", "2px dashed solid red"),
            Err(EngineError::Parse(_))
        ));
    }

    #[test]
    fn satisfies_every_property_specific_native_grammar_branch() {
        let inventory: serde_json::Value = serde_json::from_str(include_str!(
            "../../../compatibility/native-grammar-inventory.json"
        ))
        .expect("native grammar inventory should be valid JSON");
        let branches = inventory["propertyBranches"]
            .as_array()
            .expect("native grammar inventory should contain branches");

        for branch in branches {
            let id = branch["id"].as_str().expect("branch should have an id");
            let property = branch["property"]
                .as_str()
                .expect("branch should have a property");
            let input = branch["input"]
                .as_str()
                .expect("branch should have an input");
            let accepted = branch["accepted"]
                .as_bool()
                .expect("branch should have an acceptance decision");
            let result = validate_static_property(property, input);

            assert_eq!(result.is_ok(), accepted, "{id}");
        }
    }

    #[test]
    fn defaults_match_the_rc6_resource_contract() {
        assert_eq!(DEFAULT_MAX_STYLESHEET_BYTES, 64 * 1024 * 1024);
        assert_eq!(DEFAULT_MAX_DECLARATION_VALUE_BYTES, 1024 * 1024);
        assert_eq!(DEFAULT_MAX_NESTING_DEPTH, 4096);
        assert_eq!(DEFAULT_MAX_RULES, 1_000_000);
        assert_eq!(DEFAULT_MAX_DECLARATIONS_PER_BLOCK, 100_000);
    }

    #[test]
    fn rejects_oversized_inputs_before_parsing() {
        let limits = ResourceLimits {
            max_stylesheet_bytes: 8,
            ..ResourceLimits::default()
        };
        let source = "x".repeat(limits.max_stylesheet_bytes + 1);

        assert_eq!(
            canonicalize_declaration_block_with_limits(&source, limits),
            Err(EngineError::InputLimitExceeded {
                actual: limits.max_stylesheet_bytes + 1,
                limit: limits.max_stylesheet_bytes,
            })
        );
    }

    #[test]
    fn accepts_the_exact_declaration_budget() {
        let limits = ResourceLimits {
            max_declaration_value_bytes: 16,
            ..ResourceLimits::default()
        };
        let source = "x".repeat(limits.max_declaration_value_bytes);

        assert!(inspect_property_with_limits("--x", &source, limits).is_ok());
        assert_eq!(
            inspect_property_with_limits("--x", &format!("{source}x"), limits),
            Err(EngineError::InputLimitExceeded {
                actual: limits.max_declaration_value_bytes + 1,
                limit: limits.max_declaration_value_bytes,
            })
        );
    }

    #[test]
    fn converts_panics_into_recoverable_internal_errors() {
        let result: Result<(), EngineError> = run_guarded(|| panic!("simulated parser panic"));

        assert_eq!(result, Err(EngineError::UnexpectedPanic));
    }

    #[test]
    fn safety_scanner_ignores_nested_and_quoted_delimiters() {
        assert_eq!(
            scan_safety_metrics(r#"--x: "a;b"; background: fn([a;b], {c;d}); color: red"#),
            SafetyMetrics {
                declaration_count: 3,
                maximum_depth: 2,
            }
        );
    }

    #[test]
    fn rejects_excessive_nesting_before_parsing() {
        let limits = ResourceLimits {
            max_nesting_depth: 8,
            ..ResourceLimits::default()
        };
        let source = format!("--x: {}value", "fn(".repeat(limits.max_nesting_depth + 1));

        assert_eq!(
            canonicalize_declaration_block_with_limits(&source, limits),
            Err(EngineError::NestingLimitExceeded {
                actual: limits.max_nesting_depth + 1,
                limit: limits.max_nesting_depth,
            })
        );
    }

    #[test]
    fn rejects_too_many_declarations_before_parsing() {
        let limits = ResourceLimits {
            max_declarations_per_block: 2,
            ..ResourceLimits::default()
        };
        let source = "x:;".repeat(limits.max_declarations_per_block + 1);

        assert_eq!(
            canonicalize_declaration_block_with_limits(&source, limits),
            Err(EngineError::DeclarationLimitExceeded {
                actual: limits.max_declarations_per_block + 1,
                limit: limits.max_declarations_per_block,
            })
        );
    }
}
