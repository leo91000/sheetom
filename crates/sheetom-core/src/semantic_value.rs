use std::sync::Arc;

use lightningcss::{
    properties::{Property, PropertyId},
    stylesheet::{ParserOptions, PrinterOptions},
    traits::IntoOwned,
};

use crate::{
    analyze_recovered_substitutions,
    catalog::{property_grammar, PropertyGrammarOwner},
    extension_value::{
        is_numeric_extension_candidate, parse_extension_value, parse_preferred_extension_value,
    },
    font_face::FontFaceDescriptorValue,
    recover_component_values_with_limits, EngineError, PropertyParseKind, RecoveredValue,
    ResourceLimits, SemanticExtensionValue, SemanticSubstitutionValue,
};

#[derive(Clone, Debug, PartialEq)]
pub enum SemanticPropertyValue {
    Standard(Property<'static>),
    Extension(SemanticExtensionValue),
    /// A shorthand whose grammar was validated while producing its complete
    /// semantic longhand set. The recovered component tree is retained only as
    /// CSSOM provenance for reconstructing the shorthand spelling.
    ExpandedShorthand,
    FontFaceDescriptor(FontFaceDescriptorValue),
    PendingSubstitution(SemanticSubstitutionValue),
    CustomTokenStream,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SemanticDeclaration {
    property_name: Arc<str>,
    value: SemanticPropertyValue,
    recovered: RecoveredValue,
    parse_kind: PropertyParseKind,
}

impl SemanticDeclaration {
    pub(crate) fn from_standard_property(
        property_name: &str,
        value: Property<'static>,
        canonical_source: String,
    ) -> Self {
        Self {
            property_name: Arc::from(property_name),
            value: SemanticPropertyValue::Standard(value),
            recovered: RecoveredValue::compacted_explicit_source(canonical_source),
            parse_kind: PropertyParseKind::Typed,
        }
    }

    pub(crate) fn from_validated_expanded_shorthand(
        property_name: &str,
        source: &str,
        limits: ResourceLimits,
    ) -> Result<Self, EngineError> {
        Ok(Self {
            property_name: Arc::from(property_name),
            value: SemanticPropertyValue::ExpandedShorthand,
            recovered: recover_component_values_with_limits(source, limits)?,
            parse_kind: PropertyParseKind::SheetomTyped,
        })
    }

    pub(crate) fn from_font_face_descriptor(
        descriptor_name: &str,
        value: FontFaceDescriptorValue,
        recovered: RecoveredValue,
    ) -> Self {
        Self {
            property_name: Arc::from(descriptor_name),
            value: SemanticPropertyValue::FontFaceDescriptor(value),
            recovered,
            parse_kind: PropertyParseKind::SheetomTyped,
        }
    }

    pub(crate) fn compact_recovery(&mut self) {
        if !matches!(
            self.value,
            SemanticPropertyValue::Standard(_)
                | SemanticPropertyValue::Extension(_)
                | SemanticPropertyValue::FontFaceDescriptor(_)
        ) {
            return;
        }
        self.recovered.compact_if_fully_explicit();
    }

    pub fn property_name(&self) -> &str {
        &self.property_name
    }

    pub fn value(&self) -> &SemanticPropertyValue {
        &self.value
    }

    pub fn recovered(&self) -> &RecoveredValue {
        &self.recovered
    }

    pub fn parse_kind(&self) -> PropertyParseKind {
        self.parse_kind
    }

    pub fn canonical_value(&self) -> Result<String, EngineError> {
        match &self.value {
            SemanticPropertyValue::Standard(property) => property
                .value_to_css_string(PrinterOptions::default())
                .map_err(|error| EngineError::Serialize(error.to_string())),
            SemanticPropertyValue::Extension(value) => value.canonical_value(),
            SemanticPropertyValue::ExpandedShorthand => {
                self.recovered.reparsable_css_without_comments()
            }
            SemanticPropertyValue::FontFaceDescriptor(value) => value.canonical_value(),
            SemanticPropertyValue::PendingSubstitution(_)
            | SemanticPropertyValue::CustomTokenStream => self.recovered.reparsable_css(),
        }
    }
}

pub fn parse_semantic_property(
    name: &str,
    source: &str,
) -> Result<SemanticDeclaration, EngineError> {
    parse_semantic_property_with_limits(name, source, ResourceLimits::default())
}

pub fn parse_semantic_property_with_limits(
    name: &str,
    source: &str,
    limits: ResourceLimits,
) -> Result<SemanticDeclaration, EngineError> {
    let grammar = property_grammar(name)
        .ok_or_else(|| EngineError::Parse(format!("unsupported property: {name}")))?;
    let recovered = recover_component_values_with_limits(source, limits)?;
    let substitution = analyze_recovered_substitutions(&recovered)?;

    if grammar.owner() == PropertyGrammarOwner::CustomTokenStream {
        return Ok(SemanticDeclaration {
            property_name: Arc::from(grammar.canonical_name()),
            value: SemanticPropertyValue::CustomTokenStream,
            recovered,
            parse_kind: PropertyParseKind::Custom,
        });
    }
    if let Some(substitution) = substitution {
        return Ok(SemanticDeclaration {
            property_name: Arc::from(grammar.canonical_name()),
            value: SemanticPropertyValue::PendingSubstitution(substitution),
            recovered,
            parse_kind: PropertyParseKind::Unparsed,
        });
    }

    if grammar
        .extensions()
        .contains(&crate::catalog::PropertyGrammarExtension::BrowserLonghand)
    {
        let value = parse_extension_value(
            &[crate::catalog::PropertyGrammarExtension::BrowserLonghand],
            grammar.canonical_name(),
            source,
        )?
        .ok_or_else(|| unsupported_grammar_error(&grammar, source))?;
        return Ok(SemanticDeclaration {
            property_name: Arc::from(grammar.canonical_name()),
            value: SemanticPropertyValue::Extension(value),
            recovered,
            parse_kind: PropertyParseKind::SheetomTyped,
        });
    }

    if grammar
        .extensions()
        .contains(&crate::catalog::PropertyGrammarExtension::WebkitPerspective)
    {
        let value = parse_extension_value(grammar.extensions(), name, source)?
            .ok_or_else(|| unsupported_grammar_error(&grammar, source))?;
        return Ok(SemanticDeclaration {
            property_name: Arc::from(grammar.canonical_name()),
            value: SemanticPropertyValue::Extension(value),
            recovered,
            parse_kind: PropertyParseKind::SheetomTyped,
        });
    }

    let owns_numeric_candidate = grammar.extensions().iter().any(|extension| {
        matches!(
            extension,
            crate::catalog::PropertyGrammarExtension::LengthPercentageNumberCalculation
                | crate::catalog::PropertyGrammarExtension::LengthPercentageOrNumberCalculation
        )
    }) && is_numeric_extension_candidate(source);
    if owns_numeric_candidate {
        let value = parse_extension_value(grammar.extensions(), grammar.canonical_name(), source)?
            .ok_or_else(|| unsupported_grammar_error(&grammar, source))?;
        return Ok(SemanticDeclaration {
            property_name: Arc::from(grammar.canonical_name()),
            value: SemanticPropertyValue::Extension(value),
            recovered,
            parse_kind: PropertyParseKind::SheetomTyped,
        });
    }

    let extension_property_name = grammar.canonical_name();
    if let Some(value) =
        parse_preferred_extension_value(grammar.extensions(), extension_property_name, source)
    {
        return Ok(SemanticDeclaration {
            property_name: Arc::from(grammar.canonical_name()),
            value: SemanticPropertyValue::Extension(value),
            recovered,
            parse_kind: PropertyParseKind::SheetomTyped,
        });
    }

    let standard = if grammar.has_standard_parser() {
        match parse_standard_value(&grammar, source) {
            Ok(value) => {
                let parse_kind = if grammar.owner() == PropertyGrammarOwner::SheetomAlias {
                    PropertyParseKind::SheetomTyped
                } else {
                    PropertyParseKind::Typed
                };
                return Ok(SemanticDeclaration {
                    property_name: Arc::from(grammar.canonical_name()),
                    value: SemanticPropertyValue::Standard(value),
                    recovered,
                    parse_kind,
                });
            }
            Err(error) => Some(error),
        }
    } else {
        None
    };

    if let Some(value) =
        parse_extension_value(grammar.extensions(), extension_property_name, source)?
    {
        return Ok(SemanticDeclaration {
            property_name: Arc::from(grammar.canonical_name()),
            value: SemanticPropertyValue::Extension(value),
            recovered,
            parse_kind: PropertyParseKind::SheetomTyped,
        });
    }

    Err(standard.unwrap_or_else(|| unsupported_grammar_error(&grammar, source)))
}

pub fn parse_standard_semantic_property(
    name: &str,
    source: &str,
) -> Result<SemanticDeclaration, EngineError> {
    parse_standard_semantic_property_with_limits(name, source, ResourceLimits::default())
}

pub fn parse_standard_semantic_property_with_limits(
    name: &str,
    source: &str,
    limits: ResourceLimits,
) -> Result<SemanticDeclaration, EngineError> {
    let declaration = parse_semantic_property_with_limits(name, source, limits)?;
    if matches!(declaration.value(), SemanticPropertyValue::Standard(_)) {
        return Ok(declaration);
    }
    Err(EngineError::Parse(format!(
        "property requires non-standard semantics: {name}: {source}"
    )))
}

fn parse_standard_value(
    grammar: &crate::catalog::PropertyGrammar,
    source: &str,
) -> Result<Property<'static>, EngineError> {
    let property = Property::parse_string(
        PropertyId::from(grammar.parser_name()),
        source,
        ParserOptions::default(),
    )
    .map_err(|error| EngineError::Parse(error.to_string()))?;

    if matches!(property, Property::Unparsed(_) | Property::Custom(_)) {
        return Err(EngineError::Parse(format!(
            "property requires a non-standard or pending grammar: {}: {source}",
            grammar.canonical_name()
        )));
    }

    Ok(property.into_owned())
}

fn unsupported_grammar_error(
    grammar: &crate::catalog::PropertyGrammar,
    source: &str,
) -> EngineError {
    let requirement = if grammar.extensions().is_empty() {
        "an unimplemented grammar"
    } else {
        "a SheetOM extension grammar"
    };
    EngineError::Parse(format!(
        "property requires {requirement}: {}: {source}",
        grammar.canonical_name()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RecoveredComponentKind;

    #[test]
    fn owns_typed_values_after_the_parser_input_is_dropped() {
        let declaration = {
            let name = String::from("background");
            let source = String::from(
                "image-set(url(a.png) 1x, url(b.png) 2x) center / cover no-repeat red",
            );
            parse_standard_semantic_property(&name, &source).unwrap()
        };

        assert_eq!(declaration.property_name(), "background");
        assert_eq!(declaration.parse_kind(), PropertyParseKind::Typed);
        assert_eq!(
            declaration.canonical_value().unwrap(),
            "red image-set(\"a.png\" 1x, \"b.png\" 2x) center / cover no-repeat"
        );
        assert!(matches!(
            declaration.value(),
            SemanticPropertyValue::Standard(Property::Background(_))
        ));
    }

    #[test]
    fn parses_subgrid_through_the_vendored_standard_property_ast() {
        for (source, expected) in [
            ("subgrid", "subgrid"),
            ("SUBGRID", "subgrid"),
            ("subgrid []", "subgrid []"),
            ("subgrid [a b] [c]", "subgrid [a b] [c]"),
            (
                "subgrid [a] repeat(2, [b] [c d]) [e]",
                "subgrid [a] repeat(2, [b] [c d]) [e]",
            ),
            (
                "subgrid repeat(auto-fill, [column])",
                "subgrid repeat(auto-fill, [column])",
            ),
        ] {
            let declaration =
                parse_standard_semantic_property("grid-template-columns", source).unwrap();
            assert!(matches!(
                declaration.value(),
                SemanticPropertyValue::Standard(Property::GridTemplateColumns(_))
            ));
            assert_eq!(declaration.canonical_value().unwrap(), expected, "{source}");
        }

        for source in [
            "subgrid [span]",
            "subgrid [auto]",
            "subgrid [initial]",
            "subgrid repeat(auto-fit, [a])",
            "subgrid repeat(0, [a])",
            "subgrid repeat(2, 1fr)",
            "subgrid repeat(2, [span])",
        ] {
            assert!(
                parse_standard_semantic_property("grid-template-columns", source).is_err(),
                "{source}"
            );
        }
    }

    #[test]
    fn falls_back_to_owned_layout_extension_values_after_standard_parsing() {
        for (name, source, expected) in [
            ("z-index", "calc(1 + 1)", "calc(2)"),
            (
                "offset-position",
                "top 20px left 10px",
                "left 10px top 20px",
            ),
            ("offset-rotate", "10deg reverse", "reverse 10deg"),
            ("size", "landscape A4", "a4 landscape"),
        ] {
            let declaration = parse_semantic_property(name, source).unwrap();
            assert_eq!(declaration.parse_kind(), PropertyParseKind::SheetomTyped);
            assert!(matches!(
                declaration.value(),
                SemanticPropertyValue::Extension(_)
            ));
            assert_eq!(declaration.canonical_value().unwrap(), expected, "{name}");
            assert!(parse_standard_semantic_property(name, source).is_err());
        }
    }

    #[test]
    fn retains_recovery_evidence_beside_the_semantic_value() {
        let declaration = parse_standard_semantic_property("font-family", "\"Gotham").unwrap();
        let token = &declaration.recovered().values()[0];

        assert_eq!(declaration.recovered().source(), "\"Gotham");
        assert!(matches!(token.kind, RecoveredComponentKind::Token(_)));
        assert_eq!(declaration.canonical_value().unwrap(), "Gotham");
    }

    #[test]
    fn retains_explicit_nested_structure_without_reparsing_strings() {
        let declaration = parse_standard_semantic_property("width", "calc(100% - 2rem)").unwrap();
        let RecoveredComponentKind::Function { closure, .. } =
            &declaration.recovered().values()[0].kind
        else {
            panic!("expected recovered calc function")
        };

        assert_eq!(*closure, crate::RecoveredClosure::Explicit);
        assert_eq!(declaration.canonical_value().unwrap(), "calc(100% - 2rem)");
    }

    #[test]
    fn keeps_alias_ownership_separate_from_the_typed_parser_property() {
        let declaration =
            parse_standard_semantic_property("-webkit-column-rule", "2px dashed red").unwrap();

        assert_eq!(declaration.property_name(), "column-rule");
        assert_eq!(declaration.parse_kind(), PropertyParseKind::SheetomTyped);
        assert_eq!(declaration.canonical_value().unwrap(), "2px dashed red");
    }

    #[test]
    fn rejects_unparsed_and_custom_candidates_in_the_standard_path() {
        for (name, source) in [
            ("width", "var(--width)"),
            ("content", "leader(.)"),
            ("--theme", "red"),
        ] {
            assert!(
                parse_standard_semantic_property(name, source).is_err(),
                "{name}: {source}"
            );
        }
    }

    #[test]
    fn parses_anchor_size_through_the_vendored_standard_ast() {
        for (source, expected) in [
            ("anchor-size()", "anchor-size()"),
            ("anchor-size(width --card)", "anchor-size(--card width)"),
            (
                "calc(anchor-size(width) * 2)",
                "calc(2 * anchor-size(width))",
            ),
            (
                "min(anchor-size(width), 10px)",
                "min(anchor-size(width), 10px)",
            ),
        ] {
            let declaration = parse_standard_semantic_property("width", source).unwrap();
            assert_eq!(declaration.parse_kind(), PropertyParseKind::Typed);
            assert!(matches!(
                declaration.value(),
                SemanticPropertyValue::Standard(_)
            ));
            assert_eq!(declaration.canonical_value().unwrap(), expected, "{source}");
        }
    }

    #[test]
    fn parses_contrast_color_in_every_standard_color_slot() {
        for (name, source) in [
            ("-webkit-tap-highlight-color", "contrast-color(red)"),
            (
                "-webkit-text-fill-color",
                "light-dark(contrast-color(red), blue)",
            ),
            ("color", "contrast-color(red)"),
            (
                "color",
                "color-mix(in srgb, contrast-color(red), currentColor)",
            ),
            ("background-color", "contrast-color(light-dark(red, blue))"),
            (
                "background-image",
                "linear-gradient(contrast-color(red), blue)",
            ),
            ("border-color", "red contrast-color(blue) green"),
            ("box-shadow", "0 0 contrast-color(red)"),
            (
                "flood-color",
                "color-mix(in srgb, contrast-color(red), currentColor)",
            ),
            ("lighting-color", "contrast-color(red)"),
            ("scrollbar-color", "contrast-color(red) blue"),
            ("stop-color", "contrast-color(red)"),
        ] {
            let declaration = parse_standard_semantic_property(name, source).unwrap();
            assert_eq!(declaration.parse_kind(), PropertyParseKind::Typed);
            assert!(matches!(
                declaration.value(),
                SemanticPropertyValue::Standard(_)
            ));
            assert!(
                declaration
                    .canonical_value()
                    .unwrap()
                    .contains("contrast-color("),
                "{name}: {source}"
            );
        }

        for source in [
            "contrast-color()",
            "contrast-color(red blue)",
            "contrast-color(red, blue)",
        ] {
            assert!(
                parse_standard_semantic_property("color", source).is_err(),
                "{source}"
            );
        }
    }

    #[test]
    fn matches_the_complete_chromium_number_result_math_corpus() {
        let corpus: serde_json::Value = serde_json::from_str(include_str!(
            "../../../compatibility/number-result-math-capabilities.json"
        ))
        .unwrap();
        let cases = corpus["cases"].as_array().unwrap();

        for candidate in cases {
            let id = candidate["id"].as_str().unwrap();
            let name = candidate["property"].as_str().unwrap();
            let source = candidate["input"].as_str().unwrap();
            let expected = candidate["accepted"].as_bool().unwrap();
            if candidate["integration"].as_str().unwrap() == "composite-property" {
                continue;
            }
            let declaration = parse_semantic_property(name, source);
            assert_eq!(declaration.is_ok(), expected, "{id}");
            if !expected {
                continue;
            }

            assert_eq!(
                declaration.unwrap().canonical_value().unwrap(),
                candidate["observable"].as_str().unwrap(),
                "{id}"
            );
        }
    }

    #[test]
    fn parses_relative_colors_through_the_vendored_standard_ast() {
        for (source, expected) in [
            (
                "rgb(from rgb(20%, 40%, 60%, 80%) r g b / alpha)",
                "rgb(from rgba(51, 102, 153, 0.8) r g b / alpha)",
            ),
            (
                "rgba(from rebeccapurple r calc(g * .5 + g * .5) 10)",
                "rgb(from rebeccapurple r calc((0.5 * g) + (0.5 * g)) 10)",
            ),
            (
                "lab(from lab(50 -30 40) l calc(a / 3) calc(b / 2))",
                "lab(from lab(50 -30 40) l calc(0.333333 * a) calc(0.5 * b))",
            ),
            (
                "lch(from lch(none none none / none) l c h / alpha)",
                "lch(from lch(none none none / none) l c h / alpha)",
            ),
            (
                "color(from color(display-p3-linear .7 .5 .3) display-p3-linear r g b)",
                "color(from color(display-p3-linear 0.7 0.5 0.3) display-p3-linear r g b)",
            ),
        ] {
            let declaration = parse_standard_semantic_property("color", source).unwrap();
            assert_eq!(declaration.parse_kind(), PropertyParseKind::Typed);
            assert!(matches!(
                declaration.value(),
                SemanticPropertyValue::Standard(_)
            ));
            assert_eq!(declaration.canonical_value().unwrap(), expected, "{source}");
        }

        for source in [
            "rgb(from rebeccapurple r g)",
            "rgb(from rebeccapurple r calc(g +1) b)",
            "hsl(from rebeccapurple calc(h + 1deg) s l)",
            "color(from red display-p3 x y z)",
        ] {
            assert!(
                parse_standard_semantic_property("color", source).is_err(),
                "{source}"
            );
        }
    }

    #[test]
    fn owns_pending_substitutions_without_using_the_lightning_unparsed_variant() {
        let declaration =
            parse_semantic_property("padding", "72px var(--space, var(--space,").unwrap();

        assert_eq!(declaration.parse_kind(), PropertyParseKind::Unparsed);
        assert!(matches!(
            declaration.value(),
            SemanticPropertyValue::PendingSubstitution(substitution)
                if substitution.functions().len() == 2
        ));
        assert_eq!(
            declaration.canonical_value().unwrap(),
            "72px var(--space, var(--space, ))"
        );
    }

    #[test]
    fn classifies_attr_if_and_custom_functions_from_recovered_tokens() {
        for (name, source, expected_functions) in [
            ("width", "attr(data-width type(<length>), 1px)", 1),
            ("color", "if(style(--theme: dark): white; else: black)", 1),
            ("width", "calc(--double(1px) + var(--base))", 2),
            ("content", "var(--content", 1),
        ] {
            let declaration = parse_semantic_property(name, source).unwrap();
            assert!(matches!(
                declaration.value(),
                SemanticPropertyValue::PendingSubstitution(substitution)
                    if substitution.functions().len() == expected_functions
            ));
        }
    }

    #[test]
    fn owns_custom_property_tokens_and_repairs_only_reparsable_output() {
        let declaration = parse_semantic_property("--Theme", "\"dark").unwrap();

        assert_eq!(declaration.property_name(), "--Theme");
        assert_eq!(declaration.parse_kind(), PropertyParseKind::Custom);
        assert!(matches!(
            declaration.value(),
            SemanticPropertyValue::CustomTokenStream
        ));
        assert_eq!(declaration.recovered().source(), "\"dark");
        assert_eq!(declaration.canonical_value().unwrap(), "\"dark\"");
    }

    #[test]
    fn rejects_invalid_substitutions_before_any_property_parser() {
        for (name, source) in [
            ("width", "var(foo)"),
            ("width", "attr()"),
            ("color", "if()"),
            ("padding", "--spacing(1px,)"),
            ("--theme", "red; color: blue"),
        ] {
            assert!(
                parse_semantic_property(name, source).is_err(),
                "{name}: {source}"
            );
        }
    }

    #[test]
    fn enforces_recovery_resource_limits_before_semantic_parsing() {
        let limits = ResourceLimits {
            max_nesting_depth: 1,
            ..ResourceLimits::default()
        };
        assert_eq!(
            parse_standard_semantic_property_with_limits("width", "calc((1px))", limits),
            Err(EngineError::NestingLimitExceeded {
                actual: 2,
                limit: 1,
            })
        );
    }
}
