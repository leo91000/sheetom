use crate::{
    browser_longhand::{parse_browser_longhand, BrowserLonghandValue},
    shorthand::ParsedValue,
    syntax::analyze_substitutions,
    DeclarationValue, EngineError, ResourceLimits, SemanticDeclaration,
};
use cssparser::{Parser, ParserInput};
use lightningcss::{
    rules::{font_face::FontFaceProperty, CssRule},
    stylesheet::{ParserOptions, PrinterOptions, StyleSheet},
    traits::{IntoOwned, Parse, ToCss, TrySign},
    values::{calc::Calc, percentage::Percentage},
};

const DESCRIPTORS: &[&str] = &[
    "ascent-override",
    "descent-override",
    "font-display",
    "font-family",
    "font-feature-settings",
    "font-stretch",
    "font-style",
    "font-variant",
    "font-variation-settings",
    "font-weight",
    "line-gap-override",
    "size-adjust",
    "src",
    "unicode-range",
];

#[derive(Clone, Debug, PartialEq)]
pub enum FontFaceDescriptorValue {
    Typed(FontFaceProperty<'static>),
    MetricOverride(FontFaceMetricOverride),
    BrowserLonghand(BrowserLonghandValue),
    Keyword(&'static str),
}

#[derive(Clone, Debug, PartialEq)]
pub enum FontFaceMetricOverride {
    Normal,
    Percentage {
        value: Calc<Percentage>,
        wrap_calc: bool,
    },
}

impl FontFaceDescriptorValue {
    pub(crate) fn canonical_value(&self) -> Result<String, EngineError> {
        match self {
            FontFaceDescriptorValue::Typed(value) => typed_descriptor_value_to_css(value),
            FontFaceDescriptorValue::MetricOverride(FontFaceMetricOverride::Normal) => {
                Ok("normal".to_owned())
            }
            FontFaceDescriptorValue::MetricOverride(FontFaceMetricOverride::Percentage {
                value,
                wrap_calc,
            }) => {
                let serialized = serialize_descriptor_component(value)?;
                if *wrap_calc && matches!(value, Calc::Number(_) | Calc::Value(_)) {
                    return Ok(format!("calc({serialized})"));
                }
                Ok(serialized)
            }
            FontFaceDescriptorValue::BrowserLonghand(value) => value.canonical_value(),
            FontFaceDescriptorValue::Keyword(value) => Ok((*value).to_owned()),
        }
    }
}

pub(crate) fn canonical_descriptor_name(name: &str) -> Option<String> {
    if name.starts_with("--") {
        return (name.len() > 2).then(|| name.to_owned());
    }
    let name = name.to_ascii_lowercase();
    DESCRIPTORS
        .binary_search(&name.as_str())
        .is_ok()
        .then_some(name)
}

pub(crate) fn parse_descriptor_value(
    name: &str,
    value: &str,
    limits: ResourceLimits,
) -> Option<ParsedValue> {
    let substitutions = analyze_substitutions(value);
    if !substitutions.valid {
        return None;
    }
    if name.starts_with("--") {
        return crate::shorthand::parse_value_with_limits(name, value, false, limits).ok();
    }
    if substitutions.found {
        return None;
    }

    let descriptor = match name {
        "ascent-override" | "descent-override" | "line-gap-override" => {
            parse_metric_override(value, true)?
        }
        "size-adjust" => parse_metric_override(value, false)?,
        "font-display" => parse_keyword(value, &["auto", "block", "swap", "fallback", "optional"])
            .map(FontFaceDescriptorValue::Keyword)?,
        "font-feature-settings" | "font-variation-settings" => {
            FontFaceDescriptorValue::BrowserLonghand(parse_browser_longhand(name, value).ok()??)
        }
        "font-variant" => {
            parse_keyword(value, &["normal", "small-caps"]).map(FontFaceDescriptorValue::Keyword)?
        }
        "font-family" | "font-stretch" | "font-style" | "font-weight" | "src" | "unicode-range" => {
            parse_typed_descriptor(name, value)?
        }
        _ => return None,
    };
    let recovered = crate::recover_component_values_with_limits(value, limits).ok()?;
    let semantic = SemanticDeclaration::from_font_face_descriptor(name, descriptor, recovered);

    Some(ParsedValue {
        value: DeclarationValue::semantic(semantic).ok()?,
        longhands: None,
    })
}

fn parse_typed_descriptor(name: &str, value: &str) -> Option<FontFaceDescriptorValue> {
    let source = format!("@font-face{{{name}:{value}}}");
    let sheet = StyleSheet::parse(&source, ParserOptions::default()).ok()?;
    let CssRule::FontFace(rule) = sheet.rules.0.first()? else {
        return None;
    };
    let property = rule.properties.first()?;
    if !typed_variant_matches(name, property) {
        return None;
    }
    Some(FontFaceDescriptorValue::Typed(
        property.clone().into_owned(),
    ))
}

fn typed_descriptor_value_to_css(value: &FontFaceProperty<'static>) -> Result<String, EngineError> {
    match value {
        FontFaceProperty::Source(sources) => {
            let mut serialized = Vec::with_capacity(sources.len());
            for source in sources {
                serialized.push(serialize_descriptor_component(source)?);
            }
            Ok(serialized.join(", "))
        }
        FontFaceProperty::FontFamily(value) => serialize_descriptor_component(value),
        FontFaceProperty::FontStyle(value) => serialize_descriptor_component(value),
        FontFaceProperty::FontWeight(value) => serialize_descriptor_component(value),
        FontFaceProperty::FontStretch(value) => serialize_descriptor_component(value),
        FontFaceProperty::UnicodeRange(ranges) => Ok(ranges
            .iter()
            .map(|range| {
                if range.start == range.end {
                    format!("U+{:X}", range.start)
                } else {
                    format!("U+{:X}-{:X}", range.start, range.end)
                }
            })
            .collect::<Vec<_>>()
            .join(", ")),
        FontFaceProperty::Custom(_) => Err(EngineError::Serialize(
            "unsupported typed font-face descriptor".to_owned(),
        )),
    }
}

fn serialize_descriptor_component<T: ToCss>(value: &T) -> Result<String, EngineError> {
    value
        .to_css_string(PrinterOptions::default())
        .map_err(|error| EngineError::Serialize(error.to_string()))
}

fn typed_variant_matches(name: &str, property: &FontFaceProperty<'_>) -> bool {
    matches!(
        (name, property),
        ("src", FontFaceProperty::Source(_))
            | ("font-family", FontFaceProperty::FontFamily(_))
            | ("font-style", FontFaceProperty::FontStyle(_))
            | ("font-weight", FontFaceProperty::FontWeight(_))
            | ("font-stretch", FontFaceProperty::FontStretch(_))
            | ("unicode-range", FontFaceProperty::UnicodeRange(_))
    )
}

fn parse_metric_override(value: &str, allows_normal: bool) -> Option<FontFaceDescriptorValue> {
    let mut input = ParserInput::new(value);
    let mut parser = Parser::new(&mut input);
    if allows_normal
        && parser
            .try_parse(|input| input.expect_ident_matching("normal"))
            .is_ok()
    {
        parser.expect_exhausted().ok()?;
        return Some(FontFaceDescriptorValue::MetricOverride(
            FontFaceMetricOverride::Normal,
        ));
    }
    let state = parser.state();
    let wrap_calc = parser
        .expect_function()
        .is_ok_and(|function| function.eq_ignore_ascii_case("calc"));
    parser.reset(&state);
    let percentage = match parser.try_parse(Calc::<Percentage>::parse) {
        Ok(value) => value,
        Err(_) => Percentage::parse(&mut parser).ok()?.into(),
    };
    parser.expect_exhausted().ok()?;
    if matches!(&percentage, Calc::Number(_))
        || percentage.try_sign().is_some_and(|sign| sign < 0.0)
    {
        return None;
    }
    Some(FontFaceDescriptorValue::MetricOverride(
        FontFaceMetricOverride::Percentage {
            value: percentage,
            wrap_calc,
        },
    ))
}

fn parse_keyword(value: &str, keywords: &'static [&'static str]) -> Option<&'static str> {
    let mut input = ParserInput::new(value);
    let mut parser = Parser::new(&mut input);
    let identifier = parser.expect_ident_cloned().ok()?;
    parser.expect_exhausted().ok()?;
    keywords
        .iter()
        .copied()
        .find(|keyword| identifier.eq_ignore_ascii_case(keyword))
}

