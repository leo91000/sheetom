use std::{ops::Range, sync::Arc};

use cssparser::{Token, TokenCompletion, TokenWithSpan, TokenizerWithSpans};

use crate::{EngineError, ResourceLimits};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SourceSpan {
    pub start: usize,
    pub end: usize,
}

impl SourceSpan {
    fn from_token(token: &TokenWithSpan<'_>) -> Self {
        Self {
            start: token.start.byte_index(),
            end: token.end.byte_index(),
        }
    }

    pub fn range(self) -> Range<usize> {
        self.start..self.end
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RecoveredClosure {
    Explicit,
    ImplicitEof,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RecoveredTokenTermination {
    NotApplicable,
    Explicit,
    ImplicitEof,
    Invalid,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RecoveredBlockDelimiter {
    Parenthesis,
    SquareBracket,
    CurlyBracket,
}

#[derive(Clone, Debug, PartialEq)]
pub enum RecoveredTokenKind {
    Ident(String),
    AtKeyword(String),
    Hash(String),
    IdHash(String),
    String(String),
    Url(String),
    Delimiter(char),
    Number {
        has_sign: bool,
        value: f32,
        int_value: Option<i32>,
    },
    Percentage {
        has_sign: bool,
        unit_value: f32,
        int_value: Option<i32>,
    },
    Dimension {
        has_sign: bool,
        value: f32,
        int_value: Option<i32>,
        unit: String,
    },
    Whitespace,
    Comment,
    Colon,
    Semicolon,
    Comma,
    IncludeMatch,
    DashMatch,
    PrefixMatch,
    SuffixMatch,
    SubstringMatch,
    Cdo,
    Cdc,
    BadUrl(String),
    BadString(String),
    UnmatchedClose(RecoveredBlockDelimiter),
}

#[derive(Clone, Debug, PartialEq)]
pub struct RecoveredToken {
    pub kind: RecoveredTokenKind,
    pub termination: RecoveredTokenTermination,
    pub parse_error: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub enum RecoveredComponentKind {
    Token(RecoveredToken),
    Function {
        name: String,
        opening: SourceSpan,
        values: Arc<[RecoveredComponentValue]>,
        closure: RecoveredClosure,
    },
    SimpleBlock {
        delimiter: RecoveredBlockDelimiter,
        opening: SourceSpan,
        values: Arc<[RecoveredComponentValue]>,
        closure: RecoveredClosure,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct RecoveredComponentValue {
    pub span: SourceSpan,
    pub kind: RecoveredComponentKind,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RecoveredValue {
    source: Arc<str>,
    values: Arc<[RecoveredComponentValue]>,
}

impl RecoveredValue {
    pub fn source(&self) -> &str {
        &self.source
    }

    pub fn values(&self) -> &[RecoveredComponentValue] {
        &self.values
    }

    pub fn slice(&self, span: SourceSpan) -> Option<&str> {
        self.source.get(span.range())
    }

    pub fn reparsable_css(&self) -> Result<String, EngineError> {
        enum SerializationEvent<'a> {
            Component(&'a RecoveredComponentValue),
            Close(char),
        }

        let mut output = String::with_capacity(self.source.len());
        let mut pending = Vec::with_capacity(self.values.len());
        for component in self.values.iter().rev() {
            pending.push(SerializationEvent::Component(component));
        }

        while let Some(event) = pending.pop() {
            match event {
                SerializationEvent::Close(delimiter) => output.push(delimiter),
                SerializationEvent::Component(component) => match &component.kind {
                    RecoveredComponentKind::Token(token) => {
                        serialize_recovered_token(self, component.span, token, &mut output)?;
                    }
                    RecoveredComponentKind::Function {
                        opening, values, ..
                    } => {
                        append_source_slice(self, *opening, &mut output)?;
                        pending.push(SerializationEvent::Close(')'));
                        for child in values.iter().rev() {
                            pending.push(SerializationEvent::Component(child));
                        }
                    }
                    RecoveredComponentKind::SimpleBlock {
                        delimiter,
                        opening,
                        values,
                        ..
                    } => {
                        append_source_slice(self, *opening, &mut output)?;
                        pending.push(SerializationEvent::Close(match delimiter {
                            RecoveredBlockDelimiter::Parenthesis => ')',
                            RecoveredBlockDelimiter::SquareBracket => ']',
                            RecoveredBlockDelimiter::CurlyBracket => '}',
                        }));
                        for child in values.iter().rev() {
                            pending.push(SerializationEvent::Component(child));
                        }
                    }
                },
            }
        }

        Ok(output)
    }
}

fn serialize_recovered_token(
    recovered: &RecoveredValue,
    span: SourceSpan,
    token: &RecoveredToken,
    output: &mut String,
) -> Result<(), EngineError> {
    if token.parse_error || token.termination == RecoveredTokenTermination::Invalid {
        return Err(EngineError::Serialize(
            "cannot serialize a CSS token parse error".to_owned(),
        ));
    }

    append_source_slice(recovered, span, output)?;
    if token.termination != RecoveredTokenTermination::ImplicitEof {
        return Ok(());
    }

    match token.kind {
        RecoveredTokenKind::String(_) => {
            let quote = recovered
                .source()
                .as_bytes()
                .get(span.start)
                .copied()
                .filter(|quote| matches!(*quote, b'\'' | b'"'))
                .ok_or_else(|| {
                    EngineError::Serialize("recovered string lost its quote".to_owned())
                })?;
            output.push(char::from(quote));
        }
        RecoveredTokenKind::Url(_) => output.push(')'),
        RecoveredTokenKind::Comment => output.push_str("*/"),
        _ => {}
    }
    Ok(())
}

fn append_source_slice(
    recovered: &RecoveredValue,
    span: SourceSpan,
    output: &mut String,
) -> Result<(), EngineError> {
    let source = recovered.slice(span).ok_or_else(|| {
        EngineError::Serialize("recovered CSS span is outside its source".to_owned())
    })?;
    output.push_str(source);
    Ok(())
}

#[derive(Debug)]
enum OpenComponentKind {
    Function(String),
    SimpleBlock(RecoveredBlockDelimiter),
}

#[derive(Debug)]
struct OpenComponent {
    start: usize,
    opening: SourceSpan,
    kind: OpenComponentKind,
    values: Vec<RecoveredComponentValue>,
}

impl OpenComponent {
    fn expected_close(&self) -> RecoveredBlockDelimiter {
        match self.kind {
            OpenComponentKind::Function(_) => RecoveredBlockDelimiter::Parenthesis,
            OpenComponentKind::SimpleBlock(delimiter) => delimiter,
        }
    }

    fn close(self, end: usize, closure: RecoveredClosure) -> RecoveredComponentValue {
        let kind = match self.kind {
            OpenComponentKind::Function(name) => RecoveredComponentKind::Function {
                name,
                opening: self.opening,
                values: self.values.into(),
                closure,
            },
            OpenComponentKind::SimpleBlock(delimiter) => RecoveredComponentKind::SimpleBlock {
                delimiter,
                opening: self.opening,
                values: self.values.into(),
                closure,
            },
        };

        RecoveredComponentValue {
            span: SourceSpan {
                start: self.start,
                end,
            },
            kind,
        }
    }
}

pub fn recover_component_values(source: &str) -> Result<RecoveredValue, EngineError> {
    recover_component_values_with_limits(source, ResourceLimits::default())
}

pub fn recover_component_values_with_limits(
    source: &str,
    limits: ResourceLimits,
) -> Result<RecoveredValue, EngineError> {
    if source.len() > limits.max_declaration_value_bytes {
        return Err(EngineError::InputLimitExceeded {
            actual: source.len(),
            limit: limits.max_declaration_value_bytes,
        });
    }

    let mut tokenizer = TokenizerWithSpans::new(source);
    let mut roots = Vec::new();
    let mut open = Vec::new();

    while let Ok(token) = tokenizer.next_token() {
        let span = SourceSpan::from_token(&token);
        match token.token {
            Token::Function(name) => push_open(
                &mut open,
                OpenComponent {
                    start: span.start,
                    opening: span,
                    kind: OpenComponentKind::Function(name.to_string()),
                    values: Vec::new(),
                },
                limits.max_nesting_depth,
            )?,
            Token::ParenthesisBlock => push_simple_block(
                &mut open,
                span,
                RecoveredBlockDelimiter::Parenthesis,
                limits.max_nesting_depth,
            )?,
            Token::SquareBracketBlock => push_simple_block(
                &mut open,
                span,
                RecoveredBlockDelimiter::SquareBracket,
                limits.max_nesting_depth,
            )?,
            Token::CurlyBracketBlock => push_simple_block(
                &mut open,
                span,
                RecoveredBlockDelimiter::CurlyBracket,
                limits.max_nesting_depth,
            )?,
            Token::CloseParenthesis => close_or_append(
                &mut roots,
                &mut open,
                RecoveredBlockDelimiter::Parenthesis,
                span,
            ),
            Token::CloseSquareBracket => close_or_append(
                &mut roots,
                &mut open,
                RecoveredBlockDelimiter::SquareBracket,
                span,
            ),
            Token::CloseCurlyBracket => close_or_append(
                &mut roots,
                &mut open,
                RecoveredBlockDelimiter::CurlyBracket,
                span,
            ),
            token_kind => {
                if let Some(component) = recovered_token(token_kind, token.completion, span) {
                    append_component(&mut roots, &mut open, component);
                }
            }
        }
    }

    while let Some(component) = open.pop() {
        let recovered = component.close(source.len(), RecoveredClosure::ImplicitEof);
        append_component(&mut roots, &mut open, recovered);
    }

    Ok(RecoveredValue {
        source: Arc::from(source),
        values: roots.into(),
    })
}

fn push_simple_block(
    open: &mut Vec<OpenComponent>,
    span: SourceSpan,
    delimiter: RecoveredBlockDelimiter,
    maximum_depth: usize,
) -> Result<(), EngineError> {
    push_open(
        open,
        OpenComponent {
            start: span.start,
            opening: span,
            kind: OpenComponentKind::SimpleBlock(delimiter),
            values: Vec::new(),
        },
        maximum_depth,
    )
}

fn push_open(
    open: &mut Vec<OpenComponent>,
    component: OpenComponent,
    maximum_depth: usize,
) -> Result<(), EngineError> {
    let depth = open.len() + 1;
    if depth > maximum_depth {
        return Err(EngineError::NestingLimitExceeded {
            actual: depth,
            limit: maximum_depth,
        });
    }

    open.push(component);
    Ok(())
}

fn close_or_append(
    roots: &mut Vec<RecoveredComponentValue>,
    open: &mut Vec<OpenComponent>,
    delimiter: RecoveredBlockDelimiter,
    span: SourceSpan,
) {
    let matches = open
        .last()
        .is_some_and(|component| component.expected_close() == delimiter);
    if !matches {
        append_component(
            roots,
            open,
            RecoveredComponentValue {
                span,
                kind: RecoveredComponentKind::Token(RecoveredToken {
                    kind: RecoveredTokenKind::UnmatchedClose(delimiter),
                    termination: RecoveredTokenTermination::NotApplicable,
                    parse_error: true,
                }),
            },
        );
        return;
    }

    if let Some(component) = open.pop() {
        let recovered = component.close(span.end, RecoveredClosure::Explicit);
        append_component(roots, open, recovered);
    }
}

fn append_component(
    roots: &mut Vec<RecoveredComponentValue>,
    open: &mut [OpenComponent],
    component: RecoveredComponentValue,
) {
    if let Some(parent) = open.last_mut() {
        parent.values.push(component);
        return;
    }

    roots.push(component);
}

fn recovered_token(
    token: Token<'_>,
    completion: TokenCompletion,
    span: SourceSpan,
) -> Option<RecoveredComponentValue> {
    let parse_error = token.is_parse_error();
    let termination = recovered_token_termination(&token, completion);
    let kind = match token {
        Token::Ident(value) => RecoveredTokenKind::Ident(value.to_string()),
        Token::AtKeyword(value) => RecoveredTokenKind::AtKeyword(value.to_string()),
        Token::Hash(value) => RecoveredTokenKind::Hash(value.to_string()),
        Token::IDHash(value) => RecoveredTokenKind::IdHash(value.to_string()),
        Token::QuotedString(value) => RecoveredTokenKind::String(value.to_string()),
        Token::UnquotedUrl(value) => RecoveredTokenKind::Url(value.to_string()),
        Token::Delim(value) => RecoveredTokenKind::Delimiter(value),
        Token::Number {
            has_sign,
            value,
            int_value,
        } => RecoveredTokenKind::Number {
            has_sign,
            value,
            int_value,
        },
        Token::Percentage {
            has_sign,
            unit_value,
            int_value,
        } => RecoveredTokenKind::Percentage {
            has_sign,
            unit_value,
            int_value,
        },
        Token::Dimension {
            has_sign,
            value,
            int_value,
            unit,
        } => RecoveredTokenKind::Dimension {
            has_sign,
            value,
            int_value,
            unit: unit.to_string(),
        },
        Token::WhiteSpace(_) => RecoveredTokenKind::Whitespace,
        Token::Comment(_) => RecoveredTokenKind::Comment,
        Token::Colon => RecoveredTokenKind::Colon,
        Token::Semicolon => RecoveredTokenKind::Semicolon,
        Token::Comma => RecoveredTokenKind::Comma,
        Token::IncludeMatch => RecoveredTokenKind::IncludeMatch,
        Token::DashMatch => RecoveredTokenKind::DashMatch,
        Token::PrefixMatch => RecoveredTokenKind::PrefixMatch,
        Token::SuffixMatch => RecoveredTokenKind::SuffixMatch,
        Token::SubstringMatch => RecoveredTokenKind::SubstringMatch,
        Token::CDO => RecoveredTokenKind::Cdo,
        Token::CDC => RecoveredTokenKind::Cdc,
        Token::BadUrl(value) => RecoveredTokenKind::BadUrl(value.to_string()),
        Token::BadString(value) => RecoveredTokenKind::BadString(value.to_string()),
        Token::CloseParenthesis => {
            RecoveredTokenKind::UnmatchedClose(RecoveredBlockDelimiter::Parenthesis)
        }
        Token::CloseSquareBracket => {
            RecoveredTokenKind::UnmatchedClose(RecoveredBlockDelimiter::SquareBracket)
        }
        Token::CloseCurlyBracket => {
            RecoveredTokenKind::UnmatchedClose(RecoveredBlockDelimiter::CurlyBracket)
        }
        Token::Function(_)
        | Token::ParenthesisBlock
        | Token::SquareBracketBlock
        | Token::CurlyBracketBlock => return None,
    };

    Some(RecoveredComponentValue {
        span,
        kind: RecoveredComponentKind::Token(RecoveredToken {
            kind,
            termination,
            parse_error,
        }),
    })
}

fn recovered_token_termination(
    token: &Token<'_>,
    completion: TokenCompletion,
) -> RecoveredTokenTermination {
    match token {
        Token::QuotedString(_) | Token::UnquotedUrl(_) | Token::Comment(_) | Token::BadUrl(_) => {
            match completion {
                TokenCompletion::Complete => RecoveredTokenTermination::Explicit,
                TokenCompletion::ImplicitEof => RecoveredTokenTermination::ImplicitEof,
            }
        }
        Token::BadString(_) => RecoveredTokenTermination::Invalid,
        _ => RecoveredTokenTermination::NotApplicable,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn function(
        component: &RecoveredComponentValue,
    ) -> (&str, &[RecoveredComponentValue], RecoveredClosure) {
        match &component.kind {
            RecoveredComponentKind::Function {
                name,
                values,
                closure,
                ..
            } => (name, values, *closure),
            kind => panic!("expected function, got {kind:?}"),
        }
    }

    fn token(component: &RecoveredComponentValue) -> &RecoveredToken {
        match &component.kind {
            RecoveredComponentKind::Token(token) => token,
            kind => panic!("expected token, got {kind:?}"),
        }
    }

    #[test]
    fn records_nested_pending_substitution_eof_recovery() {
        let source = "72px var(--space, var(--space,";
        let recovered = recover_component_values(source).unwrap();

        assert_eq!(recovered.source(), source);
        assert_eq!(recovered.values().len(), 3);
        assert!(matches!(
            token(&recovered.values()[0]).kind,
            RecoveredTokenKind::Dimension { .. }
        ));

        let (outer_name, outer_values, outer_closure) = function(&recovered.values()[2]);
        assert_eq!(outer_name, "var");
        assert_eq!(outer_closure, RecoveredClosure::ImplicitEof);
        let (inner_name, _, inner_closure) = function(&outer_values[3]);
        assert_eq!(inner_name, "var");
        assert_eq!(inner_closure, RecoveredClosure::ImplicitEof);
        assert_eq!(
            recovered.slice(recovered.values()[2].span),
            Some("var(--space, var(--space,")
        );
    }

    #[test]
    fn records_explicit_function_and_string_boundaries() {
        let source = "url(\"image.png\") var(--asset)";
        let recovered = recover_component_values(source).unwrap();

        let (url_name, url_values, url_closure) = function(&recovered.values()[0]);
        assert_eq!(url_name, "url");
        assert_eq!(url_closure, RecoveredClosure::Explicit);
        assert_eq!(
            token(&url_values[0]).termination,
            RecoveredTokenTermination::Explicit
        );

        let (var_name, _, var_closure) = function(&recovered.values()[2]);
        assert_eq!(var_name, "var");
        assert_eq!(var_closure, RecoveredClosure::Explicit);
        assert_eq!(
            recovered.slice(recovered.values()[0].span),
            Some("url(\"image.png\")")
        );
    }

    #[test]
    fn records_independent_eof_recovery_inside_a_function() {
        let recovered = recover_component_values("url(\"image.png").unwrap();
        let (_, values, closure) = function(&recovered.values()[0]);

        assert_eq!(closure, RecoveredClosure::ImplicitEof);
        assert_eq!(
            token(&values[0]).termination,
            RecoveredTokenTermination::ImplicitEof
        );
    }

    #[test]
    fn distinguishes_explicit_and_implicit_token_closure() {
        let cases = [
            ("\"closed\"", RecoveredTokenTermination::Explicit),
            ("\"open", RecoveredTokenTermination::ImplicitEof),
            ("\"escaped\\\"", RecoveredTokenTermination::ImplicitEof),
            ("url(done)", RecoveredTokenTermination::Explicit),
            ("url(open", RecoveredTokenTermination::ImplicitEof),
            ("/* done */", RecoveredTokenTermination::Explicit),
            ("/* open", RecoveredTokenTermination::ImplicitEof),
        ];

        for (source, expected) in cases {
            let recovered = recover_component_values(source).unwrap();
            assert_eq!(
                token(&recovered.values()[0]).termination,
                expected,
                "{source}"
            );
        }
    }

    #[test]
    fn distinguishes_tokenizer_errors_from_eof_recovery() {
        let bad_string = recover_component_values("\"bad\nnext").unwrap();
        let bad_string = token(&bad_string.values()[0]);
        assert!(bad_string.parse_error);
        assert_eq!(bad_string.termination, RecoveredTokenTermination::Invalid);

        let bad_url = recover_component_values("url(bad\"").unwrap();
        let bad_url = token(&bad_url.values()[0]);
        assert!(bad_url.parse_error);
        assert_eq!(bad_url.termination, RecoveredTokenTermination::ImplicitEof);
    }

    #[test]
    fn records_mismatched_closers_as_parse_errors() {
        let recovered = recover_component_values("(]").unwrap();
        let RecoveredComponentKind::SimpleBlock {
            values, closure, ..
        } = &recovered.values()[0].kind
        else {
            panic!("expected a simple block")
        };

        assert_eq!(*closure, RecoveredClosure::ImplicitEof);
        assert!(token(&values[0]).parse_error);
        assert_eq!(
            token(&values[0]).kind,
            RecoveredTokenKind::UnmatchedClose(RecoveredBlockDelimiter::SquareBracket)
        );
    }

    #[test]
    fn keeps_utf8_byte_spans_lossless() {
        let recovered = recover_component_values("é var(--été)").unwrap();
        assert_eq!(recovered.slice(recovered.values()[0].span), Some("é"));
        assert_eq!(
            recovered.slice(recovered.values()[2].span),
            Some("var(--été)")
        );
    }

    #[test]
    fn enforces_nesting_before_mutating_the_tree() {
        let limits = ResourceLimits {
            max_nesting_depth: 2,
            ..ResourceLimits::default()
        };
        assert_eq!(
            recover_component_values_with_limits("a(b(c()))", limits),
            Err(EngineError::NestingLimitExceeded {
                actual: 3,
                limit: 2,
            })
        );
    }

    #[test]
    fn rejects_values_over_the_owned_input_limit() {
        let limits = ResourceLimits {
            max_declaration_value_bytes: 3,
            ..ResourceLimits::default()
        };
        assert_eq!(
            recover_component_values_with_limits("four", limits),
            Err(EngineError::InputLimitExceeded {
                actual: 4,
                limit: 3,
            })
        );
    }

    #[test]
    fn builds_the_maximum_supported_depth_iteratively() {
        let source = "(".repeat(ResourceLimits::default().max_nesting_depth);
        let recovered = recover_component_values(&source).unwrap();
        let mut values = recovered.values();
        let mut observed_depth = 0;

        while let Some(value) = values.first() {
            let RecoveredComponentKind::SimpleBlock {
                values: children,
                closure,
                ..
            } = &value.kind
            else {
                break;
            };
            assert_eq!(*closure, RecoveredClosure::ImplicitEof);
            observed_depth += 1;
            values = children;
        }

        assert_eq!(observed_depth, ResourceLimits::default().max_nesting_depth);
        assert_eq!(
            recovered.reparsable_css().unwrap(),
            format!(
                "{source}{}",
                ")".repeat(ResourceLimits::default().max_nesting_depth)
            )
        );
    }

    #[test]
    fn serializes_recovery_from_structure_instead_of_raw_suffixes() {
        for (source, expected) in [
            (
                "72px var(--space, var(--space,",
                "72px var(--space, var(--space,))",
            ),
            ("\"Gotham", "\"Gotham\""),
            ("url(image.png", "url(image.png)"),
            ("red/* trailing", "red/* trailing*/"),
            ("[a{b(c", "[a{b(c)}]"),
        ] {
            assert_eq!(
                recover_component_values(source)
                    .unwrap()
                    .reparsable_css()
                    .unwrap(),
                expected
            );
        }
    }

    #[test]
    fn refuses_to_serialize_tokenizer_errors() {
        for source in ["red)", "\"bad\nnext", "url(bad\""] {
            assert!(recover_component_values(source)
                .unwrap()
                .reparsable_css()
                .is_err());
        }
    }
}
