//! Authoring syntax for the revision-pinned experimental mixin contract.
//! Evaluation is deliberately absent: calls and unresolved names remain authored.
use super::*;
use crate::function_rule::parse_mixin_prelude;

pub(super) fn keyword(source: &str) -> Option<String> {
    let mut input = ParserInput::new(source);
    let mut parser = Parser::new(&mut input);
    match parser.next().ok()? {
        Token::AtKeyword(name) => Some(name.to_ascii_lowercase()),
        _ => None,
    }
}

pub(super) fn is_mixin_syntax(source: &str) -> bool {
    matches!(
        keyword(source).as_deref(),
        Some("mixin" | "apply" | "contents" | "private")
    )
}

fn empty_rule(kind: &str) -> ParsedRule {
    ParsedRule {
        kind: kind.to_owned(),
        prelude: String::new(),
        declarations: String::new(),
        children: Vec::new(),
        css_text: String::new(),
    }
}

fn parse_shell(source: &str) -> Option<(ParsedRule, Option<&str>)> {
    let block = split_outer_block(source);
    let header = block.map_or_else(|| source.trim_end_matches(';'), |(header, _)| header);
    let body = block.map(|(_, body)| body);
    let name = keyword(header)?;
    match name.as_str() {
        "mixin" => {
            body?;
            let prelude = parse_mixin_prelude(header)?;
            let mut rule = empty_rule("mixin");
            rule.prelude = prelude.name;
            rule.children = prelude
                .parameters
                .iter()
                .map(function_parameter_metadata)
                .collect();
            Some((rule, body))
        }
        "apply" => {
            let mut input = ParserInput::new(header);
            let mut parser = Parser::new(&mut input);
            parser.next().ok()?;
            let (name, arguments) = match parser.next().ok()?.clone() {
                Token::Ident(value) if value.starts_with("--") && value.len() > 2 => {
                    (value.to_string(), Vec::new())
                }
                Token::Function(value) if value.starts_with("--") && value.len() > 2 => {
                    let arguments = parser.parse_nested_block(parse_arguments).ok()?;
                    (value.to_string(), arguments)
                }
                _ => return None,
            };
            if !parser.is_exhausted() {
                return None;
            }
            let mut rule = empty_rule(if body.is_some() {
                "apply-block"
            } else {
                "apply-statement"
            });
            rule.prelude = name;
            rule.children = arguments
                .into_iter()
                .map(|argument| {
                    let mut metadata = empty_rule("mixin-argument");
                    metadata.css_text = serialize_argument(&argument);
                    metadata.declarations = argument;
                    metadata
                })
                .collect();
            Some((rule, body))
        }
        "private" => {
            body?;
            let mut input = ParserInput::new(header);
            let mut parser = Parser::new(&mut input);
            parser.next().ok()?;
            if !parser.is_exhausted() {
                return None;
            }
            // The pinned draft defines no CSSOM interface for @private. Preserve
            // its authored body as an opaque rule; never invent editable fields.
            let mut rule = empty_rule("private");
            rule.css_text = source.to_owned();
            Some((rule, None))
        }
        "contents" => {
            let mut input = ParserInput::new(header);
            let mut parser = Parser::new(&mut input);
            parser.next().ok()?;
            if !parser.is_exhausted() {
                return None;
            }
            Some((
                empty_rule(if body.is_some() {
                    "contents-block"
                } else {
                    "contents-statement"
                }),
                body,
            ))
        }
        _ => None,
    }
}

