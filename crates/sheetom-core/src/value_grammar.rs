use crate::{inspect_property, syntax::split_top_level_whitespace, PropertyParseKind};
use cssparser::{Parser, ParserInput};
use lightningcss::{
    stylesheet::PrinterOptions,
    traits::{Parse, ToCss},
    values::number::CSSNumber,
};

pub(crate) struct GrammarValue {
    pub(crate) observable_value: String,
    pub(crate) safe_value: String,
}

pub(crate) fn parse_browser_grammar_gap(name: &str, value: &str) -> Option<GrammarValue> {
    let input = value.trim();
    if name == "size" {
        return parse_page_size(input);
    }
    if name == "content" {
        return parse_content(input);
    }
    if matches!(name, "grid-template-columns" | "grid-template-rows") {
        return parse_subgrid(input);
    }
    if is_size_property(name) && input.to_ascii_lowercase().starts_with("anchor-size(") {
        return parse_anchor_size(input);
    }
    if (name == "color" || name.ends_with("-color"))
        && input.to_ascii_lowercase().starts_with("contrast-color(")
    {
        return parse_contrast_color(input);
    }
    if name == "z-index" && input.contains('(') {
        return parse_integer_calculation(input);
    }
    if name == "-webkit-box-reflect" {
        return parse_webkit_box_reflect(input);
    }
    if matches!(name, "offset-anchor" | "offset-position") {
        return parse_position(name, input);
    }
    if name == "offset-rotate" {
        return parse_offset_rotate(input);
    }
    None
}

fn parse_page_size(value: &str) -> Option<GrammarValue> {
    const NAMED_SIZES: &[&str] = &[
        "a5", "a4", "a3", "b5", "b4", "jis-b5", "jis-b4", "ledger", "legal", "letter",
    ];
    const ORIENTATIONS: &[&str] = &["portrait", "landscape"];

    let components = split_top_level_whitespace(value)?;
    match components.as_slice() {
        [single] => {
            let lower = single.to_ascii_lowercase();
            if lower == "auto"
                || NAMED_SIZES.contains(&lower.as_str())
                || ORIENTATIONS.contains(&lower.as_str())
            {
                return Some(raw(&lower));
            }
            let length = parse_page_length(single)?;
            Some(raw(&length))
        }
        [first, second] => {
            let first_lower = first.to_ascii_lowercase();
            let second_lower = second.to_ascii_lowercase();
            let named = if NAMED_SIZES.contains(&first_lower.as_str())
                && ORIENTATIONS.contains(&second_lower.as_str())
            {
                Some((first_lower, second_lower))
            } else if ORIENTATIONS.contains(&first_lower.as_str())
                && NAMED_SIZES.contains(&second_lower.as_str())
            {
                Some((second_lower, first_lower))
            } else {
                None
            };
            if let Some((size, orientation)) = named {
                return Some(raw(&format!("{size} {orientation}")));
            }
            let width = parse_page_length(first)?;
            let height = parse_page_length(second)?;
            Some(raw(&format!("{width} {height}")))
        }
        _ => None,
    }
}

fn parse_page_length(value: &str) -> Option<String> {
    let inspection = inspect_property("width", value).ok()?;
    if inspection.kind != PropertyParseKind::Typed {
        return None;
    }
    let canonical = inspection.canonical_value;
    if canonical == "0" {
        return Some("0px".to_owned());
    }
    (!matches!(
        canonical.as_str(),
        "auto" | "min-content" | "max-content" | "fit-content"
    ) && !canonical.contains('%')
        && !canonical.starts_with("anchor-size(")
        && canonical != "stretch")
        .then_some(canonical)
}

fn parse_content(value: &str) -> Option<GrammarValue> {
    let lower = value.to_ascii_lowercase();
    if lower.contains("leader(")
        || lower.contains("target-text(url(")
        || lower.contains("target-counter(url(")
        || lower.contains("target-counters(url(")
    {
        return None;
    }
    if !value.starts_with(['\'', '"']) {
        return None;
    }
    let quote = value.as_bytes()[0] as char;
    let closed = value.ends_with(quote);
    let safe = if closed {
        value.to_owned()
    } else {
        format!("{value}{quote}")
    };
    Some(GrammarValue {
        observable_value: safe.clone(),
        safe_value: safe,
    })
}

