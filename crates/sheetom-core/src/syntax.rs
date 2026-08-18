#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct SubstitutionAnalysis {
    pub(crate) found: bool,
    pub(crate) valid: bool,
}

pub(crate) fn analyze_substitutions(value: &str) -> SubstitutionAnalysis {
    let bytes = value.as_bytes();
    let mut found = false;
    let mut index = 0usize;
    let mut functions = Vec::<(String, usize)>::new();
    let mut depth = 0usize;
    let mut quote = None;
    let mut in_comment = false;

    while index < bytes.len() {
        let byte = bytes[index];
        if in_comment {
            if byte == b'*' && bytes.get(index + 1) == Some(&b'/') {
                in_comment = false;
                index += 2;
                continue;
            }
            index += 1;
            continue;
        }
        if let Some(delimiter) = quote {
            if byte == b'\\' {
                index = (index + 2).min(bytes.len());
                continue;
            }
            if byte == delimiter {
                quote = None;
            }
            index += 1;
            continue;
        }
        if byte == b'/' && bytes.get(index + 1) == Some(&b'*') {
            in_comment = true;
            index += 2;
            continue;
        }
        if byte == b'\\' {
            index = (index + 2).min(bytes.len());
            continue;
        }
        if matches!(byte, b'\'' | b'"') {
            quote = Some(byte);
            index += 1;
            continue;
        }
        if depth == 0 && matches!(byte, b';' | b'!') {
            return SubstitutionAnalysis {
                found,
                valid: false,
            };
        }
        if functions.last().is_some_and(|(name, function_depth)| {
            matches!(name.as_str(), "var" | "env" | "attr" | "if")
                && ((byte == b';' && name != "if") || (byte == b'!' && depth == *function_depth))
        }) {
            return SubstitutionAnalysis {
                found,
                valid: false,
            };
        }
        if is_name_start(byte) {
            let start = index;
            index += 1;
            while index < bytes.len() && is_name_byte(bytes[index]) {
                index += 1;
            }
            if bytes.get(index) == Some(&b'(') {
                let name = value[start..index].to_ascii_lowercase();
                if matches!(name.as_str(), "var" | "env" | "attr" | "if") {
                    found = true;
                    let arguments = function_arguments(value, index + 1);
                    if !valid_substitution_function(&name, arguments) {
                        return SubstitutionAnalysis {
                            found,
                            valid: false,
                        };
                    }
                }
                depth += 1;
                functions.push((name, depth));
                index += 1;
                continue;
            }
            continue;
        }
        if matches!(byte, b'(' | b'[' | b'{') {
            depth += 1;
        } else if matches!(byte, b')' | b']' | b'}') && depth > 0 {
            if functions
                .last()
                .is_some_and(|(_, function_depth)| *function_depth == depth)
            {
                functions.pop();
            }
            depth -= 1;
        }
        index += 1;
    }

    if !value.contains("--") && !value.contains('\\') {
        return SubstitutionAnalysis { found, valid: true };
    }
    let dashed = analyze_dashed_substitutions(value);
    SubstitutionAnalysis {
        found: found || dashed.found,
        valid: dashed.valid,
    }
}

fn analyze_dashed_substitutions(value: &str) -> SubstitutionAnalysis {
    let mut input = ParserInput::new(value);
    let mut parser = Parser::new(&mut input);
    analyze_component_values(&mut parser).unwrap_or(SubstitutionAnalysis {
        found: false,
        valid: false,
    })
}