fn parse_arguments<'i, 't>(
    parser: &mut Parser<'i, 't>,
) -> Result<Vec<String>, cssparser::ParseError<'i, ()>> {
    if parser.is_exhausted() {
        return Ok(Vec::new());
    }
    let mut arguments = Vec::new();
    let mut start = parser.position();
    while !parser.is_exhausted() {
        let before = parser.position();
        match parser.next_including_whitespace_and_comments()?.clone() {
            Token::Comma => {
                arguments.push(
                    argument_value(parser.slice(start..before))
                        .ok_or_else(|| parser.new_custom_error(()))?,
                );
                start = parser.position();
            }
            Token::Function(_)
            | Token::ParenthesisBlock
            | Token::SquareBracketBlock
            | Token::CurlyBracketBlock => {
                consume_nested_block(parser).map_err(|_| parser.new_custom_error(()))?;
            }
            Token::Semicolon
            | Token::Delim('!')
            | Token::BadString(_)
            | Token::BadUrl(_)
            | Token::CloseParenthesis
            | Token::CloseSquareBracket
            | Token::CloseCurlyBracket => {
                return Err(parser.new_custom_error(()));
            }
            _ => {}
        }
    }
    arguments
        .push(argument_value(parser.slice_from(start)).ok_or_else(|| parser.new_custom_error(()))?);
    Ok(arguments)
}

fn argument_value(source: &str) -> Option<String> {
    let source = trim_css_whitespace(source);
    let wrapped =
        split_outer_block(source).filter(|(header, _)| trim_css_whitespace(header).is_empty());
    let value = wrapped.map_or(source, |(_, body)| trim_css_whitespace(body));
    let mut pending = vec![(value, true)];
    while let Some((source, top)) = pending.pop() {
        let mut input = ParserInput::new(source);
        let mut parser = Parser::new(&mut input);
        while let Ok(token) = parser.next_including_whitespace_and_comments() {
            if token.is_parse_error()
                || (top && matches!(token, Token::Semicolon | Token::Delim('!')))
            {
                return None;
            }
            if matches!(token, Token::CurlyBracketBlock) && top && wrapped.is_none() {
                return None;
            }
            if matches!(
                token,
                Token::Function(_)
                    | Token::ParenthesisBlock
                    | Token::SquareBracketBlock
                    | Token::CurlyBracketBlock
            ) {
                let nested = parser
                    .parse_nested_block(|p| {
                        let start = p.position();
                        while p.next_including_whitespace_and_comments().is_ok() {}
                        Ok::<_, cssparser::ParseError<'_, ()>>(p.slice_from(start))
                    })
                    .ok()?;
                pending.push((nested, false));
            }
        }
    }
    Some(value.to_owned())
}

fn serialize_argument(value: &str) -> String {
    let mut input = ParserInput::new(value);
    let mut parser = Parser::new(&mut input);
    while let Ok(token) = parser.next_including_whitespace_and_comments() {
        match token {
            Token::Comma | Token::CurlyBracketBlock => return format!("{{ {value} }}"),
            Token::Function(_) | Token::ParenthesisBlock | Token::SquareBracketBlock => {
                if consume_nested_block(&mut parser).is_err() {
                    break;
                }
            }
            _ => {}
        }
    }
    value.to_owned()
}