#[cfg(test)]
mod tests {
    use super::{canonical_descriptor_name, parse_descriptor_value};
    use crate::{DeclarationValueKind, ResourceLimits, SemanticPropertyValue};

    fn parse(name: &str, value: &str) -> Option<crate::shorthand::ParsedValue> {
        parse_descriptor_value(name, value, ResourceLimits::default())
    }

    #[test]
    fn accepts_and_canonicalizes_chromium_font_face_descriptors() {
        let cases = [
            ("font-family", "Test", "Test"),
            ("src", "url(test.woff2)", "url(\"test.woff2\")"),
            ("unicode-range", "U+??", "U+0-FF"),
            ("font-display", "SWAP", "swap"),
            ("ascent-override", "1e2%", "100%"),
            ("font-feature-settings", "\"kern\" 1", "\"kern\""),
        ];
        for (name, input, expected) in cases {
            let parsed =
                parse(name, input).unwrap_or_else(|| panic!("{name} descriptor should parse"));
            assert_eq!(parsed.safe_value(), expected, "{name}");
        }
    }

    #[test]
    fn every_ordinary_descriptor_retains_owned_semantic_state() {
        let cases = [
            ("ascent-override", "90%"),
            ("descent-override", "calc(20%)"),
            ("font-display", "swap"),
            ("font-family", "Test"),
            ("font-feature-settings", "\"kern\""),
            ("font-stretch", "75% 125%"),
            ("font-style", "oblique 10deg 20deg"),
            ("font-variant", "small-caps"),
            ("font-variation-settings", "\"wght\" 500"),
            ("font-weight", "100 900"),
            ("line-gap-override", "normal"),
            ("size-adjust", "100%"),
            ("src", "local(Test), url(test.woff2)"),
            ("unicode-range", "U+??"),
        ];
        for (name, input) in cases {
            let parsed = parse(name, input)
                .unwrap_or_else(|| panic!("{name} descriptor should parse semantically"));
            assert_eq!(
                parsed.value.kind(),
                DeclarationValueKind::Semantic,
                "{name}"
            );
            assert!(
                matches!(
                    parsed.value.semantic_value().map(|value| value.value()),
                    Some(SemanticPropertyValue::FontFaceDescriptor(_))
                ),
                "{name}"
            );
        }
    }

    #[test]
    fn rejects_unknown_invalid_and_substituted_descriptors() {
        assert_eq!(canonical_descriptor_name("unknown"), None);
        assert!(parse("font-display", "initial").is_none());
        assert!(parse("font-family", "A, B").is_none());
        assert!(parse("src", "none").is_none());
        assert!(parse("font-stretch", "condensed expanded").is_none());
        assert!(parse("size-adjust", "-1%").is_none());
        assert!(parse("font-display", "var(--x)").is_none());
    }

    #[test]
    fn preserves_custom_properties_and_font_variant_observability() {
        let custom = parse("--x", "var(--y").expect("custom descriptor");
        assert_eq!(custom.observable_value(), "var(--y");
        let variant = parse("font-variant", "small-caps").expect("variant");
        assert_eq!(variant.observable_value(), "");
        assert_eq!(variant.safe_value(), "small-caps");
    }
}