fn analyze_component_values<'i, 't>(
    input: &mut Parser<'i, 't>,
) -> Result<SubstitutionAnalysis, ParseError<'i, ()>> {
    let mut analysis = SubstitutionAnalysis {
        found: false,
        valid: true,
    };
    while let Ok(token) = input.next_including_whitespace_and_comments() {
        let token = token.clone();
        match token {
            Token::Function(name) => {
                let dashed = name.starts_with("--");
                if dashed && name.len() <= 2 {
                    return Ok(SubstitutionAnalysis {
                        found: true,
                        valid: false,
                    });
                }
                let nested = if dashed {
                    input.parse_nested_block(analyze_dashed_arguments)?
                } else {
                    input.parse_nested_block(analyze_component_values)?
                };
                analysis.found |= dashed || nested.found;
                analysis.valid &= nested.valid;
            }
            Token::ParenthesisBlock | Token::SquareBracketBlock | Token::CurlyBracketBlock => {
                let nested = input.parse_nested_block(analyze_component_values)?;
                analysis.found |= nested.found;
                analysis.valid &= nested.valid;
            }
            _ => {}
        }
    }
    Ok(analysis)
}

fn analyze_dashed_arguments<'i, 't>(
    input: &mut Parser<'i, 't>,
) -> Result<SubstitutionAnalysis, ParseError<'i, ()>> {
    let mut analysis = SubstitutionAnalysis {
        found: true,
        valid: true,
    };
    let mut comma_count = 0usize;
    let mut segment_has_value = false;
    while let Ok(token) = input.next_including_whitespace_and_comments() {
        let token = token.clone();
        match token {
            Token::WhiteSpace(_) | Token::Comment(_) => {}
            Token::Comma => {
                if comma_count > 0 && !segment_has_value {
                    analysis.valid = false;
                }
                comma_count += 1;
                segment_has_value = false;
            }
            Token::Semicolon | Token::Delim('!') => analysis.valid = false,
            Token::Function(name) => {
                let dashed = name.starts_with("--");
                if dashed && name.len() <= 2 {
                    analysis.valid = false;
                }
                let nested = if dashed {
                    input.parse_nested_block(analyze_dashed_arguments)?
                } else {
                    input.parse_nested_block(analyze_component_values)?
                };
                analysis.found |= dashed || nested.found;
                analysis.valid &= nested.valid;
                segment_has_value = true;
            }
            Token::ParenthesisBlock | Token::SquareBracketBlock | Token::CurlyBracketBlock => {
                let nested = input.parse_nested_block(analyze_component_values)?;
                analysis.found |= nested.found;
                analysis.valid &= nested.valid;
                segment_has_value = true;
            }
            _ => segment_has_value = true,
        }
    }
    if comma_count > 0 && !segment_has_value {
        analysis.valid = false;
    }
    Ok(analysis)
}

fn is_name_start(byte: u8) -> bool {
    byte.is_ascii_alphabetic() || matches!(byte, b'_' | b'-')
}

fn is_name_byte(byte: u8) -> bool {
    is_name_start(byte) || byte.is_ascii_digit()
}

fn function_arguments(value: &str, start: usize) -> &str {
    let bytes = value.as_bytes();
    let mut depth = 0usize;
    let mut index = start;
    let mut quote = None;
    while index < bytes.len() {
        let byte = bytes[index];
        if let Some(delimiter) = quote {
            if byte == b'\\' {
                index = (index + 2).min(bytes.len());
                continue;
            }
            if byte == delimiter {
                quote = None;
            }
        } else if matches!(byte, b'\'' | b'"') {
            quote = Some(byte);
        } else if matches!(byte, b'(' | b'[' | b'{') {
            depth += 1;
        } else if matches!(byte, b')' | b']' | b'}') {
            if depth == 0 {
                return &value[start..index];
            }
            depth -= 1;
        }
        index += 1;
    }
    &value[start..]
}

fn valid_substitution_function(name: &str, arguments: &str) -> bool {
    let first = trim_css_whitespace(
        arguments
            .split_once(',')
            .map_or(arguments, |(first, _)| first),
    );
    match name {
        "var" => first.starts_with("--") && first.len() > 2,
        "env" | "attr" => !first.is_empty(),
        "if" => arguments.contains(':'),
        _ => true,
    }
}

pub(crate) fn parse_declaration_list(source: &str) -> Vec<SourceDeclaration<'_>> {
    declaration_segments(source)
        .into_iter()
        .filter_map(parse_declaration)
        .collect()
}

