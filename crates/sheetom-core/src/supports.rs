use crate::{DeclarationContext, DeclarationState, EngineError, ResourceLimits};
use cssparser::{Parser, ParserInput, Token, TokenizerWithSpans};
use lightningcss::{
    rules::CssRule,
    selector::{Component, PseudoClass, PseudoElement},
    stylesheet::{ParserOptions, StyleSheet},
};

/// Test authoring capability without evaluating a stylesheet or touching a DOM.
pub fn css_supports(source: &str, value: Option<&str>) -> Result<bool, EngineError> {
    let limits = ResourceLimits::default();
    if let Some(value) = value {
        let mut state =
            DeclarationState::new_with_context_and_limits(DeclarationContext::Style, limits);
        state.set_property_checked(source, value, "")?;
        return Ok(!state.is_empty());
    }
    if source.len() > limits.max_stylesheet_bytes {
        return Err(EngineError::InputLimitExceeded {
            actual: source.len(),
            limit: limits.max_stylesheet_bytes,
        });
    }
    let depth = crate::scan_safety_metrics(source).maximum_depth;
    if depth > limits.max_nesting_depth {
        return Err(EngineError::NestingLimitExceeded {
            actual: depth,
            limit: limits.max_nesting_depth,
        });
    }
    if !valid_condition_tokens(source) {
        return Ok(false);
    }
    // Parse only one boolean level at a time. Neither the parser, evaluator,
    // nor destruction of the work list consumes stack proportional to depth.
    enum Work<'a> {
        Evaluate(&'a str),
        Value(bool),
        Join(usize, bool, bool),
    }
    let mut pending = vec![Work::Evaluate(source)];
    let mut results = Vec::<bool>::new();
    while let Some(work) = pending.pop() {
        match work {
            Work::Value(value) => results.push(value),
            Work::Join(count, and, negate) => {
                let start = results
                    .len()
                    .checked_sub(count)
                    .ok_or(EngineError::UnexpectedPanic)?;
                let value = if and {
                    results[start..].iter().all(|value| *value)
                } else {
                    results[start..].iter().any(|value| *value)
                };
                results.truncate(start);
                results.push(value != negate);
            }
            Work::Evaluate(source) => {
                let mut input = ParserInput::new(source);
                let mut parser = Parser::new(&mut input);
                if parser
                    .try_parse(|p| {
                        p.expect_ident()?;
                        p.expect_colon()
                    })
                    .is_ok()
                {
                    let mut semicolon = false;
                    while let Ok(token) = parser.next() {
                        semicolon |= matches!(token, Token::Semicolon);
                    }
                    let mut state = DeclarationState::new_with_context_and_limits(
                        DeclarationContext::Style,
                        limits,
                    );
                    if !semicolon {
                        state.replace_css_text_checked(source)?;
                    }
                    results.push(!state.is_empty());
                    continue;
                }
                let negate = parser.try_parse(|p| p.expect_ident_matching("not")).is_ok();
                let mut terms = Vec::new();
                let mut operator = None;
                let valid = loop {
                    let start = parser.position();
                    let token = match parser.next() {
                        Ok(token) => token.clone(),
                        Err(_) => break false,
                    };
                    if !matches!(token, Token::ParenthesisBlock | Token::Function(_)) {
                        break false;
                    }
                    let body = parser.parse_nested_block(|p| {
                        let start = p.position();
                        while p.next_including_whitespace_and_comments().is_ok() {}
                        Ok::<_, cssparser::ParseError<'_, ()>>(p.slice_from(start))
                    });
                    let Ok(body) = body else { break false };
                    terms.push(match token {
                        Token::ParenthesisBlock => Work::Evaluate(body),
                        Token::Function(name) if name.eq_ignore_ascii_case("selector") => {
                            Work::Value(selector_supported(body))
                        }
                        _ => Work::Value(font_capability(parser.slice_from(start))),
                    });
                    if parser.is_exhausted() {
                        break true;
                    }
                    if negate {
                        break false;
                    }
                    let next = match parser.expect_ident() {
                        Ok(name) if name.eq_ignore_ascii_case("and") => true,
                        Ok(name) if name.eq_ignore_ascii_case("or") => false,
                        _ => break false,
                    };
                    if operator.is_some_and(|operator| operator != next) {
                        break false;
                    }
                    operator = Some(next);
                };
                if !valid {
                    results.push(false);
                    continue;
                }
                pending.push(Work::Join(terms.len(), operator.unwrap_or(true), negate));
                pending.extend(terms.into_iter().rev());
            }
        }
    }
    results.pop().ok_or(EngineError::UnexpectedPanic)
}

