use crate::catalog::shorthand_longhands;
use crate::recovered_value::RecoveredObservableText;
use crate::{EngineError, SemanticDeclaration, SemanticExtensionValue, SemanticPropertyValue};
use cssparser::{Token, TokenizerWithSpans};
use lightningcss::{
    properties::{Property, PropertyId},
    stylesheet::{ParserOptions, PrinterOptions},
};

#[derive(Clone, Copy)]
enum ObservableCategory {
    Typed,
    PendingSubstitution,
    Custom,
}

pub(crate) struct DeclarationProjection {
    pub(crate) canonical: String,
    pub(crate) observable: String,
}

pub(crate) fn project_declaration(
    declaration: &SemanticDeclaration,
) -> Result<DeclarationProjection, EngineError> {
    let name = declaration.property_name();
    let input = trim_css_whitespace(declaration.recovered().source());
    let canonical = declaration.canonical_value()?;
    let category = match declaration.value() {
        SemanticPropertyValue::Standard(_)
        | SemanticPropertyValue::Extension(_)
        | SemanticPropertyValue::ExpandedShorthand
        | SemanticPropertyValue::FontFaceDescriptor(_) => ObservableCategory::Typed,
        SemanticPropertyValue::PendingSubstitution(_) => ObservableCategory::PendingSubstitution,
        SemanticPropertyValue::CustomTokenStream => ObservableCategory::Custom,
    };
    let preserve_comments = matches!(
        category,
        ObservableCategory::PendingSubstitution | ObservableCategory::Custom
    );
    let recovered = declaration.recovered().observable_text(preserve_comments)?;
    let retained = if preserve_comments {
        trim_token_stream_trivia(&recovered.retained)
    } else {
        trim_css_whitespace(&recovered.retained)
    };
    let closed = trim_css_whitespace(&recovered.closed);
    let observable = if matches!(
        declaration.value(),
        SemanticPropertyValue::FontFaceDescriptor(_)
    ) && name == "font-variant"
    {
        String::new()
    } else if !matches!(category, ObservableCategory::Typed) {
        if matches!(category, ObservableCategory::Custom) && recovered.unterminated_url {
            closed.to_owned()
        } else {
            retained.to_owned()
        }
    } else if let SemanticPropertyValue::Extension(SemanticExtensionValue::Geometric(value)) =
        declaration.value()
    {
        if let Some(gradient) = value.gradient_observable_value()? {
            serialize_gradient_observable(&gradient)
        } else if let Some(observable) = value.image_set_observable_value()? {
            observable
        } else {
            canonical.clone()
        }
    } else {
        serialize_typed_observable(name, input, closed, &canonical, &recovered)
    };

    Ok(DeclarationProjection {
        canonical,
        observable,
    })
}

fn serialize_gradient_observable(input: &str) -> String {
    let mut value = replace_gradient_color_tokens(input);
    value = replace_comments_with_space(&value);
    value = normalize_comma_whitespace(&value);
    value = canonicalize_leading_decimal(&value);
    value = canonicalize_color_identifiers(&value);
    if value
        .get(..value.find('(').unwrap_or(value.len()))
        .is_some_and(|name| name.eq_ignore_ascii_case("-webkit-gradient"))
    {
        value = normalize_webkit_gradient_points(&value);
    }
    if let Some(open) = value.find('(') {
        value[..open].make_ascii_lowercase();
    }
    value
}

fn normalize_webkit_gradient_points(input: &str) -> String {
    let mut tokenizer = TokenizerWithSpans::new(input);
    let mut replacements = Vec::new();
    while let Ok(token) = tokenizer.next_token() {
        let Token::Ident(identifier) = token.token else {
            continue;
        };
        let replacement =
            if identifier.eq_ignore_ascii_case("left") || identifier.eq_ignore_ascii_case("top") {
                "0%"
            } else if identifier.eq_ignore_ascii_case("right")
                || identifier.eq_ignore_ascii_case("bottom")
            {
                "100%"
            } else if identifier.eq_ignore_ascii_case("center") {
                "50%"
            } else {
                continue;
            };
        replacements.push((
            token.start.byte_index(),
            token.end.byte_index(),
            replacement,
        ));
    }

    let mut output = input.to_owned();
    for (start, end, replacement) in replacements.into_iter().rev() {
        output.replace_range(start..end, replacement);
    }
    output
}