fn declaration_segments(source: &str) -> Vec<&str> {
    let bytes = source.as_bytes();
    let mut segments = Vec::new();
    let mut start = 0usize;
    let mut index = 0usize;
    let mut depth = 0usize;
    let mut quote = None;
    let mut in_comment = false;
    while index < bytes.len() {
        let byte = bytes[index];
        if in_comment {
            if byte == b'*' && bytes.get(index + 1) == Some(&b'/') {
                in_comment = false;
                index += 2;
                continue;
            }
            index += 1;
            continue;
        }
        if let Some(delimiter) = quote {
            if byte == b'\\' {
                index = (index + 2).min(bytes.len());
                continue;
            }
            if byte == delimiter || matches!(byte, b'\n' | b'\r' | b'\x0c') {
                quote = None;
            }
            index += 1;
            continue;
        }
        if byte == b'/' && bytes.get(index + 1) == Some(&b'*') {
            in_comment = true;
            index += 2;
            continue;
        }
        if byte == b'\\' {
            index = (index + 2).min(bytes.len());
            continue;
        }
        if matches!(byte, b'\'' | b'"') {
            quote = Some(byte);
        } else if matches!(byte, b'(' | b'[' | b'{') {
            depth += 1;
        } else if matches!(byte, b')' | b']' | b'}') {
            depth = depth.saturating_sub(1);
        } else if byte == b';' && depth == 0 {
            segments.push(&source[start..index]);
            start = index + 1;
        }
        index += 1;
    }
    segments.push(&source[start..]);
    segments
}

fn parse_declaration(source: &str) -> Option<SourceDeclaration<'_>> {
    let colon = first_top_level_colon(source)?;
    let name = parse_identifier(&source[..colon])?;
    let (value, important) = split_priority(&source[colon + 1..]);
    Some(SourceDeclaration {
        name,
        value,
        important,
    })
}

fn parse_identifier(source: &str) -> Option<Cow<'_, str>> {
    let source = trim_css_whitespace(source);
    let mut input = ParserInput::new(source);
    let mut parser = Parser::new(&mut input);
    let identifier = parser.expect_ident_cloned().ok()?;
    parser.expect_exhausted().ok()?;
    Some(if identifier.as_ref() == source {
        Cow::Borrowed(source)
    } else {
        Cow::Owned(identifier.as_ref().to_owned())
    })
}

fn first_top_level_colon(source: &str) -> Option<usize> {
    let bytes = source.as_bytes();
    let mut index = 0usize;
    let mut quote = None;
    let mut in_comment = false;
    while index < bytes.len() {
        let byte = bytes[index];
        if in_comment {
            if byte == b'*' && bytes.get(index + 1) == Some(&b'/') {
                in_comment = false;
                index += 2;
                continue;
            }
            index += 1;
            continue;
        }
        if let Some(delimiter) = quote {
            if byte == b'\\' {
                index = (index + 2).min(bytes.len());
                continue;
            }
            if byte == delimiter {
                quote = None;
            }
        } else if byte == b'/' && bytes.get(index + 1) == Some(&b'*') {
            in_comment = true;
            index += 2;
            continue;
        } else if byte == b'\\' {
            index = (index + 2).min(bytes.len());
            continue;
        } else if matches!(byte, b'\'' | b'"') {
            quote = Some(byte);
        } else if byte == b':' {
            return Some(index);
        }
        index += 1;
    }
    None
}

