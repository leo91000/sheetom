#![cfg_attr(
    not(test),
    deny(clippy::expect_used, clippy::panic, clippy::unwrap_used)
)]

mod catalog;
mod declaration_state;
mod font_face;
mod observable;
mod rules;
mod shorthand;
mod syntax;
mod value_grammar;

pub use catalog::{
    CHROMIUM_BASELINE, INITIAL_VALUES_SOURCE_SHA256,
    SOURCE_SHA256 as PROPERTY_CATALOG_SOURCE_SHA256,
};
pub use declaration_state::{
    DeclarationContext, DeclarationRecord, DeclarationState, MutationOutcome, ParsedDeclaration,
    PendingSubstitutionGroup,
};
pub use rules::{
    parse_recovered_rule_tree, parse_rule_tree, parse_stylesheet_tree, scan_top_level_rules,
    ParsedRule,
};

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

pub const ENGINE_REVISION: &str = "lightningcss-1.33.0-c6a0c3ce-sheetom.6";
const MAX_DECLARATION_BYTES: usize = 1024 * 1024;
const MAX_DECLARATIONS_PER_BLOCK: usize = 100_000;
const MAX_NESTING_DEPTH: usize = 4096;

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
                "SHEETOM_INPUT_LIMIT: declaration block is {actual} bytes; the limit is {limit} bytes"
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

pub(crate) fn sheetom_parser_property_name(name: &str) -> Option<&'static str> {
    if matches!(
        name,
        "-webkit-border-after"
            | "-webkit-border-before"
            | "-webkit-border-end"
            | "-webkit-border-start"
            | "-webkit-column-rule"
            | "-webkit-text-stroke"
            | "column-rule"
            | "row-rule"
            | "rule"
    ) {
        return Some("border");
    }
    if name == "grid-gap" {
        return Some("gap");
    }
    if name.ends_with("rule-width") || name == "-webkit-text-stroke-width" {
        return Some("border-top-width");
    }
    if name.ends_with("rule-style") {
        return Some("border-top-style");
    }
    if name.ends_with("rule-color") || name == "-webkit-text-stroke-color" {
        return Some("border-top-color");
    }
    None
}

#[doc(hidden)]
pub fn inspect_property<'i>(
    name: &'i str,
    value: &'i str,
) -> Result<PropertyInspection, EngineError> {
    if value.len() > MAX_DECLARATION_BYTES {
        return Err(EngineError::InputLimitExceeded {
            actual: value.len(),
            limit: MAX_DECLARATION_BYTES,
        });
    }

    let metrics = scan_safety_metrics(value);
    if metrics.maximum_depth > MAX_NESTING_DEPTH {
        return Err(EngineError::NestingLimitExceeded {
            actual: metrics.maximum_depth,
            limit: MAX_NESTING_DEPTH,
        });
    }

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
    if source.len() > MAX_DECLARATION_BYTES {
        return Err(EngineError::InputLimitExceeded {
            actual: source.len(),
            limit: MAX_DECLARATION_BYTES,
        });
    }

    let metrics = scan_safety_metrics(source);
    if metrics.maximum_depth > MAX_NESTING_DEPTH {
        return Err(EngineError::NestingLimitExceeded {
            actual: metrics.maximum_depth,
            limit: MAX_NESTING_DEPTH,
        });
    }

    if metrics.declaration_count > MAX_DECLARATIONS_PER_BLOCK {
        return Err(EngineError::DeclarationLimitExceeded {
            actual: metrics.declaration_count,
            limit: MAX_DECLARATIONS_PER_BLOCK,
        });
    }

    run_guarded(|| canonicalize_unchecked(source))
}

#[doc(hidden)]
pub fn fuzz_declaration_block(source: &str) {
    let _ = canonicalize_declaration_block(source);
}

#[cfg(test)]
mod tests {
    use super::{
        canonicalize_declaration_block, inspect_property, run_guarded, scan_safety_metrics,
        validate_static_property, EngineError, PropertyInspection, PropertyParseKind,
        SafetyMetrics, ENGINE_REVISION, MAX_DECLARATIONS_PER_BLOCK, MAX_DECLARATION_BYTES,
        MAX_NESTING_DEPTH,
    };

    #[test]
    fn reports_the_vendored_engine_revision() {
        assert_eq!(ENGINE_REVISION, "lightningcss-1.33.0-c6a0c3ce-sheetom.6");
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
    fn rejects_oversized_inputs_before_parsing() {
        let source = "x".repeat(MAX_DECLARATION_BYTES + 1);

        assert_eq!(
            canonicalize_declaration_block(&source),
            Err(EngineError::InputLimitExceeded {
                actual: MAX_DECLARATION_BYTES + 1,
                limit: MAX_DECLARATION_BYTES,
            })
        );
    }

    #[test]
    fn accepts_the_exact_declaration_budget() {
        let source = format!("--x: {}", "x".repeat(MAX_DECLARATION_BYTES - 5));

        assert!(canonicalize_declaration_block(&source).is_ok());
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
        let source = format!("--x: {}value", "fn(".repeat(MAX_NESTING_DEPTH + 1));

        assert_eq!(
            canonicalize_declaration_block(&source),
            Err(EngineError::NestingLimitExceeded {
                actual: MAX_NESTING_DEPTH + 1,
                limit: MAX_NESTING_DEPTH,
            })
        );
    }

    #[test]
    fn rejects_too_many_declarations_before_parsing() {
        let source = "x:;".repeat(MAX_DECLARATIONS_PER_BLOCK + 1);

        assert_eq!(
            canonicalize_declaration_block(&source),
            Err(EngineError::DeclarationLimitExceeded {
                actual: MAX_DECLARATIONS_PER_BLOCK + 1,
                limit: MAX_DECLARATIONS_PER_BLOCK,
            })
        );
    }
}
