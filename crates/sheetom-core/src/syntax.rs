#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct SubstitutionAnalysis {
    pub(crate) found: bool,
    pub(crate) valid: bool,
}

pub(crate) fn analyze_substitutions(value: &str) -> SubstitutionAnalysis {
    let bytes = value.as_bytes();
    let mut found = false;
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
                index += 1;
                continue;
            }
            continue;
        }
        if matches!(byte, b'(' | b'[' | b'{') {
            depth += 1;
        } else if matches!(byte, b')' | b']' | b'}') && depth > 0 {
            depth -= 1;
        }
        index += 1;
    }

    SubstitutionAnalysis { found, valid: true }
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
    let first = arguments
        .split_once(',')
        .map_or(arguments, |(first, _)| first)
        .trim();
    match name {
        "var" => first.starts_with("--") && first.len() > 2,
        "env" | "attr" => !first.is_empty(),
        "if" => arguments.contains(':'),
        _ => true,
    }
}

pub(crate) fn parse_declaration_list(source: &str) -> Vec<SourceDeclaration> {
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

fn parse_declaration(source: &str) -> Option<SourceDeclaration> {
    let colon = first_top_level_colon(source)?;
    let name = parse_identifier(&source[..colon])?;
    let (value, important) = split_priority(&source[colon + 1..]);
    Some(SourceDeclaration {
        name,
        value: value.to_owned(),
        important,
    })
}

fn parse_identifier(source: &str) -> Option<String> {
    let mut input = ParserInput::new(source);
    let mut parser = Parser::new(&mut input);
    let identifier = parser.expect_ident_cloned().ok()?;
    parser.expect_exhausted().ok()?;
    Some(identifier.as_ref().to_owned())
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
        return (source.trim(), false);
    };
    let priority = remove_comments(&source[bang + 1..]);
    if priority.trim().eq_ignore_ascii_case("important") {
        (source[..bang].trim(), true)
    } else {
        (source.trim(), false)
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
    split_top_level(value, |byte| byte.is_ascii_whitespace(), false)
}

pub(crate) fn split_top_level_delimiter(value: &str, delimiter: u8) -> Option<Vec<&str>> {
    split_top_level(value, |byte| byte == delimiter, true)
}

fn split_top_level(
    value: &str,
    is_separator: impl Fn(u8) -> bool,
    require_nonempty_parts: bool,
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
        if byte == b'/' && bytes.get(index + 1) == Some(&b'*') {
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
                let component = value[component_start..index].trim();
                if !component.is_empty() {
                    components.push(component);
                }
            } else if require_nonempty_parts {
                return None;
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
        let component = value[component_start..].trim();
        if !component.is_empty() {
            components.push(component);
        }
    } else if require_nonempty_parts {
        return None;
    }
    Some(components)
}

#[cfg(test)]
mod tests {
    use super::{analyze_substitutions, parse_declaration_list, serialize_identifier};

    #[test]
    fn detects_recovered_substitutions_without_accepting_declaration_boundaries() {
        let recovered = analyze_substitutions("72px var(--space, var(--space,");
        assert!(recovered.found);
        assert!(recovered.valid);

        for invalid in ["var(foo)", "var()", "var(--x, red); color: blue"] {
            assert!(!analyze_substitutions(invalid).valid, "{invalid}");
        }
    }

    #[test]
    fn ignores_function_spelling_inside_strings_and_comments() {
        assert!(!analyze_substitutions("\"var(--x)\" /* env(foo) */").found);
    }

    #[test]
    fn parses_declaration_boundaries_priorities_and_escaped_names() {
        let declarations = parse_declaration_list(
            "--foo\\:bar: red; width: calc(1px; 2px); color: blue ! /**/ IMPORTANT;",
        );
        assert_eq!(declarations.len(), 3);
        assert_eq!(declarations[0].name, "--foo:bar");
        assert_eq!(declarations[0].value, "red");
        assert!(!declarations[0].important);
        assert_eq!(declarations[1].value, "calc(1px; 2px)");
        assert_eq!(declarations[2].value, "blue");
        assert!(declarations[2].important);
    }

    #[test]
    fn serializes_logical_custom_property_names() {
        assert_eq!(serialize_identifier("--foo:bar"), "--foo\\:bar");
        assert_eq!(serialize_identifier("-- x"), "--\\ x");
        assert_eq!(serialize_identifier("--é"), "--é");
    }
}
use cssparser::{Parser, ParserInput};

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct SourceDeclaration {
    pub(crate) name: String,
    pub(crate) value: String,
    pub(crate) important: bool,
}