fn split_priority(source: &str) -> (&str, bool) {
    let bytes = source.as_bytes();
    let mut index = 0usize;
    let mut depth = 0usize;
    let mut quote = None;
    let mut in_comment = false;
    let mut bang = None;
    while index < bytes.len() {
        let byte = bytes[index];
        if in_comment {
            if byte == b'*' && bytes.get(index + 1) == Some(&b'/') {
                in_comment = false;
                index += 2;
                continue;
            }
            index += 1;
            continue;
        }
        if let Some(delimiter) = quote {
            if byte == b'\\' {
                index = (index + 2).min(bytes.len());
                continue;
            }
            if byte == delimiter {
                quote = None;
            }
            index += 1;
            continue;
        }
        if byte == b'/' && bytes.get(index + 1) == Some(&b'*') {
            in_comment = true;
            index += 2;
            continue;
        }
        if byte == b'\\' {
            index = (index + 2).min(bytes.len());
            continue;
        }
        if matches!(byte, b'\'' | b'"') {
            quote = Some(byte);
        } else if matches!(byte, b'(' | b'[' | b'{') {
            depth += 1;
        } else if matches!(byte, b')' | b']' | b'}') {
            depth = depth.saturating_sub(1);
        } else if byte == b'!' && depth == 0 {
            bang = Some(index);
        }
        index += 1;
    }
    let Some(bang) = bang else {
        return (trim_css_whitespace(source), false);
    };
    let priority = remove_comments(&source[bang + 1..]);
    if trim_css_whitespace(&priority).eq_ignore_ascii_case("important") {
        (trim_css_whitespace(&source[..bang]), true)
    } else {
        (trim_css_whitespace(source), false)
    }
}

fn remove_comments(source: &str) -> String {
    let mut result = String::with_capacity(source.len());
    let mut remaining = source;
    while let Some(start) = remaining.find("/*") {
        result.push_str(&remaining[..start]);
        let Some(end) = remaining[start + 2..].find("*/") else {
            return result;
        };
        remaining = &remaining[start + end + 4..];
    }
    result.push_str(remaining);
    result
}

pub(crate) fn serialize_identifier(value: &str) -> String {
    let characters = value.chars().collect::<Vec<_>>();
    let mut result = String::new();
    for (index, character) in characters.iter().copied().enumerate() {
        let code_point = character as u32;
        if code_point == 0 {
            result.push('\u{fffd}');
        } else if (1..=31).contains(&code_point)
            || code_point == 127
            || (character.is_ascii_digit()
                && (index == 0 || (index == 1 && characters.first() == Some(&'-'))))
        {
            result.push_str(&format!("\\{code_point:x} "));
        } else if index == 0 && character == '-' && characters.len() == 1 {
            result.push_str("\\-");
        } else if code_point >= 128
            || character == '-'
            || character == '_'
            || character.is_ascii_alphanumeric()
        {
            result.push(character);
        } else {
            result.push('\\');
            result.push(character);
        }
    }
    result
}

pub(crate) fn split_top_level_whitespace(value: &str) -> Option<Vec<&str>> {
    split_top_level(
        value,
        |byte| byte.is_ascii_whitespace(),
        SeparatorMode::Collapse,
    )
}

pub(crate) fn split_top_level_delimiter(value: &str, delimiter: u8) -> Option<Vec<&str>> {
    split_top_level(
        value,
        |byte| byte == delimiter,
        SeparatorMode::RequireValues,
    )
}

