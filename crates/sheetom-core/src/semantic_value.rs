use std::sync::Arc;

use lightningcss::{
    properties::{Property, PropertyId},
    stylesheet::{ParserOptions, PrinterOptions},
    traits::IntoOwned,
};

use crate::{
    analyze_recovered_substitutions,
    catalog::{property_grammar, PropertyGrammarOwner},
    recover_component_values_with_limits, EngineError, PropertyParseKind, RecoveredValue,
    ResourceLimits, SemanticSubstitutionValue,
};

#[derive(Clone, Debug, PartialEq)]
pub enum SemanticPropertyValue {
    Standard(Property<'static>),
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

    parse_standard_with_recovered(grammar, source, recovered)
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

fn parse_standard_with_recovered(
    grammar: crate::catalog::PropertyGrammar,
    source: &str,
    recovered: RecoveredValue,
) -> Result<SemanticDeclaration, EngineError> {
    if !grammar.has_standard_parser() {
        let requirement = if grammar.extensions().is_empty() {
            "an unimplemented grammar"
        } else {
            "a SheetOM extension grammar"
        };
        return Err(EngineError::Parse(format!(
            "property requires {requirement}: {}: {source}",
            grammar.canonical_name()
        )));
    }

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

    let parse_kind = if grammar.owner() == PropertyGrammarOwner::SheetomAlias {
        PropertyParseKind::SheetomTyped
    } else {
        PropertyParseKind::Typed
    };
    let property = property.into_owned();

    Ok(SemanticDeclaration {
        property_name: Arc::from(grammar.canonical_name()),
        value: SemanticPropertyValue::Standard(property),
        recovered,
        parse_kind,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{RecoveredClosure, RecoveredComponentKind};

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

        assert_eq!(*closure, RecoveredClosure::Explicit);
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
            ("width", "anchor-size(width)"),
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
            "72px var(--space, var(--space,))"
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
