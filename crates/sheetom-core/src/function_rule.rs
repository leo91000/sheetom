use crate::{
    shorthand::{parse_value, ParsedValue},
    syntax::{analyze_substitutions, parse_declaration_list, serialize_identifier},
};
use cssparser::{ParseError, Parser, ParserInput, Token};
use lightningcss::values::syntax::SyntaxString;

const MAX_FUNCTION_PARAMETERS: usize = 100_000;

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ParsedFunctionPrelude {
    pub(crate) name: String,
    pub(crate) parameters: Vec<ParsedFunctionParameter>,
    pub(crate) return_type: String,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ParsedFunctionParameter {
    pub(crate) name: String,
    pub(crate) value_type: String,
    pub(crate) default_value: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
struct FunctionSyntax {
    components: Vec<FunctionSyntaxComponent>,
    universal: bool,
}

#[derive(Clone, Debug, PartialEq)]
struct FunctionSyntaxComponent {
    kind: FunctionSyntaxKind,
    multiplier: FunctionSyntaxMultiplier,
}

#[derive(Clone, Debug, PartialEq)]
enum FunctionSyntaxKind {
    Standard(&'static str),
    Literal(String),
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum FunctionSyntaxMultiplier {
    None,
    Space,
    Comma,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum DefaultValueDisposition {
    Accepted,
    Omitted,
    InvalidRule,
}

pub(crate) fn parse_function_prelude(header: &str) -> Option<ParsedFunctionPrelude> {
    let mut input = ParserInput::new(header);
    let mut parser = Parser::new(&mut input);
    let at_keyword = match parser.next().ok()? {
        Token::AtKeyword(value) => value,
        _ => return None,
    };
    if !at_keyword.eq_ignore_ascii_case("function") {
        return None;
    }

    let name = match parser.next().ok()? {
        Token::Function(value) if valid_dashed_identifier(value) => value.to_string(),
        _ => return None,
    };
    let parameters = parser.parse_nested_block(parse_function_parameters).ok()?;

    let return_type = if parser.is_exhausted() {
        "*".to_owned()
    } else {
        parser.expect_ident_matching("returns").ok()?;
        let start = parser.position();
        consume_remaining_tokens(&mut parser).ok()?;
        let syntax = parse_css_type(parser.slice_from(start))?;
        syntax.serialize()
    };
    parser.is_exhausted().then_some(ParsedFunctionPrelude {
        name,
        parameters,
        return_type,
    })
}

pub(crate) fn canonical_function_descriptor_name(name: &str) -> Option<String> {
    if name.starts_with("--") {
        return (name.len() > 2).then(|| name.to_owned());
    }
    name.eq_ignore_ascii_case("result")
        .then(|| "result".to_owned())
}

pub(crate) fn parse_function_descriptor_value(name: &str, value: &str) -> Option<ParsedValue> {
    if name.starts_with("--") {
        return parse_value(name, value, false).ok();
    }
    (name == "result")
        .then(|| parse_value("--sheetom-function-result", value, false).ok())
        .flatten()
}

fn parse_function_parameters<'i, 't>(
    input: &mut Parser<'i, 't>,
) -> Result<Vec<ParsedFunctionParameter>, ParseError<'i, ()>> {
    skip_css_trivia(input);
    if input.is_exhausted() {
        return Ok(Vec::new());
    }
    let mut count = 0usize;
    input.parse_comma_separated(|parameter| {
        count += 1;
        if count > MAX_FUNCTION_PARAMETERS {
            return Err(parameter.new_custom_error(()));
        }
        parse_function_parameter(parameter)
    })
}

fn parse_function_parameter<'i, 't>(
    input: &mut Parser<'i, 't>,
) -> Result<ParsedFunctionParameter, ParseError<'i, ()>> {
    skip_css_trivia(input);
    let name = input.expect_ident_cloned()?;
    if !valid_dashed_identifier(&name) {
        return Err(input.new_custom_error(()));
    }

    let rest_start = input.position();
    consume_remaining_tokens(input)?;
    let rest = input.slice_from(rest_start);
    let (type_source, default_value) = split_parameter_default(rest);
    let type_source = trim_css_trivia(type_source);
    let syntax = if type_source.is_empty() {
        FunctionSyntax::universal()
    } else {
        parse_css_type(type_source).ok_or_else(|| input.new_custom_error(()))?
    };
    let default_value = match default_value {
        Some(raw_value) => match classify_default_value(
            trim_css_trivia(raw_value),
            &syntax,
            has_recovery_blocking_trailing_whitespace(raw_value),
        ) {
            DefaultValueDisposition::Accepted => Some(trim_css_trivia(raw_value)),
            DefaultValueDisposition::Omitted => None,
            DefaultValueDisposition::InvalidRule => return Err(input.new_custom_error(())),
        },
        None => None,
    };

    Ok(ParsedFunctionParameter {
        name: name.to_string(),
        value_type: syntax.serialize(),
        default_value: default_value.map(str::to_owned),
    })
}

fn skip_css_trivia(input: &mut Parser<'_, '_>) {
    loop {
        let state = input.state();
        match input.next_including_whitespace_and_comments() {
            Ok(Token::WhiteSpace(_) | Token::Comment(_)) => {}
            _ => {
                input.reset(&state);
                return;
            }
        }
    }
}

fn consume_remaining_tokens<'i, 't>(input: &mut Parser<'i, 't>) -> Result<(), ParseError<'i, ()>> {
    while let Ok(token) = input.next_including_whitespace_and_comments() {
        if matches!(
            token,
            Token::Function(_)
                | Token::ParenthesisBlock
                | Token::SquareBracketBlock
                | Token::CurlyBracketBlock
        ) {
            input.parse_nested_block(consume_remaining_tokens)?;
        }
    }
    Ok(())
}

fn split_parameter_default(source: &str) -> (&str, Option<&str>) {
    let bytes = source.as_bytes();
    let mut quote = None;
    let mut escaped = false;
    let mut comment = false;
    let mut depth = 0usize;
    let mut index = 0usize;
    while index < bytes.len() {
        let byte = bytes[index];
        if comment {
            if byte == b'*' && bytes.get(index + 1) == Some(&b'/') {
                comment = false;
                index += 2;
                continue;
            }
            index += 1;
            continue;
        }
        if escaped {
            escaped = false;
            index += 1;
            continue;
        }
        if let Some(delimiter) = quote {
            if byte == b'\\' {
                escaped = true;
            } else if byte == delimiter {
                quote = None;
            }
            index += 1;
            continue;
        }
        if byte == b'/' && bytes.get(index + 1) == Some(&b'*') {
            comment = true;
            index += 2;
            continue;
        }
        if matches!(byte, b'\'' | b'"') {
            quote = Some(byte);
            index += 1;
            continue;
        }
        match byte {
            b'(' | b'[' | b'{' => depth += 1,
            b')' | b']' | b'}' if depth > 0 => depth -= 1,
            b':' if depth == 0 => return (&source[..index], Some(&source[index + 1..])),
            _ => {}
        }
        index += 1;
    }
    (source, None)
}

fn trim_css_trivia(mut value: &str) -> &str {
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

fn has_recovery_blocking_trailing_whitespace(mut value: &str) -> bool {
    loop {
        if value
            .chars()
            .next_back()
            .is_some_and(|character| matches!(character, ' ' | '\t' | '\n' | '\r' | '\u{000c}'))
        {
            return true;
        }
        let Some(comment_body) = value.strip_suffix("*/") else {
            return false;
        };
        let Some(start) = comment_body.rfind("/*") else {
            return false;
        };
        value = &comment_body[..start];
    }
}

fn parse_css_type(source: &str) -> Option<FunctionSyntax> {
    let source = trim_css_trivia(source);
    let (source, wrapped) =
        unwrap_type_function(source).map_or((source, false), |body| (body, true));
    let syntax = FunctionSyntax::parse(source, wrapped)?;
    if syntax.universal && !wrapped {
        return None;
    }
    Some(syntax)
}

fn unwrap_type_function(source: &str) -> Option<&str> {
    let mut input = ParserInput::new(source);
    let mut parser = Parser::new(&mut input);
    let name = match parser.next().ok()? {
        Token::Function(name) => name,
        _ => return None,
    };
    if !name.eq_ignore_ascii_case("type") {
        return None;
    }
    let body = parser
        .parse_nested_block(|nested| {
            let start = nested.position();
            consume_remaining_tokens(nested)?;
            Ok(nested.slice_from(start))
        })
        .ok()?;
    parser.is_exhausted().then_some(body)
}

impl FunctionSyntax {
    fn universal() -> Self {
        Self {
            components: Vec::new(),
            universal: true,
        }
    }

    fn parse(source: &str, allow_union: bool) -> Option<Self> {
        let mut input = ParserInput::new(source);
        let mut parser = Parser::new(&mut input);
        let initial_state = parser.state();
        if matches!(parser.next().ok(), Some(Token::Delim('*'))) {
            return parser.is_exhausted().then(Self::universal);
        }
        parser.reset(&initial_state);

        let mut components = Vec::new();
        loop {
            components.push(FunctionSyntaxComponent::parse(&mut parser)?);
            if parser.is_exhausted() {
                break;
            }
            if !allow_union || !matches!(parser.next().ok(), Some(Token::Delim('|'))) {
                return None;
            }
            if parser.is_exhausted() {
                return None;
            }
        }
        (!components.is_empty()).then_some(Self {
            components,
            universal: false,
        })
    }

    fn serialize(&self) -> String {
        if self.universal {
            return "*".to_owned();
        }
        self.components
            .iter()
            .map(FunctionSyntaxComponent::serialize)
            .collect::<Vec<_>>()
            .join(" | ")
    }

    fn accepts(&self, value: &str) -> bool {
        if self.universal {
            return true;
        }
        self.components
            .iter()
            .any(|component| component.accepts(value))
    }
}

impl FunctionSyntaxComponent {
    fn parse(parser: &mut Parser<'_, '_>) -> Option<Self> {
        let kind = match parser.next().ok()? {
            Token::Delim('<') => {
                let name = match parser.next().ok()? {
                    Token::Ident(name) => supported_type_name(name)?,
                    _ => return None,
                };
                if !matches!(parser.next().ok(), Some(Token::Delim('>'))) {
                    return None;
                }
                FunctionSyntaxKind::Standard(name)
            }
            Token::Ident(name) => FunctionSyntaxKind::Literal(name.to_string()),
            _ => return None,
        };
        let state = parser.state();
        let multiplier = match parser.next().ok() {
            Some(Token::Delim('+')) => FunctionSyntaxMultiplier::Space,
            Some(Token::Delim('#')) => FunctionSyntaxMultiplier::Comma,
            _ => {
                parser.reset(&state);
                FunctionSyntaxMultiplier::None
            }
        };
        if matches!(kind, FunctionSyntaxKind::Standard("transform-list"))
            && multiplier != FunctionSyntaxMultiplier::None
        {
            return None;
        }
        Some(Self { kind, multiplier })
    }

    fn serialize(&self) -> String {
        let mut result = match &self.kind {
            FunctionSyntaxKind::Standard(name) => format!("<{name}>"),
            FunctionSyntaxKind::Literal(name) => serialize_identifier(name),
        };
        match self.multiplier {
            FunctionSyntaxMultiplier::None => {}
            FunctionSyntaxMultiplier::Space => result.push('+'),
            FunctionSyntaxMultiplier::Comma => result.push('#'),
        }
        result
    }

    fn accepts(&self, value: &str) -> bool {
        match &self.kind {
            FunctionSyntaxKind::Standard(_) => {
                let Ok(syntax) = SyntaxString::parse_string(&self.serialize()) else {
                    return false;
                };
                let mut input = ParserInput::new(value);
                let mut parser = Parser::new(&mut input);
                syntax.parse_value(&mut parser).is_ok() && parser.is_exhausted()
            }
            FunctionSyntaxKind::Literal(expected) => {
                validate_literal_value(value, expected, self.multiplier)
            }
        }
    }
}

fn classify_default_value(
    value: &str,
    syntax: &FunctionSyntax,
    recovery_blocked: bool,
) -> DefaultValueDisposition {
    if has_top_level_bang(value) {
        return DefaultValueDisposition::InvalidRule;
    }
    let declaration = format!("--sheetom-function-parameter:{value}");
    let declarations = parse_declaration_list(&declaration);
    let [declaration] = declarations.as_slice() else {
        return DefaultValueDisposition::InvalidRule;
    };
    if declaration.important {
        return DefaultValueDisposition::InvalidRule;
    }
    let substitutions = analyze_substitutions(value);
    if !substitutions.valid {
        return if substitutions.found
            && !recovery_blocked
            && terminal_component_has_invalid_substitution(value)
        {
            DefaultValueDisposition::Omitted
        } else {
            DefaultValueDisposition::InvalidRule
        };
    }
    if substitutions.found {
        return DefaultValueDisposition::Accepted;
    }
    if syntax.accepts(value) {
        DefaultValueDisposition::Accepted
    } else {
        DefaultValueDisposition::InvalidRule
    }
}

fn terminal_component_has_invalid_substitution(value: &str) -> bool {
    let mut input = ParserInput::new(value);
    let mut parser = Parser::new(&mut input);
    let mut terminal_invalid = false;
    while !parser.is_exhausted() {
        let start = parser.position();
        let token = match parser.next_including_whitespace_and_comments() {
            Ok(token) => token.clone(),
            Err(_) => return false,
        };
        if matches!(token, Token::WhiteSpace(_) | Token::Comment(_)) {
            continue;
        }
        if matches!(
            token,
            Token::Function(_)
                | Token::ParenthesisBlock
                | Token::SquareBracketBlock
                | Token::CurlyBracketBlock
        ) && parser.parse_nested_block(consume_remaining_tokens).is_err()
        {
            return false;
        }
        let component = parser.slice(start..parser.position());
        let substitutions = analyze_substitutions(component);
        terminal_invalid = substitutions.found && !substitutions.valid;
    }
    terminal_invalid
}

fn has_top_level_bang(value: &str) -> bool {
    let mut input = ParserInput::new(value);
    let mut parser = Parser::new(&mut input);
    while let Ok(token) = parser.next_including_whitespace_and_comments() {
        match token {
            Token::Delim('!') => return true,
            Token::Function(_)
            | Token::ParenthesisBlock
            | Token::SquareBracketBlock
            | Token::CurlyBracketBlock => {
                if parser.parse_nested_block(consume_remaining_tokens).is_err() {
                    return true;
                }
            }
            _ => {}
        }
    }
    false
}

fn validate_literal_value(
    value: &str,
    expected: &str,
    multiplier: FunctionSyntaxMultiplier,
) -> bool {
    let mut input = ParserInput::new(value);
    let mut parser = Parser::new(&mut input);
    match multiplier {
        FunctionSyntaxMultiplier::None => {
            parser
                .expect_ident()
                .is_ok_and(|actual| actual.as_ref() == expected)
                && parser.is_exhausted()
        }
        FunctionSyntaxMultiplier::Space => {
            let mut count = 0usize;
            while parser
                .try_parse(|input| input.expect_ident_cloned())
                .is_ok_and(|actual| actual.as_ref() == expected)
            {
                count += 1;
            }
            count > 0 && parser.is_exhausted()
        }
        FunctionSyntaxMultiplier::Comma => {
            parser
                .parse_comma_separated(|input| {
                    let actual = input.expect_ident_cloned()?;
                    if actual.as_ref() == expected {
                        Ok(())
                    } else {
                        Err(input.new_custom_error::<(), ()>(()))
                    }
                })
                .is_ok()
                && parser.is_exhausted()
        }
    }
}

fn supported_type_name(name: &str) -> Option<&'static str> {
    [
        "angle",
        "color",
        "custom-ident",
        "image",
        "integer",
        "length",
        "length-percentage",
        "number",
        "percentage",
        "resolution",
        "string",
        "time",
        "transform-function",
        "transform-list",
        "url",
    ]
    .into_iter()
    .find(|candidate| name.eq_ignore_ascii_case(candidate))
}

fn valid_dashed_identifier(value: &str) -> bool {
    value.starts_with("--") && value.len() > 2
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    #[test]
    fn matches_the_versioned_chromium_and_wpt_prelude_corpus() {
        let corpus: Value = serde_json::from_str(include_str!(
            "../../../compatibility/function-rule-cases.json"
        ))
        .unwrap();
        for test_case in corpus["cases"].as_array().unwrap() {
            let prelude = test_case["prelude"].as_str().unwrap();
            let accepted = test_case["accepted"].as_bool().unwrap();
            assert_eq!(
                parse_function_prelude(prelude).is_some(),
                accepted,
                "{}: {prelude}",
                test_case["id"].as_str().unwrap()
            );
        }
    }

    #[test]
    fn omits_invalid_substitution_defaults_without_dropping_the_function() {
        for invalid_default in [
            "var(foo)",
            "env()",
            "attr()",
            "if()",
            "--fallback(,)",
            "1px var(foo)",
            "foo --fallback(,)",
        ] {
            let prelude = format!("@function --foo(--x <length>: {invalid_default})");
            let parsed = parse_function_prelude(&prelude).unwrap_or_else(|| {
                panic!("function should survive invalid default: {invalid_default}")
            });
            assert_eq!(
                parsed.parameters,
                vec![ParsedFunctionParameter {
                    name: "--x".to_owned(),
                    value_type: "<length>".to_owned(),
                    default_value: None,
                }],
                "{invalid_default}"
            );
        }
    }

    #[test]
    fn rejects_tokens_after_an_invalid_substitution_default() {
        for invalid_default in [
            "var(foo) 1px",
            "var(foo), var(--x)",
            "--fallback(,) foo",
            "var(foo)\u{00a0}",
        ] {
            let prelude = format!("@function --foo(--x <length>: {invalid_default})");
            assert!(
                parse_function_prelude(&prelude).is_none(),
                "{invalid_default}"
            );
        }
    }

    #[test]
    fn bounds_function_parameter_count() {
        let parameters = std::iter::repeat_n("--x", MAX_FUNCTION_PARAMETERS + 1)
            .collect::<Vec<_>>()
            .join(",");
        assert!(parse_function_prelude(&format!("@function --foo({parameters})")).is_none());
    }
}