pub(crate) fn split_top_level_delimiter_allow_empty(
    value: &str,
    delimiter: u8,
) -> Option<Vec<&str>> {
    split_top_level(
        value,
        |byte| byte == delimiter,
        SeparatorMode::PreserveValues,
    )
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum SeparatorMode {
    Collapse,
    RequireValues,
    PreserveValues,
}

fn split_top_level(
    value: &str,
    is_separator: impl Fn(u8) -> bool,
    separator_mode: SeparatorMode,
) -> Option<Vec<&str>> {
    let bytes = value.as_bytes();
    let mut components = Vec::new();
    let mut start = None;
    let mut depth = 0usize;
    let mut index = 0usize;
    let mut quote = None;
    let mut in_comment = false;

    while index < bytes.len() {
        let byte = bytes[index];
        if in_comment {
            if byte == b'*' && bytes.get(index + 1) == Some(&b'/') {
                in_comment = false;
                index += 2;
                continue;
            }
            index += 1;
            continue;
        }
        if let Some(delimiter) = quote {
            if byte == b'\\' {
                index = (index + 2).min(bytes.len());
                continue;
            }
            if byte == delimiter {
                quote = None;
            }
            index += 1;
            continue;
        }
        if byte == b'\\' {
            start.get_or_insert(index);
            index += 1;
            let mut hex_digits = 0;
            while hex_digits < 6
                && bytes
                    .get(index)
                    .is_some_and(|escaped| escaped.is_ascii_hexdigit())
            {
                index += 1;
                hex_digits += 1;
            }
            if hex_digits > 0 {
                if bytes.get(index) == Some(&b'\r') && bytes.get(index + 1) == Some(&b'\n') {
                    index += 2;
                } else if bytes
                    .get(index)
                    .is_some_and(|escaped| escaped.is_ascii_whitespace())
                {
                    index += 1;
                }
            } else if index < bytes.len() {
                index += 1;
            }
            continue;
        }
        if byte == b'/' && bytes.get(index + 1) == Some(&b'*') {
            if depth == 0 && is_separator(b' ') {
                if let Some(component_start) = start.take() {
                    let component = trim_css_whitespace(&value[component_start..index]);
                    if !component.is_empty() {
                        components.push(component);
                    }
                }
            }
            in_comment = true;
            index += 2;
            continue;
        }
        if matches!(byte, b'\'' | b'"') {
            quote = Some(byte);
            start.get_or_insert(index);
            index += 1;
            continue;
        }
        if matches!(byte, b'(' | b'[' | b'{') {
            depth += 1;
            start.get_or_insert(index);
            index += 1;
            continue;
        }
        if matches!(byte, b')' | b']' | b'}') {
            if depth == 0 {
                return None;
            }
            depth -= 1;
            index += 1;
            continue;
        }
        if depth == 0 && is_separator(byte) {
            if let Some(component_start) = start.take() {
                let component = trim_css_whitespace(&value[component_start..index]);
                if !component.is_empty() {
                    components.push(component);
                } else {
                    match separator_mode {
                        SeparatorMode::Collapse => {}
                        SeparatorMode::RequireValues => return None,
                        SeparatorMode::PreserveValues => components.push(""),
                    }
                }
            } else {
                match separator_mode {
                    SeparatorMode::Collapse => {}
                    SeparatorMode::RequireValues => return None,
                    SeparatorMode::PreserveValues => components.push(""),
                }
            }
            index += 1;
            continue;
        }
        start.get_or_insert(index);
        index += 1;
    }
    if depth != 0 || quote.is_some() || in_comment {
        return None;
    }
    if let Some(component_start) = start {
        let component = trim_css_whitespace(&value[component_start..]);
        if !component.is_empty() {
            components.push(component);
        } else {
            match separator_mode {
                SeparatorMode::Collapse => {}
                SeparatorMode::RequireValues => return None,
                SeparatorMode::PreserveValues => components.push(""),
            }
        }
    } else {
        match separator_mode {
            SeparatorMode::Collapse => {}
            SeparatorMode::RequireValues => return None,
            SeparatorMode::PreserveValues => components.push(""),
        }
    }
    Some(components)
}

fn trim_css_whitespace(value: &str) -> &str {
    value.trim_matches(|character| matches!(character, ' ' | '\t' | '\n' | '\r' | '\u{000c}'))
}

#[cfg(test)]
mod tests {
    use super::{
        analyze_substitutions, parse_declaration_list, serialize_identifier,
        split_top_level_delimiter_allow_empty,
    };
    use std::borrow::Cow;

    #[test]
    fn preserves_optional_empty_delimiter_sections_without_splitting_functions() {
        assert_eq!(
            split_top_level_delimiter_allow_empty("1 fill / / calc(1px / 2)", b'/'),
            Some(vec!["1 fill", "", "calc(1px / 2)"])
        );
        assert_eq!(
            split_top_level_delimiter_allow_empty("/ 1px /", b'/'),
            Some(vec!["", "1px", ""])
        );
        assert_eq!(
            split_top_level_delimiter_allow_empty("url(\"a/b\") / 1px", b'/'),
            Some(vec!["url(\"a/b\")", "1px"])
        );
        assert_eq!(
            split_top_level_delimiter_allow_empty("1 / calc(1px", b'/'),
            None
        );
        assert_eq!(
            super::split_top_level_whitespace("r\\65 peat none"),
            Some(vec!["r\\65 peat", "none"])
        );
        assert_eq!(
            split_top_level_delimiter_allow_empty("ident\\/part / 1px", b'/'),
            Some(vec!["ident\\/part", "1px"])
        );
    }

    #[test]
    fn detects_recovered_substitutions_without_accepting_declaration_boundaries() {
        let recovered = analyze_substitutions("72px var(--space, var(--space,");
        assert!(recovered.found);
        assert!(recovered.valid);

        for invalid in [
            "var(foo)",
            "var()",
            "var(--x, red); color: blue",
            "var(--x, red; color: blue)",
            "var(--x, !important)",
        ] {
            assert!(!analyze_substitutions(invalid).valid, "{invalid}");
        }
        assert!(analyze_substitutions("var(--x, fn(!important))").valid);
    }

    #[test]
    fn ignores_function_spelling_inside_strings_and_comments() {
        assert!(!analyze_substitutions("\"var(--x)\" /* env(foo) */").found);
    }

    #[test]
    fn validates_custom_dashed_substitution_functions() {
        for valid in [
            "--f()",
            "--f(1px)",
            "calc(--f() + 1px)",
            "--f(,a)",
            "--f({a,b})",
            "--f(foo(a;b))",
            "--f(foo(!))",
            "--f(a",
            "--\\66()",
        ] {
            let analysis = analyze_substitutions(valid);
            assert!(analysis.found, "{valid}");
            assert!(analysis.valid, "{valid}");
        }
        for invalid in [
            "--()",
            "--f(,)",
            "--f(a,)",
            "--f(a,,b)",
            "--f(a;b)",
            "--f(!)",
        ] {
            let analysis = analyze_substitutions(invalid);
            assert!(analysis.found, "{invalid}");
            assert!(!analysis.valid, "{invalid}");
        }
    }

    #[test]
    fn parses_declaration_boundaries_priorities_and_escaped_names() {
        let declarations = parse_declaration_list(
            "--foo\\:bar: red; width: calc(1px; 2px); color: blue ! /**/ IMPORTANT;",
        );
        assert_eq!(declarations.len(), 3);
        assert_eq!(declarations[0].name, "--foo:bar");
        assert!(matches!(declarations[0].name, Cow::Owned(_)));
        assert_eq!(declarations[0].value, "red");
        assert!(!declarations[0].important);
        assert!(matches!(declarations[1].name, Cow::Borrowed("width")));
        assert_eq!(declarations[1].value, "calc(1px; 2px)");
        assert_eq!(declarations[2].value, "blue");
        assert!(declarations[2].important);

        let non_css_whitespace =
            parse_declaration_list("--x:\u{00a0}red\u{00a0}; width:\u{00a0}1px\u{00a0};");
        assert_eq!(non_css_whitespace[0].value, "\u{00a0}red\u{00a0}");
        assert_eq!(non_css_whitespace[1].value, "\u{00a0}1px\u{00a0}");
    }

    #[test]
    fn serializes_logical_custom_property_names() {
        assert_eq!(serialize_identifier("--foo:bar"), "--foo\\:bar");
        assert_eq!(serialize_identifier("-- x"), "--\\ x");
        assert_eq!(serialize_identifier("--é"), "--é");
    }
}
use cssparser::{ParseError, Parser, ParserInput, Token};
use std::borrow::Cow;

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct SourceDeclaration<'a> {
    pub(crate) name: Cow<'a, str>,
    pub(crate) value: &'a str,
    pub(crate) important: bool,
}
