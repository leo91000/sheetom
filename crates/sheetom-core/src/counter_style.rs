use crate::{
    shorthand::parse_value,
    syntax::{
        analyze_substitutions, parse_declaration_list, serialize_identifier,
        split_top_level_delimiter, split_top_level_whitespace,
    },
};
use cssparser::{Parser, ParserInput, Token};
use serde::Serialize;

pub const COUNTER_STYLE_DESCRIPTORS: &[&str] = &[
    "system",
    "symbols",
    "additive-symbols",
    "negative",
    "prefix",
    "suffix",
    "range",
    "pad",
    "speak-as",
    "fallback",
];

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ParsedCounterStyleDescriptor {
    pub name: String,
    pub value: String,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ParsedCounterStyleName {
    pub name: String,
    pub serialized: String,
}

pub fn parse_counter_style_name(value: &str) -> Option<ParsedCounterStyleName> {
    let mut input = ParserInput::new(value);
    let mut parser = Parser::new(&mut input);
    let name = match parser.next().ok()? {
        Token::Ident(name) => name.to_string(),
        _ => return None,
    };
    if !parser.is_exhausted() || !valid_custom_ident(&name) {
        return None;
    }
    Some(ParsedCounterStyleName {
        serialized: serialize_identifier(&name),
        name,
    })
}

pub fn parse_counter_style_descriptor(name: &str, value: &str) -> Option<String> {
    let name = name.to_ascii_lowercase();
    if !COUNTER_STYLE_DESCRIPTORS.contains(&name.as_str()) {
        return None;
    }
    let analysis = analyze_substitutions(value);
    if !analysis.valid || analysis.found || is_css_wide_keyword(value) {
        return None;
    }
    let canonical = parse_value("--sheetom-counter-style", value, false)
        .ok()?
        .safe_value()
        .to_owned();
    match name.as_str() {
        "system" => canonical_system(&canonical),
        "symbols" => canonical_symbols(&canonical),
        "additive-symbols" => canonical_additive_symbols(&canonical),
        "negative" => canonical_symbol_count(&canonical, 1, 2),
        "prefix" | "suffix" => canonical_symbol_count(&canonical, 1, 1),
        "range" => canonical_range(&canonical),
        "pad" => canonical_pad(&canonical),
        "speak-as" if valid_single_ident(&canonical) => Some(canonical),
        "fallback" if valid_single_ident(&canonical) => Some(canonical_fallback(canonical)),
        _ => None,
    }
}

pub fn parse_counter_style_descriptors(source: &str) -> Vec<ParsedCounterStyleDescriptor> {
    parse_declaration_list(source)
        .into_iter()
        .filter(|declaration| !declaration.important)
        .filter_map(|declaration| {
            let name = declaration.name.to_ascii_lowercase();
            let value = parse_counter_style_descriptor(&name, &declaration.value)?;
            Some(ParsedCounterStyleDescriptor { name, value })
        })
        .collect()
}

fn canonical_system(value: &str) -> Option<String> {
    let components = split_top_level_whitespace(value)?;
    match components.as_slice() {
        [keyword] if keyword.eq_ignore_ascii_case("fixed") => Some("fixed 1".to_owned()),
        [keyword]
            if matches!(
                keyword.to_ascii_lowercase().as_str(),
                "cyclic" | "numeric" | "alphabetic" | "symbolic" | "additive"
            ) =>
        {
            Some(keyword.to_ascii_lowercase())
        }
        [keyword, integer] if keyword.eq_ignore_ascii_case("fixed") => {
            Some(format!("fixed {}", integer.parse::<i32>().ok()?))
        }
        [keyword, ident]
            if keyword.eq_ignore_ascii_case("extends") && valid_custom_ident(ident) =>
        {
            Some(format!("extends {ident}"))
        }
        _ => None,
    }
}

fn canonical_symbols(value: &str) -> Option<String> {
    let components = split_top_level_whitespace(value)?;
    (!components.is_empty() && components.iter().all(|component| valid_symbol(component)))
        .then(|| components.join(" "))
}

fn canonical_additive_symbols(value: &str) -> Option<String> {
    let entries = split_top_level_delimiter(value, b',')?;
    let mut previous = None;
    let mut canonical = Vec::with_capacity(entries.len());
    for entry in entries {
        let components = split_top_level_whitespace(entry)?;
        let (weight, symbol) = parse_weight_and_symbol(&components)?;
        if previous.is_some_and(|previous| weight >= previous) || !valid_symbol(symbol) {
            return None;
        }
        previous = Some(weight);
        canonical.push(format!("{weight} {symbol}"));
    }
    Some(canonical.join(", "))
}

fn canonical_symbol_count(value: &str, minimum: usize, maximum: usize) -> Option<String> {
    let components = split_top_level_whitespace(value)?;
    ((minimum..=maximum).contains(&components.len())
        && components.iter().all(|component| valid_symbol(component)))
    .then(|| components.join(" "))
}

fn canonical_range(value: &str) -> Option<String> {
    if value.eq_ignore_ascii_case("auto") {
        return Some("auto".to_owned());
    }
    let entries = split_top_level_delimiter(value, b',')?;
    let mut canonical = Vec::with_capacity(entries.len());
    for entry in entries {
        let components = split_top_level_whitespace(entry)?;
        let [start, end] = components.as_slice() else {
            return None;
        };
        let start = parse_range_bound(start, true);
        let end = parse_range_bound(end, false);
        let valid = match (&start, &end) {
            (Some(RangeBound::NegativeInfinity), Some(_)) => true,
            (Some(_), Some(RangeBound::PositiveInfinity)) => true,
            (Some(RangeBound::Integer(start)), Some(RangeBound::Integer(end))) => start <= end,
            _ => false,
        };
        if !valid {
            return None;
        }
        canonical.push(format!(
            "{} {}",
            canonical_range_bound(start?),
            canonical_range_bound(end?)
        ));
    }
    Some(canonical.join(", "))
}

enum RangeBound {
    NegativeInfinity,
    PositiveInfinity,
    Integer(i32),
}

fn canonical_range_bound(bound: RangeBound) -> String {
    match bound {
        RangeBound::NegativeInfinity | RangeBound::PositiveInfinity => "infinite".to_owned(),
        RangeBound::Integer(value) => value.to_string(),
    }
}

fn parse_range_bound(value: &str, start: bool) -> Option<RangeBound> {
    if value.eq_ignore_ascii_case("infinite") {
        return Some(if start {
            RangeBound::NegativeInfinity
        } else {
            RangeBound::PositiveInfinity
        });
    }
    value.parse::<i32>().ok().map(RangeBound::Integer)
}

fn canonical_pad(value: &str) -> Option<String> {
    let components = split_top_level_whitespace(value)?;
    let (width, symbol) = parse_weight_and_symbol(&components)?;
    valid_symbol(symbol).then(|| format!("{width} {symbol}"))
}

fn parse_weight_and_symbol<'a>(components: &'a [&'a str]) -> Option<(u32, &'a str)> {
    let [first, second] = components else {
        return None;
    };
    if let Ok(weight) = first.parse::<u32>() {
        return Some((weight, second));
    }
    Some((second.parse::<u32>().ok()?, first))
}

fn canonical_fallback(value: String) -> String {
    if value.eq_ignore_ascii_case("decimal") {
        return "decimal".to_owned();
    }
    value
}

fn valid_single_ident(value: &str) -> bool {
    split_top_level_whitespace(value)
        .is_some_and(|components| components.len() == 1 && valid_custom_ident(components[0]))
}

fn valid_symbol(value: &str) -> bool {
    let mut input = ParserInput::new(value);
    let mut parser = Parser::new(&mut input);
    let valid = matches!(
        parser.next().ok(),
        Some(Token::Ident(_)) | Some(Token::QuotedString(_))
    );
    valid && parser.is_exhausted()
}

fn valid_custom_ident(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    if is_css_wide_keyword(&lower) || matches!(lower.as_str(), "default" | "none") {
        return false;
    }
    let mut input = ParserInput::new(value);
    let mut parser = Parser::new(&mut input);
    matches!(parser.next().ok(), Some(Token::Ident(_))) && parser.is_exhausted()
}

fn is_css_wide_keyword(value: &str) -> bool {
    matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "initial" | "inherit" | "unset" | "revert" | "revert-layer" | "revert-rule"
    )
}