fn replace_gradient_color_tokens(input: &str) -> String {
    #[derive(Clone, Copy)]
    enum Opening {
        Parenthesis { color_start: Option<usize> },
        Square,
        Curly,
    }

    let mut tokenizer = TokenizerWithSpans::new(input);
    let mut openings = Vec::new();
    let mut replacements = Vec::<(usize, usize, String)>::new();
    while let Ok(token) = tokenizer.next_token() {
        match token.token {
            Token::Function(name) => openings.push(Opening::Parenthesis {
                color_start: is_serializable_color_function(&name)
                    .then_some(token.start.byte_index()),
            }),
            Token::ParenthesisBlock => openings.push(Opening::Parenthesis { color_start: None }),
            Token::SquareBracketBlock => openings.push(Opening::Square),
            Token::CurlyBracketBlock => openings.push(Opening::Curly),
            Token::CloseParenthesis => {
                let Some(Opening::Parenthesis { color_start }) = openings.pop() else {
                    continue;
                };
                let Some(start) = color_start else {
                    continue;
                };
                let end = token.end.byte_index();
                let Some(source) = input.get(start..end) else {
                    continue;
                };
                if let Some(color) = canonicalize_nested_color(source) {
                    replacements.push((start, end, color));
                }
            }
            Token::CloseSquareBracket => {
                if matches!(openings.last(), Some(Opening::Square)) {
                    openings.pop();
                }
            }
            Token::CloseCurlyBracket => {
                if matches!(openings.last(), Some(Opening::Curly)) {
                    openings.pop();
                }
            }
            Token::Hash(value) | Token::IDHash(value) => {
                let source = format!("#{value}");
                if let Some(color) = serialize_hex_color(&source) {
                    replacements.push((token.start.byte_index(), token.end.byte_index(), color));
                }
            }
            _ => {}
        }
    }

    replacements.sort_unstable_by_key(|(start, _, _)| *start);
    let mut outermost = Vec::with_capacity(replacements.len());
    let mut covered_until = 0usize;
    for replacement in replacements {
        if replacement.0 < covered_until {
            continue;
        }
        covered_until = replacement.1;
        outermost.push(replacement);
    }
    let mut output = input.to_owned();
    for (start, end, replacement) in outermost.into_iter().rev() {
        output.replace_range(start..end, &replacement);
    }
    output
}

fn is_serializable_color_function(name: &str) -> bool {
    [
        "rgb", "rgba", "hsl", "hsla", "hwb", "lab", "lch", "oklab", "oklch", "color",
    ]
    .iter()
    .any(|candidate| name.eq_ignore_ascii_case(candidate))
}

fn canonicalize_nested_color(source: &str) -> Option<String> {
    let property =
        Property::parse_string(PropertyId::Color, source, ParserOptions::default()).ok()?;
    let safe = property
        .value_to_css_string(PrinterOptions::default())
        .ok()?;
    Some(serialize_color(source, &safe))
}

fn replace_comments_with_space(input: &str) -> String {
    let mut tokenizer = TokenizerWithSpans::new(input);
    let mut comments = Vec::new();
    while let Ok(token) = tokenizer.next_token() {
        if matches!(token.token, Token::Comment(_)) {
            comments.push((token.start.byte_index(), token.end.byte_index()));
        }
    }
    let mut output = input.to_owned();
    for (start, end) in comments.into_iter().rev() {
        output.replace_range(start..end, " ");
    }
    output
}

