use crate::catalog::shorthand_longhands;

pub(crate) enum ObservableCategory {
    Typed,
    PendingSubstitution,
    Custom,
}

pub(crate) fn serialize_observable_value(
    name: &str,
    input: &str,
    safe_value: &str,
    category: ObservableCategory,
) -> String {
    let input = trim_css_whitespace(input);
    let preserve_comments = matches!(
        category,
        ObservableCategory::PendingSubstitution | ObservableCategory::Custom
    );
    let recovered = recover_token_text(input, preserve_comments);
    if !matches!(category, ObservableCategory::Typed) {
        if matches!(category, ObservableCategory::Custom)
            && input.to_ascii_lowercase().starts_with("url(")
            && input.ends_with('\\')
        {
            return recovered.closed;
        }
        return recovered.retained;
    }
    if name == "font-family" {
        return serialize_font_family(input, safe_value, &recovered);
    }
    if shorthand_longhands(name).is_some_and(|longhands| longhands.len() > 1)
        && !recovered.recovered
    {
        return input.to_owned();
    }
    if name == "color" || name.ends_with("-color") {
        return serialize_color(&recovered.closed, safe_value);
    }
    if name == "z-index" {
        if let Some(value) = serialize_integer_calculation(&recovered.closed) {
            return value;
        }
    }
    if starts_math_function(&recovered.closed) {
        return if safe_value.starts_with("calc(") {
            safe_value.to_owned()
        } else {
            format!("calc({safe_value})")
        };
    }
    if recovered.closed.contains("gradient(") {
        return recovered.closed;
    }
    if !recovered.recovered {
        return canonicalize_leading_decimal(safe_value);
    }
    if recovered.closed.to_ascii_lowercase().starts_with("url(") || input.starts_with(['\'', '"']) {
        return safe_value.to_owned();
    }
    if input.contains("/*") {
        return recovered.retained;
    }
    recovered.closed
}

fn canonicalize_leading_decimal(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut result = String::with_capacity(value.len() + 1);
    let mut index = 0usize;
    while index < bytes.len() {
        let signed = matches!(bytes[index], b'+' | b'-');
        let dot = if signed { index + 1 } else { index };
        if bytes.get(dot) == Some(&b'.')
            && bytes.get(dot + 1).is_some_and(u8::is_ascii_digit)
            && (index == 0
                || !bytes[index - 1].is_ascii_alphanumeric()
                    && !matches!(bytes[index - 1], b'_' | b'-'))
        {
            if signed {
                result.push(bytes[index] as char);
            }
            result.push_str("0.");
            index = dot + 1;
            continue;
        }
        let character = value[index..].chars().next().unwrap_or('\u{fffd}');
        result.push(character);
        index += character.len_utf8();
    }
    result
}

struct RecoveredTokenText {
    closed: String,
    recovered: bool,
    retained: String,
    single_string: Option<String>,
}

fn recover_token_text(input: &str, preserve_comments: bool) -> RecoveredTokenText {
    let bytes = input.as_bytes();
    let mut retained = String::with_capacity(input.len());
    let mut closings = Vec::new();
    let mut index = 0usize;
    let mut recovered = false;
    let mut significant = 0usize;
    let mut single_string = None;

    while index < bytes.len() {
        if bytes[index].is_ascii_whitespace() {
            retained.push(bytes[index] as char);
            index += 1;
            continue;
        }
        if bytes[index] == b'/' && bytes.get(index + 1) == Some(&b'*') {
            let start = index;
            recovered = true;
            index += 2;
            while index < bytes.len()
                && !(bytes[index] == b'*' && bytes.get(index + 1) == Some(&b'/'))
            {
                index += 1;
            }
            let closed = index < bytes.len();
            index = (index + 2).min(bytes.len());
            if preserve_comments && closed {
                retained.push_str(&input[start..index]);
            }
            continue;
        }
        if matches!(bytes[index], b'\'' | b'"') {
            significant += 1;
            let quote = bytes[index];
            let start = index;
            index += 1;
            let mut value = String::new();
            let mut closed = false;
            while index < bytes.len() {
                if bytes[index] == b'\\' {
                    if let Some(next) = bytes.get(index + 1) {
                        retained.push_str(&input[start..index]);
                        retained.push('\\');
                        retained.push(*next as char);
                        value.push(*next as char);
                        index += 2;
                        closed = bytes.get(index.wrapping_sub(1)) == Some(&quote);
                        break;
                    }
                    value.push('\u{fffd}');
                    index += 1;
                    recovered = true;
                    break;
                }
                if bytes[index] == quote {
                    index += 1;
                    closed = true;
                    break;
                }
                value.push(bytes[index] as char);
                index += 1;
            }
            retained.push_str(&input[start..index]);
            if !closed {
                recovered = true;
            }
            single_string = Some(value);
            continue;
        }
        significant += 1;
        if bytes[index] == b'\\' && index + 1 == bytes.len() {
            retained.push('\u{fffd}');
            recovered = true;
            index += 1;
            continue;
        }
        let character = input[index..].chars().next().unwrap_or('\u{fffd}');
        retained.push(character);
        if matches!(character, '(' | '[' | '{') {
            closings.push(match character {
                '(' => ')',
                '[' => ']',
                '{' => '}',
                _ => unreachable!(),
            });
        } else if matches!(character, ')' | ']' | '}') && closings.last() == Some(&character) {
            closings.pop();
        }
        index += character.len_utf8();
    }

    if !closings.is_empty() {
        recovered = true;
    }
    let retained = if preserve_comments {
        trim_token_stream_trivia(&retained)
    } else {
        trim_css_whitespace(&retained)
    };
    let mut closed = retained.to_owned();
    for closing in closings.iter().rev() {
        closed.push(*closing);
    }
    RecoveredTokenText {
        closed,
        recovered,
        retained: retained.to_owned(),
        single_string: (significant == 1).then_some(single_string).flatten(),
    }
}

