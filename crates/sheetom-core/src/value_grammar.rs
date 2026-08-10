use crate::{inspect_property, syntax::split_top_level_whitespace, PropertyParseKind};

pub(crate) struct GrammarValue {
    pub(crate) observable_value: String,
    pub(crate) safe_value: String,
}

pub(crate) fn parse_browser_grammar_gap(name: &str, value: &str) -> Option<GrammarValue> {
    let input = value.trim();
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
    if name == "z-index" && input.to_ascii_lowercase().starts_with("calc(") {
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
    let body = function_body(value, "calc")?;
    for operator in ['+', '-'] {
        let Some((left, right)) = body.split_once(operator) else {
            continue;
        };
        let left = left.trim().parse::<f64>().ok()?;
        let right = right.trim().parse::<f64>().ok()?;
        let result = if operator == '+' {
            left + right
        } else {
            left - right
        };
        if result.fract() == 0.0 {
            let value = format!("calc({result:.0})");
            return Some(GrammarValue {
                observable_value: value.clone(),
                safe_value: value,
            });
        }
    }
    None
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
    fn rejects_reviewed_content_false_positives() {
        for value in ["leader(.)", "target-text(url(#x))"] {
            assert!(
                parse_browser_grammar_gap("content", value).is_none(),
                "{value}"
            );
        }
    }
}