fn parse_subgrid(value: &str) -> Option<GrammarValue> {
    let rest = value.strip_prefix("subgrid")?;
    if !rest.trim().is_empty() && !balanced_line_names(rest.trim()) {
        return None;
    }
    Some(raw(value))
}

fn balanced_line_names(value: &str) -> bool {
    let mut depth = 0usize;
    for character in value.chars() {
        match character {
            '[' => depth += 1,
            ']' if depth > 0 => depth -= 1,
            ']' => return false,
            _ if depth == 0 && !character.is_whitespace() => return false,
            _ => {}
        }
    }
    depth == 0
}

fn is_size_property(name: &str) -> bool {
    matches!(
        name,
        "width"
            | "height"
            | "min-width"
            | "min-height"
            | "max-width"
            | "max-height"
            | "inline-size"
            | "block-size"
            | "min-inline-size"
            | "min-block-size"
            | "max-inline-size"
            | "max-block-size"
    )
}

fn parse_anchor_size(value: &str) -> Option<GrammarValue> {
    let arguments = function_body(value, "anchor-size")?;
    if arguments.is_empty()
        || arguments.contains([';', '!'])
        || arguments.starts_with(',')
        || arguments.ends_with(',')
    {
        return None;
    }
    Some(raw(value))
}

fn parse_webkit_box_reflect(value: &str) -> Option<GrammarValue> {
    matches!(value, "above" | "below" | "left" | "right").then(|| GrammarValue {
        observable_value: format!("{value} 0px"),
        safe_value: format!("{value} 0px"),
    })
}

fn parse_position(name: &str, value: &str) -> Option<GrammarValue> {
    if (name == "offset-anchor" && value == "auto")
        || (name == "offset-position" && value == "normal")
    {
        return Some(raw(value));
    }
    let components = split_top_level_whitespace(value)?;
    if components.is_empty()
        || components.len() > 4
        || !components.iter().all(|value| {
            matches!(*value, "left" | "right" | "top" | "bottom" | "center")
                || is_length_percentage(value)
        })
    {
        return None;
    }
    let canonical = match components.as_slice() {
        ["center"] => "center center".to_owned(),
        [horizontal @ ("left" | "right")] => format!("{horizontal} center"),
        [vertical @ ("top" | "bottom")] => format!("center {vertical}"),
        [single] => format!("{single} center"),
        _ => components.join(" "),
    };
    Some(GrammarValue {
        observable_value: canonical.clone(),
        safe_value: canonical,
    })
}

fn parse_offset_rotate(value: &str) -> Option<GrammarValue> {
    let components = split_top_level_whitespace(value)?;
    if components.is_empty() || components.len() > 2 {
        return None;
    }
    let keywords = components
        .iter()
        .filter(|component| matches!(**component, "auto" | "reverse"))
        .count();
    let angles = components
        .iter()
        .filter(|component| is_angle(component))
        .count();
    (keywords <= 1 && angles <= 1 && keywords + angles == components.len()).then(|| raw(value))
}

fn is_length_percentage(value: &str) -> bool {
    if value == "0" {
        return true;
    }
    [
        "%", "px", "em", "rem", "vw", "vh", "vmin", "vmax", "cm", "mm", "in", "pt", "pc",
    ]
    .iter()
    .any(|unit| {
        value
            .strip_suffix(unit)
            .is_some_and(|number| number.parse::<f64>().is_ok())
    })
}

fn is_angle(value: &str) -> bool {
    value == "0"
        || ["deg", "grad", "rad", "turn"].iter().any(|unit| {
            value
                .strip_suffix(unit)
                .is_some_and(|number| number.parse::<f64>().is_ok())
        })
}