fn trim_token_stream_trivia(mut value: &str) -> &str {
    loop {
        value = trim_css_whitespace(value);
        let Some(comment) = value.strip_prefix("/*") else {
            break;
        };
        let Some(end) = comment.find("*/") else {
            break;
        };
        value = &comment[end + 2..];
    }
    loop {
        value = trim_css_whitespace(value);
        let Some(comment_body) = value.strip_suffix("*/") else {
            break;
        };
        let Some(start) = comment_body.rfind("/*") else {
            break;
        };
        value = &comment_body[..start];
    }
    trim_css_whitespace(value)
}

fn trim_css_whitespace(value: &str) -> &str {
    value.trim_matches(|character| matches!(character, ' ' | '\t' | '\n' | '\r' | '\u{000c}'))
}

fn serialize_font_family(input: &str, safe_value: &str, recovered: &RecoveredTokenText) -> String {
    let Some(value) = &recovered.single_string else {
        return safe_value.to_owned();
    };
    if is_identifier(value) && !is_generic_font_family(value) {
        return value.clone();
    }
    if !recovered.recovered && input.ends_with(['\'', '"']) {
        return safe_value.to_owned();
    }
    format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
}

fn is_identifier(value: &str) -> bool {
    let mut characters = value.chars();
    let Some(first) = characters.next() else {
        return false;
    };
    (first == '-' || first == '_' || first.is_alphabetic())
        && characters
            .all(|character| character == '-' || character == '_' || character.is_alphanumeric())
}

fn is_generic_font_family(value: &str) -> bool {
    matches!(
        value.to_ascii_lowercase().as_str(),
        "serif"
            | "sans-serif"
            | "monospace"
            | "cursive"
            | "fantasy"
            | "system-ui"
            | "ui-serif"
            | "ui-sans-serif"
            | "ui-monospace"
            | "ui-rounded"
            | "math"
            | "fangsong"
            | "emoji"
    )
}

fn serialize_color(value: &str, safe_value: &str) -> String {
    if let Some(color) = serialize_rgb_color(value) {
        return color;
    }
    if let Some(color) = serialize_hex_color(value) {
        return color;
    }
    if value
        .chars()
        .all(|character| character.is_ascii_alphabetic() || character == '-')
    {
        return value.to_ascii_lowercase();
    }
    if [
        "hsl(", "hsla(", "hwb(", "lab(", "lch(", "oklab(", "oklch(", "color(",
    ]
    .iter()
    .any(|prefix| value.to_ascii_lowercase().starts_with(prefix))
    {
        return serialize_hex_color(safe_value).unwrap_or_else(|| value.to_owned());
    }
    value.to_owned()
}

fn serialize_hex_color(value: &str) -> Option<String> {
    let hex = value.strip_prefix('#')?;
    let expanded = match hex.len() {
        3 | 4 => hex
            .chars()
            .flat_map(|character| [character, character])
            .collect(),
        6 | 8 => hex.to_owned(),
        _ => return None,
    };
    let red = u8::from_str_radix(&expanded[0..2], 16).ok()?;
    let green = u8::from_str_radix(&expanded[2..4], 16).ok()?;
    let blue = u8::from_str_radix(&expanded[4..6], 16).ok()?;
    if expanded.len() == 6 {
        return Some(format!("rgb({red}, {green}, {blue})"));
    }
    let alpha = u8::from_str_radix(&expanded[6..8], 16).ok()?;
    Some(format!(
        "rgba({red}, {green}, {blue}, {})",
        format_number(f64::from(alpha) / 255.0)
    ))
}