/// Assemble nested mixin/style/group bodies iteratively so configured deep
/// syntax budgets do not turn a valid stylesheet into a process-stack limit.
pub(super) fn parse(source: &str, depth: usize) -> Result<ParsedRule, EngineError> {
    if !contains_exactly_one_rule(source)? && !contains_exactly_one_rule(&format!("{source};"))? {
        return Err(EngineError::Parse(
            "a mixin mutation must contain exactly one rule".to_owned(),
        ));
    }
    let (root, body) = parse_shell(source)
        .ok_or_else(|| EngineError::Parse("invalid experimental mixin rule".to_owned()))?;
    let mut nodes = vec![Some(root)];
    let mut children = vec![Vec::<usize>::new()];
    let mut pending = vec![(0usize, body, depth)];
    let limits = current_resource_limits();
    while let Some((index, body, depth)) = pending.pop() {
        if depth > limits.max_nesting_depth {
            return Err(EngineError::NestingLimitExceeded {
                actual: depth,
                limit: limits.max_nesting_depth,
            });
        }
        let Some(body) = body else { continue };
        let mut declarations = Vec::new();
        let mut parts = Vec::new();
        for fragment in scan_items(body)? {
            if keyword(fragment).as_deref() == Some("mixin") {
                continue;
            }
            let shell = if is_mixin_syntax(fragment) {
                parse_shell(fragment)
            } else if let Some((header, nested_body)) = split_outer_block(fragment) {
                let probe = format!("{header}{{}}");
                parse_rule_tree_active(&probe).ok().and_then(|mut rule| {
                    if !matches!(
                        rule.kind.as_str(),
                        "style"
                            | "media"
                            | "supports"
                            | "container"
                            | "layer-block"
                            | "scope"
                            | "starting-style"
                    ) {
                        return None;
                    }
                    preserve_source_prelude(&mut rule, header);
                    rule.declarations.clear();
                    rule.children.clear();
                    rule.css_text.clear();
                    Some((rule, Some(nested_body)))
                })
            } else {
                None
            };
            if let Some(shell) = shell {
                flush_declarations(&mut parts, &mut declarations);
                parts.push(shell);
            } else if !is_at_rule_source(fragment) {
                declarations.push(fragment);
            }
        }
        flush_declarations(&mut parts, &mut declarations);
        // Style rules own the initial declaration run; group-like mixin blocks
        // expose every declaration run as CSSNestedDeclarations instead.
        if nodes[index]
            .as_ref()
            .is_some_and(|rule| rule.kind == "style")
            && parts
                .first()
                .is_some_and(|(rule, _)| rule.kind == "nested-declarations")
        {
            let (mut declarations, _) = parts.remove(0);
            nodes[index]
                .as_mut()
                .ok_or(EngineError::UnexpectedPanic)?
                .declarations = std::mem::take(&mut declarations.declarations);
        }
        for (rule, body) in parts {
            let child_index = nodes.len();
            if child_index >= limits.max_rules {
                return Err(EngineError::RuleLimitExceeded {
                    actual: child_index + 1,
                    limit: limits.max_rules,
                });
            }
            nodes.push(Some(rule));
            children.push(Vec::new());
            children[index].push(child_index);
            pending.push((child_index, body, depth + 1));
        }
    }
    for index in (0..nodes.len()).rev() {
        for child_index in &children[index] {
            let child = nodes[*child_index]
                .take()
                .ok_or(EngineError::UnexpectedPanic)?;
            nodes[index]
                .as_mut()
                .ok_or(EngineError::UnexpectedPanic)?
                .children
                .push(child);
        }
    }
    nodes[0].take().ok_or(EngineError::UnexpectedPanic)
}

pub(super) fn scan_items(source: &str) -> Result<Vec<&str>, EngineError> {
    let mut input = ParserInput::new(source);
    let mut parser = Parser::new(&mut input);
    let mut items = Vec::new();
    while !parser.is_exhausted() {
        let start = parser.position();
        let first = match parser.next_including_whitespace_and_comments() {
            Ok(token) => token.clone(),
            Err(_) => break,
        };
        if is_rule_trivia(&first) || matches!(first, Token::Semicolon) {
            continue;
        }
        let custom_property = matches!(&first, Token::Ident(name) if name.starts_with("--"))
            && parser.try_parse(|parser| parser.expect_colon()).is_ok();
        let mut token = first;
        loop {
            match token {
                Token::CurlyBracketBlock => {
                    consume_nested_block(&mut parser)?;
                    if !custom_property {
                        break;
                    }
                }
                Token::Function(_) | Token::ParenthesisBlock | Token::SquareBracketBlock => {
                    consume_nested_block(&mut parser)?;
                }
                Token::Semicolon => break,
                _ => {}
            }
            token = match parser.next_including_whitespace_and_comments() {
                Ok(token) => token.clone(),
                Err(_) => break,
            };
        }
        items.push(trim_css_whitespace(parser.slice_from(start)));
    }
    Ok(items)
}

fn flush_declarations(parts: &mut Vec<(ParsedRule, Option<&str>)>, declarations: &mut Vec<&str>) {
    if declarations.is_empty() {
        return;
    }
    let mut rule = empty_rule("nested-declarations");
    rule.declarations = declarations.join(" ");
    declarations.clear();
    parts.push((rule, None));
}
