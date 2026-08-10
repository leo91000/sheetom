use crate::{
    shorthand::ParsedValue,
    syntax::{analyze_substitutions, split_top_level_delimiter},
};
use cssparser::{serialize_string, Parser, ParserInput};
use lightningcss::{
    rules::{font_face::FontFaceProperty, CssRule},
    stylesheet::{ParserOptions, PrinterOptions, StyleSheet},
    traits::{Parse, ToCss},
    values::percentage::Percentage,
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

pub(crate) fn parse_descriptor_value(name: &str, value: &str) -> Option<ParsedValue> {
    let substitutions = analyze_substitutions(value);
    if !substitutions.valid {
        return None;
    }
    if name.starts_with("--") {
        return crate::shorthand::parse_value(name, value, false).ok();
    }
    if substitutions.found {
        return None;
    }

    let (observable_value, safe_value) = match name {
        "ascent-override" | "descent-override" | "line-gap-override" => {
            parse_metric_override(value, true)?
        }
        "size-adjust" => parse_metric_override(value, false)?,
        "font-display" => parse_font_display(value)?,
        "font-feature-settings" => parse_font_feature_settings(value)?,
        "font-variation-settings" => parse_font_variation_settings(value)?,
        "font-variant" => parse_font_variant(value)?,
        "font-family" | "font-stretch" | "font-style" | "font-weight" | "src" | "unicode-range" => {
            parse_typed_descriptor(name, value)?
        }
        _ => return None,
    };

    Some(ParsedValue {
        observable_value,
        safe_value,
        longhands: None,
        pending_substitution: false,
    })
}

fn parse_typed_descriptor(name: &str, value: &str) -> Option<(String, String)> {
    let source = format!("@font-face{{{name}:{value}}}");
    let sheet = StyleSheet::parse(&source, ParserOptions::default()).ok()?;
    let CssRule::FontFace(rule) = sheet.rules.0.first()? else {
        return None;
    };
    let property = rule.properties.first()?;
    if !typed_variant_matches(name, property) {
        return None;
    }
    if let FontFaceProperty::UnicodeRange(ranges) = property {
        let serialized = ranges
            .iter()
            .map(|range| {
                if range.start == range.end {
                    format!("U+{:X}", range.start)
                } else {
                    format!("U+{:X}-{:X}", range.start, range.end)
                }
            })
            .collect::<Vec<_>>()
            .join(", ");
        return Some((serialized.clone(), serialized));
    }
    let declaration = property.to_css_string(PrinterOptions::default()).ok()?;
    let (_, serialized_value) = declaration.split_once(':')?;
    let serialized_value = serialized_value.trim().to_owned();
    let observable_value = if name == "font-family" && value.trim().starts_with(['\'', '"']) {
        canonical_string(value)?
    } else {
        serialized_value.clone()
    };
    Some((observable_value, serialized_value))
}

fn canonical_string(value: &str) -> Option<String> {
    let mut input = ParserInput::new(value);
    let mut parser = Parser::new(&mut input);
    let value = parser.expect_string_cloned().ok()?;
    parser.expect_exhausted().ok()?;
    let mut serialized = String::new();
    serialize_string(&value, &mut serialized).ok()?;
    Some(serialized)
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

fn parse_metric_override(value: &str, allows_normal: bool) -> Option<(String, String)> {
    let value = value.trim();
    if allows_normal && value.eq_ignore_ascii_case("normal") {
        return Some(("normal".to_owned(), "normal".to_owned()));
    }

    let mut input = ParserInput::new(value);
    let mut parser = Parser::new(&mut input);
    let percentage = Percentage::parse(&mut parser).ok()?;
    parser.expect_exhausted().ok()?;
    let is_math_function = value
        .as_bytes()
        .first()
        .is_some_and(u8::is_ascii_alphabetic);
    if !is_math_function && percentage.0 < 0.0 {
        return None;
    }
    let canonical = if is_math_function {
        value.replace(",", ", ")
    } else {
        percentage.to_css_string(PrinterOptions::default()).ok()?
    };
    Some((canonical.clone(), canonical))
}

fn parse_font_display(value: &str) -> Option<(String, String)> {
    let value = value.trim().to_ascii_lowercase();
    matches!(
        value.as_str(),
        "auto" | "block" | "swap" | "fallback" | "optional"
    )
    .then(|| (value.clone(), value))
}

fn parse_font_variant(value: &str) -> Option<(String, String)> {
    let value = value.trim().to_ascii_lowercase();
    matches!(value.as_str(), "normal" | "small-caps").then(|| (String::new(), value))
}

fn parse_font_feature_settings(value: &str) -> Option<(String, String)> {
    let value = value.trim();
    if value.eq_ignore_ascii_case("normal") {
        return Some(("normal".to_owned(), "normal".to_owned()));
    }
    let mut serialized = Vec::new();
    for entry in split_top_level_delimiter(value, b',')? {
        let mut input = ParserInput::new(entry);
        let mut parser = Parser::new(&mut input);
        let tag = parser.expect_string_cloned().ok()?;
        if !valid_opentype_tag(&tag) {
            return None;
        }
        let setting = if parser.is_exhausted()
            || parser
                .try_parse(|input| input.expect_ident_matching("on"))
                .is_ok()
        {
            1
        } else if parser
            .try_parse(|input| input.expect_ident_matching("off"))
            .is_ok()
        {
            0
        } else {
            parser.expect_integer().ok()?
        };
        parser.expect_exhausted().ok()?;
        if setting < 0 {
            return None;
        }
        let mut item = String::new();
        serialize_string(&tag, &mut item).ok()?;
        if setting != 1 {
            item.push(' ');
            item.push_str(&setting.to_string());
        }
        serialized.push(item);
    }
    let serialized = serialized.join(", ");
    Some((serialized.clone(), serialized))
}

fn parse_font_variation_settings(value: &str) -> Option<(String, String)> {
    let value = value.trim();
    if value.eq_ignore_ascii_case("normal") {
        return Some(("normal".to_owned(), "normal".to_owned()));
    }
    let mut serialized = Vec::new();
    for entry in split_top_level_delimiter(value, b',')? {
        let mut input = ParserInput::new(entry);
        let mut parser = Parser::new(&mut input);
        let tag = parser.expect_string_cloned().ok()?;
        if !valid_opentype_tag(&tag) {
            return None;
        }
        let setting = parser.expect_number().ok()?;
        parser.expect_exhausted().ok()?;
        if !setting.is_finite() {
            return None;
        }
        let mut item = String::new();
        serialize_string(&tag, &mut item).ok()?;
        item.push(' ');
        item.push_str(&serialize_number(setting));
        serialized.push(item);
    }
    let serialized = serialized.join(", ");
    Some((serialized.clone(), serialized))
}

fn valid_opentype_tag(tag: &str) -> bool {
    tag.len() == 4 && tag.bytes().all(|byte| (0x20..=0x7e).contains(&byte))
}

fn serialize_number(value: f32) -> String {
    if value.fract() == 0.0 {
        format!("{value:.0}")
    } else {
        value.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::{canonical_descriptor_name, parse_descriptor_value};

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
            let parsed = parse_descriptor_value(name, input)
                .unwrap_or_else(|| panic!("{name} descriptor should parse"));
            assert_eq!(parsed.safe_value, expected, "{name}");
        }
    }

    #[test]
    fn rejects_unknown_invalid_and_substituted_descriptors() {
        assert_eq!(canonical_descriptor_name("unknown"), None);
        assert!(parse_descriptor_value("font-display", "initial").is_none());
        assert!(parse_descriptor_value("font-family", "A, B").is_none());
        assert!(parse_descriptor_value("src", "none").is_none());
        assert!(parse_descriptor_value("size-adjust", "-1%").is_none());
        assert!(parse_descriptor_value("font-display", "var(--x)").is_none());
    }

    #[test]
    fn preserves_custom_properties_and_font_variant_observability() {
        let custom = parse_descriptor_value("--x", "var(--y").expect("custom descriptor");
        assert_eq!(custom.observable_value, "var(--y");
        let variant = parse_descriptor_value("font-variant", "small-caps").expect("variant");
        assert_eq!(variant.observable_value, "");
        assert_eq!(variant.safe_value, "small-caps");
    }
}