fn serialize_rgb_color(value: &str) -> Option<String> {
    let open = value.find('(')?;
    let function = value[..open].trim().to_ascii_lowercase();
    if !matches!(function.as_str(), "rgb" | "rgba") || !value.ends_with(')') {
        return None;
    }
    let body = value[open + 1..value.len() - 1].trim();
    let (channels, slash_alpha) = body.split_once('/').map_or((body, None), |(left, right)| {
        (left.trim(), Some(right.trim()))
    });
    let mut parts = if channels.contains(',') {
        channels.split(',').map(str::trim).collect::<Vec<_>>()
    } else {
        channels.split_ascii_whitespace().collect::<Vec<_>>()
    };
    let alpha = if parts.len() == 4 {
        parts.pop()
    } else {
        slash_alpha
    };
    if parts.len() != 3 {
        return None;
    }
    let channels = parts
        .into_iter()
        .map(parse_color_channel)
        .collect::<Option<Vec<_>>>()?;
    if let Some(alpha) = alpha {
        return Some(format!(
            "rgba({}, {}, {}, {})",
            channels[0],
            channels[1],
            channels[2],
            format_number(parse_alpha(alpha)?)
        ));
    }
    Some(format!(
        "rgb({}, {}, {})",
        channels[0], channels[1], channels[2]
    ))
}

fn parse_color_channel(value: &str) -> Option<u8> {
    let number = value.trim_end_matches('%').parse::<f64>().ok()?;
    let normalized = if value.ends_with('%') {
        number.clamp(0.0, 100.0) * 255.0 / 100.0
    } else {
        number.clamp(0.0, 255.0)
    };
    Some(normalized.round() as u8)
}

fn parse_alpha(value: &str) -> Option<f64> {
    let number = value.trim_end_matches('%').parse::<f64>().ok()?;
    Some(if value.ends_with('%') {
        (number / 100.0).clamp(0.0, 1.0)
    } else {
        number.clamp(0.0, 1.0)
    })
}

fn format_number(value: f64) -> String {
    let rounded = (value * 1000.0).round() / 1000.0;
    if rounded.fract() == 0.0 {
        format!("{rounded:.0}")
    } else {
        rounded.to_string()
    }
}

fn serialize_integer_calculation(value: &str) -> Option<String> {
    let body = value.strip_prefix("calc(")?.strip_suffix(')')?.trim();
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
            return Some(format!("calc({result:.0})"));
        }
    }
    None
}

fn starts_math_function(value: &str) -> bool {
    ["calc(", "min(", "max(", "clamp("]
        .iter()
        .any(|prefix| value.to_ascii_lowercase().starts_with(prefix))
}

#[cfg(test)]
mod tests {
    use super::{serialize_observable_value, ObservableCategory};

    #[test]
    fn recovers_browser_facing_token_text() {
        assert_eq!(
            serialize_observable_value("--x", "red/*comment", "red", ObservableCategory::Custom),
            "red"
        );
        assert_eq!(
            serialize_observable_value("--x", "foo\\", "foo�", ObservableCategory::Custom),
            "foo�"
        );
        assert_eq!(
            serialize_observable_value("width", "calc(1px", "1px", ObservableCategory::Typed),
            "calc(1px)"
        );
    }

    #[test]
    fn preserves_internal_comments_for_custom_and_pending_token_streams() {
        for (category, input, expected) in [
            (ObservableCategory::Custom, "a/*c*/b", "a/*c*/b"),
            (
                ObservableCategory::Custom,
                "\u{00a0}red\u{00a0}",
                "\u{00a0}red\u{00a0}",
            ),
            (ObservableCategory::Custom, "/*c*/a/*tail*/", "a"),
            (
                ObservableCategory::PendingSubstitution,
                "calc(var(--x)/*c*/ + 1px)",
                "calc(var(--x)/*c*/ + 1px)",
            ),
            (
                ObservableCategory::PendingSubstitution,
                "--f(a/*c*/,b)",
                "--f(a/*c*/,b)",
            ),
            (
                ObservableCategory::PendingSubstitution,
                "--f(a)/*c*/",
                "--f(a)",
            ),
            (
                ObservableCategory::PendingSubstitution,
                "--f(a/*c)",
                "--f(a",
            ),
        ] {
            assert_eq!(
                serialize_observable_value("width", input, input, category),
                expected,
                "{input}"
            );
        }
    }

    #[test]
    fn serializes_cssom_colors() {
        assert_eq!(
            serialize_observable_value(
                "color",
                "rgb(1 2 3 / 50%)",
                "#01020380",
                ObservableCategory::Typed,
            ),
            "rgba(1, 2, 3, 0.5)"
        );
        assert_eq!(
            serialize_observable_value("color", "white", "#fff", ObservableCategory::Typed),
            "white"
        );
    }
}
