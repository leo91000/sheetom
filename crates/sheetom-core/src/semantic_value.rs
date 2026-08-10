use std::sync::Arc;

use lightningcss::{
    properties::{Property, PropertyId},
    stylesheet::{ParserOptions, PrinterOptions},
    traits::IntoOwned,
};

use crate::{
    catalog::canonical_property_name, recover_component_values_with_limits,
    sheetom_parser_property_name, EngineError, PropertyParseKind, RecoveredValue, ResourceLimits,
};

#[derive(Clone, Debug, PartialEq)]
pub enum SemanticPropertyValue {
    Standard(Property<'static>),
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
        }
    }
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
    let property_name = canonical_property_name(name)
        .ok_or_else(|| EngineError::Parse(format!("unsupported property: {name}")))?;
    if property_name.starts_with("--") {
        return Err(EngineError::Parse(format!(
            "custom property requires token-stream semantics: {name}"
        )));
    }

    let recovered = recover_component_values_with_limits(source, limits)?;
    let parser_name = sheetom_parser_property_name(&property_name).unwrap_or(&property_name);
    let property = Property::parse_string(
        PropertyId::from(parser_name),
        source,
        ParserOptions::default(),
    )
    .map_err(|error| EngineError::Parse(error.to_string()))?;

    if matches!(property, Property::Unparsed(_) | Property::Custom(_)) {
        return Err(EngineError::Parse(format!(
            "property requires a non-standard or pending grammar: {property_name}: {source}"
        )));
    }

    let parse_kind = if parser_name == property_name {
        PropertyParseKind::Typed
    } else {
        PropertyParseKind::SheetomTyped
    };
    let property = property.into_owned();

    Ok(SemanticDeclaration {
        property_name: Arc::from(property_name),
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