/// Flatten selector-argument functions for capability testing only. Each
/// argument is checked in its original pseudo-class grammar before replacement;
/// :has() provenance remains visible so nested :has() stays invalid. This avoids
/// both the Rust linear-memory stack and the host VM stack on deep input.
fn selector_supported(source: &str) -> bool {
    if crate::scan_safety_metrics(source).maximum_depth <= 64 {
        return selector_supported_shallow(source);
    }
    struct Node {
        start: usize,
        end: usize,
        name: String,
        pseudo_element: bool,
        children: Vec<usize>,
        replacement: String,
        has: bool,
    }
    let mut tokenizer = TokenizerWithSpans::new(source);
    let mut nodes = Vec::<Node>::new();
    let mut roots = Vec::new();
    let mut blocks = Vec::new();
    let mut current: Option<usize> = None;
    let mut colons = 0;
    while let Ok(span) = tokenizer.next_token() {
        match span.token {
            Token::Comment(_) => continue,
            Token::Colon => {
                colons += 1;
                continue;
            }
            Token::Function(name) => {
                let name = name.to_ascii_lowercase();
                let candidate = colons > 0
                    && matches!(
                        name.as_str(),
                        "is" | "where"
                            | "not"
                            | "has"
                            | "host"
                            | "slotted"
                            | "cue"
                            | "nth-child"
                            | "nth-last-child"
                    );
                let previous = current;
                let index = candidate.then_some(nodes.len());
                if let Some(index) = index {
                    if let Some(parent) = current {
                        nodes[parent].children.push(index);
                    } else {
                        roots.push(index);
                    }
                    nodes.push(Node {
                        start: span.start.byte_index(),
                        end: source.len(),
                        name,
                        pseudo_element: colons == 2,
                        children: Vec::new(),
                        replacement: String::new(),
                        has: false,
                    });
                    current = Some(index);
                }
                blocks.push((index, previous));
            }
            Token::ParenthesisBlock | Token::SquareBracketBlock | Token::CurlyBracketBlock => {
                blocks.push((None, current))
            }
            Token::CloseParenthesis | Token::CloseSquareBracket | Token::CloseCurlyBracket => {
                if let Some((index, previous)) = blocks.pop() {
                    if let Some(index) = index {
                        nodes[index].end = span.end.byte_index();
                    }
                    current = previous;
                }
            }
            _ => {}
        }
        colons = 0;
    }
    fn replace(
        source: &str,
        start: usize,
        end: usize,
        children: &[usize],
        nodes: &[Node],
    ) -> String {
        let mut result = String::new();
        let mut cursor = start;
        for index in children {
            let child = &nodes[*index];
            result.push_str(&source[cursor..child.start]);
            result.push_str(&child.replacement);
            cursor = child.end;
        }
        result.push_str(&source[cursor..end]);
        result
    }
    for index in (0..nodes.len()).rev() {
        let node = &nodes[index];
        let has_child = node.children.iter().any(|child| nodes[*child].has);
        if node.name == "has" && has_child {
            return false;
        }
        let argument = replace(source, node.start, node.end, &node.children, &nodes);
        let prefix = if node.pseudo_element { "::" } else { ":" };
        if !selector_supported_shallow(&format!("{prefix}{argument}")) {
            return false;
        }
        let contains_has = node.name == "has" || has_child;
        let nested_has = if has_child {
            ":has(.sheetom-probe)"
        } else {
            ""
        };
        let nth = if node.name.starts_with("nth-") {
            "1 of "
        } else {
            ""
        };
        let replacement = format!("{}({nth}.sheetom-probe{nested_has})", node.name);
        nodes[index].replacement = replacement;
        nodes[index].has = contains_has;
    }
    selector_supported_shallow(&replace(source, 0, source.len(), &roots, &nodes))
}

fn selector_supported_shallow(source: &str) -> bool {
    let source = format!("{source} {{}}");
    let Ok(sheet) = StyleSheet::parse(
        &source,
        ParserOptions {
            namespace_prefixes: Some(Vec::new()),
            strict_selector_lists: true,
            ..ParserOptions::default()
        },
    ) else {
        return false;
    };
    let [CssRule::Style(rule)] = sheet.rules.0.as_slice() else {
        return false;
    };
    if rule.selectors.0.len() != 1 {
        return false;
    }
    crate::selector_cssom::all_components(&rule.selectors, |component| match component {
        Component::Namespace(..) => false,
        Component::NonTSPseudoClass(PseudoClass::Lang { languages }) if languages.len() != 1 => {
            false
        }
        Component::NonTSPseudoClass(
            PseudoClass::Custom { .. }
            | PseudoClass::CustomFunction { .. }
            | PseudoClass::Closed
            | PseudoClass::LocalLink
            | PseudoClass::TargetWithin,
        )
        | Component::PseudoElement(
            PseudoElement::Custom { .. } | PseudoElement::CustomFunction { .. },
        ) => false,
        _ => true,
    })
}

fn font_capability(source: &str) -> bool {
    let mut input = ParserInput::new(source);
    let mut parser = Parser::new(&mut input);
    let name = match parser.next() {
        Ok(Token::Function(name)) => name.to_ascii_lowercase(),
        _ => return false,
    };
    let argument = parser.parse_nested_block(|input| {
        let argument = input.expect_ident()?.to_ascii_lowercase();
        input.expect_exhausted()?;
        Ok::<_, cssparser::ParseError<'_, ()>>(argument)
    });
    let Ok(argument) = argument else { return false };
    if !parser.is_exhausted() {
        return false;
    }
    match name.as_str() {
        "font-format" => matches!(
            argument.as_str(),
            "woff" | "woff2" | "truetype" | "opentype" | "collection"
        ),
        "font-tech" => matches!(
            argument.as_str(),
            "features-opentype"
                | "features-aat"
                | "variations"
                | "palettes"
                | "color-colrv0"
                | "color-colrv1"
                | "color-sbix"
                | "color-cbdt"
        ),
        _ => false,
    }
}

fn valid_condition_tokens(source: &str) -> bool {
    let mut tokenizer = TokenizerWithSpans::new(source);
    let mut blocks = Vec::new();
    while let Ok(span) = tokenizer.next_token() {
        match span.token {
            Token::BadString(_) | Token::BadUrl(_) => return false,
            Token::Function(_) | Token::ParenthesisBlock => blocks.push(')'),
            Token::SquareBracketBlock => blocks.push(']'),
            Token::CurlyBracketBlock => blocks.push('}'),
            Token::CloseParenthesis if blocks.pop() != Some(')') => return false,
            Token::CloseSquareBracket if blocks.pop() != Some(']') => return false,
            Token::CloseCurlyBracket if blocks.pop() != Some('}') => return false,
            _ => {}
        }
    }
    true
}