#[cfg(test)]
mod tests {
    use super::{
        parse_counter_style_descriptor, parse_counter_style_descriptors, parse_counter_style_name,
    };

    #[test]
    fn accepts_every_counter_style_descriptor_family() {
        for (name, value, expected) in [
            ("system", "fixed", "fixed 1"),
            ("system", "fixed 3", "fixed 3"),
            ("system", "extends decimal", "extends decimal"),
            ("symbols", "\"a\" foo", "\"a\" foo"),
            (
                "additive-symbols",
                "100 \"c\", 10 \"x\"",
                "100 \"c\", 10 \"x\"",
            ),
            ("negative", "\"(\" \")\"", "\"(\" \")\""),
            ("prefix", "\"(\"", "\"(\""),
            ("suffix", "foo", "foo"),
            ("range", "1 10, 20 infinite", "1 10, 20 infinite"),
            ("pad", "2 \"0\"", "2 \"0\""),
            ("speak-as", "spell-out", "spell-out"),
            ("fallback", "decimal", "decimal"),
        ] {
            assert_eq!(
                parse_counter_style_descriptor(name, value).as_deref(),
                Some(expected),
                "{name}: {value}"
            );
        }
        assert_eq!(
            parse_counter_style_descriptor("system", "FIXED +003").as_deref(),
            Some("fixed 3")
        );
        assert_eq!(
            parse_counter_style_descriptor("range", "infinite -02, +004 INFINITE").as_deref(),
            Some("infinite -2, 4 infinite")
        );
        assert_eq!(
            parse_counter_style_descriptor("pad", "\"0\" 2").as_deref(),
            Some("2 \"0\"")
        );
        assert_eq!(
            parse_counter_style_descriptor("additive-symbols", "\"x\" 10, \"i\" 1").as_deref(),
            Some("10 \"x\", 1 \"i\"")
        );
        assert_eq!(
            parse_counter_style_descriptor("symbols", "foo/**/bar").as_deref(),
            Some("foo bar")
        );
    }

    #[test]
    fn rejects_invalid_neighbors_and_priorities() {
        for (name, value) in [
            ("symbols", "var(--x)"),
            ("additive-symbols", "1 \"a\" 2 \"b\""),
            ("negative", "a b c"),
            ("range", "10 1"),
            ("pad", "-1 \"0\""),
            ("fallback", "foo bar"),
            ("fallback", "none"),
            ("speak-as", "default"),
            ("system", "extends none"),
        ] {
            assert!(parse_counter_style_descriptor(name, value).is_none());
        }
        assert!(parse_counter_style_descriptors("symbols: \"x\" !important").is_empty());
    }

    #[test]
    fn parses_and_serializes_counter_style_names() {
        let escaped = parse_counter_style_name("\\78").expect("escaped identifier");
        assert_eq!(escaped.name, "x");
        assert_eq!(escaped.serialized, "x");
        assert_eq!(
            parse_counter_style_name("--icons").map(|value| value.name),
            Some("--icons".to_owned())
        );
        for invalid in ["", "123", "bad name", "none", "default", "inherit"] {
            assert!(parse_counter_style_name(invalid).is_none(), "{invalid}");
        }
    }
}