fn normalize_comma_whitespace(source: &str) -> String {
    let mut output = String::with_capacity(source.len() + 4);
    let mut characters = source.chars().peekable();
    let mut quote = None;
    let mut escaped = false;
    while let Some(character) = characters.next() {
        if let Some(delimiter) = quote {
            output.push(character);
            if escaped {
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == delimiter {
                quote = None;
            }
            continue;
        }
        if matches!(character, '\'' | '"') {
            quote = Some(character);
            output.push(character);
            continue;
        }
        if character != ',' {
            output.push(character);
            continue;
        }
        while output.chars().last().is_some_and(char::is_whitespace) {
            output.pop();
        }
        output.push_str(", ");
        while characters.next_if(|value| value.is_whitespace()).is_some() {}
    }
    output
}

fn serialize_typed_observable(
    name: &str,
    input: &str,
    closed: &str,
    canonical: &str,
    recovered: &RecoveredObservableText,
) -> String {
    if name == "font-family" {
        return serialize_font_family(
            input,
            canonical,
            recovered.single_string.as_deref(),
            recovered.recovered,
        );
    }
    if shorthand_longhands(name).is_some_and(|longhands| longhands.len() > 1)
        && !recovered.recovered
    {
        return input.to_owned();
    }
    if name == "color" || name.ends_with("-color") {
        return serialize_color(closed, canonical);
    }
    if name == "z-index" {
        if let Some(value) = serialize_integer_calculation(closed) {
            return value;
        }
    }
    if name == "object-position" {
        if starts_math_function(input) && !starts_math_function(canonical) {
            if let Some((first, rest)) = canonical.split_once(' ') {
                return format!("calc({first}) {rest}");
            }
        }
        return canonicalize_leading_decimal(canonical);
    }
    if starts_math_function(closed) {
        let value = canonicalize_leading_decimal(canonical);
        if starts_math_function(&value) {
            return value;
        }
        return format!("calc({value})");
    }
    if closed.contains("gradient(") {
        return closed.to_owned();
    }
    serialize_default_observable(input, closed, canonical, recovered)
}

fn serialize_default_observable(
    input: &str,
    closed: &str,
    canonical: &str,
    recovered: &RecoveredObservableText,
) -> String {
    if !recovered.recovered {
        return canonicalize_leading_decimal(canonical);
    }
    if closed.to_ascii_lowercase().starts_with("url(") || input.starts_with(['\'', '"']) {
        return canonical.to_owned();
    }
    if input.contains("/*") {
        return trim_css_whitespace(&recovered.retained).to_owned();
    }
    closed.to_owned()
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

fn serialize_font_family(
    input: &str,
    safe_value: &str,
    single_string: Option<&str>,
    recovered: bool,
) -> String {
    let Some(value) = single_string else {
        return safe_value.to_owned();
    };
    if is_identifier(value) && !is_generic_font_family(value) {
        return value.to_owned();
    }
    if !recovered && input.ends_with(['\'', '"']) && safe_value.starts_with(['\'', '"']) {
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
    if is_relative_color_function(value) {
        return safe_value.to_owned();
    }
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
    canonicalize_color_identifiers(value)
}

pub(crate) fn serialize_observable_color(value: &str) -> String {
    serialize_color(value, value)
}

fn is_relative_color_function(value: &str) -> bool {
    let mut tokenizer = TokenizerWithSpans::new(value);
    let Some(Token::Function(function)) = next_significant_token(&mut tokenizer) else {
        return false;
    };
    if ![
        "rgb", "rgba", "hsl", "hsla", "hwb", "lab", "lch", "oklab", "oklch", "color",
    ]
    .iter()
    .any(|candidate| function.eq_ignore_ascii_case(candidate))
    {
        return false;
    }
    next_significant_token(&mut tokenizer).is_some_and(
        |token| matches!(token, Token::Ident(ident) if ident.eq_ignore_ascii_case("from")),
    )
}

fn next_significant_token<'i>(tokenizer: &mut TokenizerWithSpans<'i>) -> Option<Token<'i>> {
    loop {
        let token = tokenizer.next_token().ok()?.token;
        if !matches!(token, Token::WhiteSpace(_) | Token::Comment(_)) {
            return Some(token);
        }
    }
}

fn canonicalize_color_identifiers(value: &str) -> String {
    let mut tokenizer = TokenizerWithSpans::new(value);
    let mut output = String::with_capacity(value.len());
    let mut cursor = 0usize;
    while let Ok(token) = tokenizer.next_token() {
        let Token::Ident(identifier) = token.token else {
            continue;
        };
        if !identifier.eq_ignore_ascii_case("currentcolor") {
            continue;
        }
        let start = token.start.byte_index();
        let end = token.end.byte_index();
        let Some(prefix) = value.get(cursor..start) else {
            return value.to_owned();
        };
        output.push_str(prefix);
        output.push_str("currentcolor");
        cursor = end;
    }
    let Some(suffix) = value.get(cursor..) else {
        return value.to_owned();
    };
    output.push_str(suffix);
    output
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
    let mut tokenizer = TokenizerWithSpans::new(value);
    let Some(Token::Function(function)) = next_significant_token(&mut tokenizer) else {
        return false;
    };
    [
        "calc", "min", "max", "clamp", "round", "rem", "mod", "abs", "sign", "hypot", "sin", "cos",
        "tan", "asin", "acos", "atan", "atan2", "pow", "sqrt", "log", "exp",
    ]
    .iter()
    .any(|candidate| function.eq_ignore_ascii_case(candidate))
}

#[cfg(test)]
mod tests {
    use super::project_declaration;
    use crate::parse_semantic_property;

    fn observable(name: &str, input: &str) -> String {
        let declaration = parse_semantic_property(name, input).unwrap();
        project_declaration(&declaration).unwrap().observable
    }

    #[test]
    fn recovers_browser_facing_token_text() {
        assert_eq!(observable("--x", "red/*comment"), "red");
        assert_eq!(observable("--x", "foo\\"), "foo�");
        assert_eq!(observable("width", "calc(1px"), "calc(1px)");
    }

    #[test]
    fn serializes_typed_math_like_chromium_cssom() {
        for (name, input, expected) in [
            ("width", "calc(1px / 2)", "calc(0.5px)"),
            ("width", "min(1px, 2%)", "min(1px, 2%)"),
            ("width", "round(1px, 2px)", "calc(2px)"),
            ("width", "hypot(3px, 4px)", "calc(5px)"),
            ("rotate", "atan2(1, 1)", "calc(45deg)"),
            ("opacity", "pow(2, 3)", "calc(8)"),
        ] {
            assert_eq!(observable(name, input), expected, "{name}: {input}");
        }
    }

    #[test]
    fn preserves_internal_comments_for_custom_and_pending_token_streams() {
        for (name, input, expected) in [
            ("--x", "a/*c*/b", "a/*c*/b"),
            ("--x", "\u{00a0}red\u{00a0}", "\u{00a0}red\u{00a0}"),
            ("--x", "/*c*/a/*tail*/", "a"),
            (
                "width",
                "calc(var(--x)/*c*/ + 1px)",
                "calc(var(--x)/*c*/ + 1px)",
            ),
            ("width", "--f(a/*c*/,b)", "--f(a/*c*/,b)"),
            ("width", "--f(a)/*c*/", "--f(a)"),
            ("width", "--f(a/*c)", "--f(a"),
        ] {
            assert_eq!(observable(name, input), expected, "{input}");
        }
    }

    #[test]
    fn serializes_cssom_colors() {
        assert_eq!(
            observable("color", "rgb(1 2 3 / 50%)"),
            "rgba(1, 2, 3, 0.5)"
        );
        assert_eq!(observable("color", "white"), "white");
        assert_eq!(
            observable(
                "color",
                "color-mix(in srgb, contrast-color(red), currentColor)",
            ),
            "color-mix(in srgb, contrast-color(red), currentcolor)"
        );
        assert_eq!(
            observable("color", "contrast-color(current\\43 olor)"),
            "contrast-color(currentcolor)"
        );
        assert_eq!(
            observable(
                "color",
                "RGBA(from rgb(20%, 40%, 60%, 80%) r calc(g * .5 + g * .5) b / alpha)",
            ),
            "rgb(from rgba(51, 102, 153, 0.8) r calc((0.5 * g) + (0.5 * g)) b / alpha)"
        );
        assert_eq!(
            observable(
                "color",
                "lab(from var(--mycolor) l a b / calc(alpha * 0.8))",
            ),
            "lab(from var(--mycolor) l a b / calc(alpha * 0.8))"
        );
    }

    #[test]
    fn serializes_gradient_images_without_erasing_authored_color_identity() {
        for (input, expected) in [
            ("linear-gradient(red,blue)", "linear-gradient(red, blue)"),
            (
                "linear-gradient(red 0%, rgb(0 0 255) 100%)",
                "linear-gradient(red 0%, rgb(0, 0, 255) 100%)",
            ),
            (
                "LINEAR-GRADIENT(#f00 0%, rgba(0,0,255,.5) 100%)",
                "linear-gradient(rgb(255, 0, 0) 0%, rgba(0, 0, 255, 0.5) 100%)",
            ),
            (
                "linear-gradient(hsl(120 100% 50%), transparent)",
                "linear-gradient(rgb(0, 255, 0), transparent)",
            ),
        ] {
            assert_eq!(observable("shape-outside", input), expected, "{input}");
        }
    }
}