fn parse_contrast_color(value: &str) -> Option<GrammarValue> {
    let argument = function_body(value, "contrast-color")?;
    let inspection = inspect_property("color", argument).ok()?;
    if !matches!(
        inspection.kind,
        PropertyParseKind::Typed | PropertyParseKind::SheetomTyped
    ) {
        return None;
    }
    Some(raw(value))
}

fn parse_integer_calculation(value: &str) -> Option<GrammarValue> {
    let mut input = ParserInput::new(value);
    let mut parser = Parser::new(&mut input);
    let number = CSSNumber::parse(&mut parser).ok()?;
    parser.expect_exhausted().ok()?;
    let number = if number.is_nan() {
        "NaN".to_owned()
    } else if number == f32::INFINITY {
        "infinity".to_owned()
    } else if number == f32::NEG_INFINITY {
        "-infinity".to_owned()
    } else {
        number.to_css_string(PrinterOptions::default()).ok()?
    };
    let value = format!("calc({number})");
    Some(GrammarValue {
        observable_value: value.clone(),
        safe_value: value,
    })
}

fn function_body<'a>(value: &'a str, name: &str) -> Option<&'a str> {
    let prefix = value.get(..name.len())?;
    if !prefix.eq_ignore_ascii_case(name) {
        return None;
    }
    value[name.len()..]
        .strip_prefix('(')?
        .strip_suffix(')')
        .map(str::trim)
}

fn raw(value: &str) -> GrammarValue {
    GrammarValue {
        observable_value: value.to_owned(),
        safe_value: value.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::parse_browser_grammar_gap;

    #[test]
    fn accepts_reviewed_modern_grammar_families() {
        for (name, value) in [
            ("grid-template-columns", "subgrid"),
            ("width", "anchor-size(width)"),
            ("color", "contrast-color(red)"),
            ("z-index", "calc(1 + 1)"),
            ("content", "\"safe\""),
            ("-webkit-box-reflect", "below"),
        ] {
            assert!(
                parse_browser_grammar_gap(name, value).is_some(),
                "{name}: {value}"
            );
        }
    }

    #[test]
    fn canonicalizes_reparsable_z_index_math() {
        for (input, expected) in [
            ("calc(2)", "calc(2)"),
            ("calc(1 + 1 + 1)", "calc(3)"),
            ("calc(2 * (1 + 1))", "calc(4)"),
            ("min(1, 2)", "calc(1)"),
            ("round(1.5, 1)", "calc(2)"),
            ("hypot(3, 4)", "calc(5)"),
            ("calc(pi)", "calc(3.14159)"),
            ("calc(infinity)", "calc(infinity)"),
            ("calc(-infinity)", "calc(-infinity)"),
            ("calc(NaN)", "calc(NaN)"),
        ] {
            let parsed = parse_browser_grammar_gap("z-index", input);
            assert_eq!(
                parsed.map(|value| (value.observable_value, value.safe_value)),
                Some((expected.to_owned(), expected.to_owned())),
                "{input}"
            );
        }
        for input in ["calc(1px)", "calc()", "unknown(1)"] {
            assert!(
                parse_browser_grammar_gap("z-index", input).is_none(),
                "{input}"
            );
        }
    }

    #[test]
    fn rejects_reviewed_content_false_positives() {
        for value in ["leader(.)", "target-text(url(#x))"] {
            assert!(
                parse_browser_grammar_gap("content", value).is_none(),
                "{value}"
            );
        }
    }

    #[test]
    fn parses_page_size_branches() {
        assert_eq!(
            parse_browser_grammar_gap("size", "landscape A4").map(|value| value.observable_value),
            Some("a4 landscape".to_owned())
        );
        assert_eq!(
            parse_browser_grammar_gap("size", "10cm 20cm").map(|value| value.observable_value),
            Some("10cm 20cm".to_owned())
        );
        assert_eq!(
            parse_browser_grammar_gap("size", "0").map(|value| value.observable_value),
            Some("0px".to_owned())
        );
        assert!(parse_browser_grammar_gap("size", "50%").is_none());
        assert!(parse_browser_grammar_gap("size", "auto landscape").is_none());
    }
}
