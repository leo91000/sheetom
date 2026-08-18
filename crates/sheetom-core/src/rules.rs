use crate::{
    function_rule::{parse_function_prelude, ParsedFunctionParameter, ParsedFunctionPrelude},
    observable::serialize_observable_color,
    scan_safety_metrics,
    syntax::{parse_declaration_list, serialize_identifier, split_top_level_whitespace},
    EngineError, ResourceLimits,
};
use cssparser::{serialize_string, Parser, ParserInput, SourcePosition, Token, TokenizerWithSpans};
use lightningcss::{
    rules::{
        font_palette_values::FontPaletteValuesProperty, view_transition::ViewTransitionProperty,
        CssRule, CssRuleList,
    },
    stylesheet::{ParserOptions, PrinterOptions, StyleSheet},
    traits::ToCss,
    values::ident::NoneOrCustomIdentList,
};
use serde::Serialize;
use std::{
    cell::Cell,
    collections::HashMap,
    ops::Range,
    panic::{catch_unwind, AssertUnwindSafe},
};

thread_local! {
    static ACTIVE_RESOURCE_LIMITS: Cell<ResourceLimits> = Cell::new(ResourceLimits::default());
}

const LARGE_STACK_DEPTH_THRESHOLD: usize = 256;

fn current_resource_limits() -> ResourceLimits {
    ACTIVE_RESOURCE_LIMITS.with(Cell::get)
}

fn with_resource_limits<T>(limits: ResourceLimits, operation: impl FnOnce() -> T) -> T {
    ACTIVE_RESOURCE_LIMITS.with(|active| {
        let previous = active.replace(limits);
        struct Reset<'a> {
            active: &'a Cell<ResourceLimits>,
            previous: ResourceLimits,
        }
        impl Drop for Reset<'_> {
            fn drop(&mut self) {
                self.active.set(self.previous);
            }
        }
        let _reset = Reset { active, previous };
        operation()
    })
}

fn run_parser_operation<T>(
    source: &str,
    operation: impl FnOnce() -> Result<T, EngineError>,
) -> Result<T, EngineError> {
    validate_stylesheet_budget(source)?;
    run_caught(operation)
}

fn run_caught<T>(operation: impl FnOnce() -> Result<T, EngineError>) -> Result<T, EngineError> {
    catch_unwind(AssertUnwindSafe(operation)).unwrap_or(Err(EngineError::UnexpectedPanic))
}

/// An owned, parser-independent description of one CSS rule.
///
/// This is the only rule representation allowed to cross Node-API. In
/// particular, Lightning CSS AST nodes always remain inside Rust.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ParsedRule {
    pub kind: String,
    pub prelude: String,
    pub declarations: String,
    pub children: Vec<ParsedRule>,
    pub css_text: String,
}

impl Clone for ParsedRule {
    fn clone(&self) -> Self {
        let mut nodes = Vec::<Option<ParsedRule>>::new();
        let mut parents = Vec::<Option<usize>>::new();
        let mut pending = vec![(self, None)];
        while let Some((rule, parent)) = pending.pop() {
            let index = nodes.len();
            nodes.push(Some(ParsedRule {
                kind: rule.kind.clone(),
                prelude: rule.prelude.clone(),
                declarations: rule.declarations.clone(),
                children: Vec::with_capacity(rule.children.len()),
                css_text: rule.css_text.clone(),
            }));
            parents.push(parent);
            for child in rule.children.iter().rev() {
                pending.push((child, Some(index)));
            }
        }

        for index in (0..nodes.len()).rev() {
            let Some(mut rule) = nodes[index].take() else {
                continue;
            };
            rule.children.reverse();
            let Some(parent) = parents[index] else {
                return rule;
            };
            if let Some(parent_rule) = nodes.get_mut(parent).and_then(Option::as_mut) {
                parent_rule.children.push(rule);
            }
        }
        ParsedRule {
            kind: self.kind.clone(),
            prelude: self.prelude.clone(),
            declarations: self.declarations.clone(),
            children: Vec::new(),
            css_text: self.css_text.clone(),
        }
    }
}

impl PartialEq for ParsedRule {
    fn eq(&self, other: &Self) -> bool {
        let mut pending = vec![(self, other)];
        while let Some((left, right)) = pending.pop() {
            if left.kind != right.kind
                || left.prelude != right.prelude
                || left.declarations != right.declarations
                || left.css_text != right.css_text
                || left.children.len() != right.children.len()
            {
                return false;
            }
            pending.extend(left.children.iter().zip(&right.children));
        }
        true
    }
}

enum RuleJsonWork<'a> {
    Rule(&'a ParsedRule),
    RuleSuffix(&'a ParsedRule),
    Rules {
        rules: &'a [ParsedRule],
        index: usize,
    },
}

/// Serializes one owned rule DTO without recursing through its child tree.
pub fn serialize_parsed_rule_json(rule: &ParsedRule) -> Result<String, EngineError> {
    serialize_parsed_rule_forest_json(std::slice::from_ref(rule), false)
}

/// Serializes an owned rule forest without consuming native call-stack depth.
pub fn serialize_parsed_rules_json(rules: &[ParsedRule]) -> Result<String, EngineError> {
    serialize_parsed_rule_forest_json(rules, true)
}

fn serialize_parsed_rule_forest_json(
    rules: &[ParsedRule],
    include_root_array: bool,
) -> Result<String, EngineError> {
    let mut output = String::new();
    let mut pending = if include_root_array {
        vec![RuleJsonWork::Rules { rules, index: 0 }]
    } else {
        let Some(rule) = rules.first() else {
            return Err(EngineError::Serialize(
                "a single rule JSON payload requires one rule".to_owned(),
            ));
        };
        vec![RuleJsonWork::Rule(rule)]
    };

    while let Some(work) = pending.pop() {
        match work {
            RuleJsonWork::Rule(rule) => {
                output.push_str("{\"kind\":");
                push_json_string(&mut output, &rule.kind)?;
                output.push_str(",\"prelude\":");
                push_json_string(&mut output, &rule.prelude)?;
                output.push_str(",\"declarations\":");
                push_json_string(&mut output, &rule.declarations)?;
                output.push_str(",\"children\":");
                pending.push(RuleJsonWork::RuleSuffix(rule));
                pending.push(RuleJsonWork::Rules {
                    rules: &rule.children,
                    index: 0,
                });
            }
            RuleJsonWork::RuleSuffix(rule) => {
                output.push_str(",\"cssText\":");
                push_json_string(&mut output, &rule.css_text)?;
                output.push('}');
            }
            RuleJsonWork::Rules { rules, index } => {
                if index == 0 {
                    output.push('[');
                }
                if index == rules.len() {
                    output.push(']');
                    continue;
                }
                if index > 0 {
                    output.push(',');
                }
                pending.push(RuleJsonWork::Rules {
                    rules,
                    index: index + 1,
                });
                pending.push(RuleJsonWork::Rule(&rules[index]));
            }
        }
    }
    Ok(output)
}

fn push_json_string(output: &mut String, value: &str) -> Result<(), EngineError> {
    let encoded =
        serde_json::to_string(value).map_err(|error| EngineError::Serialize(error.to_string()))?;
    output.push_str(&encoded);
    Ok(())
}

impl Drop for ParsedRule {
    fn drop(&mut self) {
        let mut pending = std::mem::take(&mut self.children);
        while let Some(mut descendant) = pending.pop() {
            pending.append(&mut descendant.children);
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ParsedContainerPrelude {
    pub condition_text: String,
    pub name: String,
    pub query: String,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ParsedScopePrelude {
    pub start: Option<String>,
    pub end: Option<String>,
}

pub fn normalize_selector_text(source: &str) -> Result<String, EngineError> {
    normalize_selector_text_with_limits(source, ResourceLimits::default())
}

pub fn normalize_selector_text_with_limits(
    source: &str,
    limits: ResourceLimits,
) -> Result<String, EngineError> {
    with_resource_limits(limits, || normalize_selector_text_active(source))
}

fn normalize_selector_text_active(source: &str) -> Result<String, EngineError> {
    let rule_source = format!("{source}{{}} ");
    with_internal_wrapper_budget(source, &rule_source, || {
        run_parser_operation(&rule_source, || {
            let sheet = StyleSheet::parse(&rule_source, ParserOptions::default())
                .map_err(|error| EngineError::Parse(error.to_string()))?;
            let Some(CssRule::Style(rule)) = sheet.rules.0.first() else {
                return Err(EngineError::Parse("invalid selector list".to_owned()));
            };
            rule.selectors
                .to_css_string(PrinterOptions::default())
                .map_err(|error| EngineError::Serialize(error.to_string()))
        })
    })
}

pub fn normalize_media_text(source: &str) -> Result<String, EngineError> {
    normalize_media_text_with_limits(source, ResourceLimits::default())
}

pub fn normalize_media_text_with_limits(
    source: &str,
    limits: ResourceLimits,
) -> Result<String, EngineError> {
    with_resource_limits(limits, || normalize_media_text_active(source))
}

fn normalize_media_text_active(source: &str) -> Result<String, EngineError> {
    if trim_css_whitespace(source).is_empty() {
        return Ok(String::new());
    }
    let wrapper = format!("@media {source}{{}}");
    let parsed =
        with_internal_wrapper_budget(source, &wrapper, || parse_rule_tree_active(&wrapper))?;
    if parsed.kind != "media" {
        return Err(EngineError::Parse("invalid media query list".to_owned()));
    }
    Ok(format_condition_text(source))
}

pub fn normalize_supports_text(source: &str) -> Result<String, EngineError> {
    normalize_supports_text_with_limits(source, ResourceLimits::default())
}

pub fn normalize_supports_text_with_limits(
    source: &str,
    limits: ResourceLimits,
) -> Result<String, EngineError> {
    with_resource_limits(limits, || normalize_supports_text_active(source))
}

fn normalize_supports_text_active(source: &str) -> Result<String, EngineError> {
    let wrapper = format!("@supports {source}{{}}");
    let mut parsed =
        with_internal_wrapper_budget(source, &wrapper, || parse_rule_tree_active(&wrapper))?;
    if parsed.kind != "supports" {
        return Err(EngineError::Parse("invalid supports condition".to_owned()));
    }
    Ok(std::mem::take(&mut parsed.prelude))
}

pub fn parse_container_prelude(source: &str) -> Result<ParsedContainerPrelude, EngineError> {
    parse_container_prelude_with_limits(source, ResourceLimits::default())
}

pub fn parse_container_prelude_with_limits(
    source: &str,
    limits: ResourceLimits,
) -> Result<ParsedContainerPrelude, EngineError> {
    with_resource_limits(limits, || parse_container_prelude_active(source))
}

fn parse_container_prelude_active(source: &str) -> Result<ParsedContainerPrelude, EngineError> {
    let wrapper = format!("@container {source}{{}}");
    let parsed =
        with_internal_wrapper_budget(source, &wrapper, || parse_rule_tree_active(&wrapper))?;
    if parsed.kind != "container" {
        return Err(EngineError::Parse("invalid container query".to_owned()));
    }
    let condition_text = format_condition_text(source);
    let query_start = container_query_start(&condition_text);
    let (name, query) = condition_text.split_at(query_start);
    let name = trim_css_whitespace(name).to_owned();
    let query = trim_css_whitespace(query).to_owned();
    if query.is_empty() {
        return Err(EngineError::Parse("container query is empty".to_owned()));
    }
    Ok(ParsedContainerPrelude {
        condition_text,
        name,
        query,
    })
}

pub fn parse_scope_prelude(source: &str) -> Result<ParsedScopePrelude, EngineError> {
    parse_scope_prelude_with_limits(source, ResourceLimits::default())
}

pub fn parse_scope_prelude_with_limits(
    source: &str,
    limits: ResourceLimits,
) -> Result<ParsedScopePrelude, EngineError> {
    with_resource_limits(limits, || parse_scope_prelude_active(source))
}

fn parse_scope_prelude_active(source: &str) -> Result<ParsedScopePrelude, EngineError> {
    let wrapper = format!("@scope {source}{{}}");
    let parsed =
        with_internal_wrapper_budget(source, &wrapper, || parse_rule_tree_active(&wrapper))?;
    if parsed.kind != "scope" {
        return Err(EngineError::Parse("invalid scope prelude".to_owned()));
    }
    let text = trim_css_whitespace(source);
    if text.is_empty() {
        return Ok(ParsedScopePrelude {
            start: None,
            end: None,
        });
    }
    let Some((start, remainder)) = consume_parenthesized(text) else {
        return Err(EngineError::Parse("invalid scope start".to_owned()));
    };
    let remainder = trim_css_whitespace(remainder);
    if remainder.is_empty() {
        return Ok(ParsedScopePrelude {
            start: Some(normalize_selector_text_active(start)?),
            end: None,
        });
    }
    let Some(after_to) = remainder.strip_prefix("to") else {
        return Err(EngineError::Parse("invalid scope boundary".to_owned()));
    };
    let Some((end, trailing)) = consume_parenthesized(trim_css_whitespace(after_to)) else {
        return Err(EngineError::Parse("invalid scope end".to_owned()));
    };
    if !trim_css_whitespace(trailing).is_empty() {
        return Err(EngineError::Parse(
            "invalid scope trailing input".to_owned(),
        ));
    }
    Ok(ParsedScopePrelude {
        start: Some(normalize_selector_text_active(start)?),
        end: Some(normalize_selector_text_active(end)?),
    })
}

pub fn serialize_font_family_setter(value: &str) -> String {
    value
        .split(',')
        .map(trim_css_whitespace)
        .filter(|family| !family.is_empty())
        .map(|family| {
            if valid_unquoted_font_family(family) {
                return serialize_identifier(family);
            }
            let mut serialized = String::new();
            if serialize_string(family, &mut serialized).is_err() {
                return "\"\"".to_owned();
            }
            serialized
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn valid_unquoted_font_family(value: &str) -> bool {
    if value.starts_with("--")
        || matches!(
            value.to_ascii_lowercase().as_str(),
            "initial"
                | "inherit"
                | "unset"
                | "default"
                | "revert"
                | "revert-layer"
                | "revert-rule"
                | "serif"
                | "sans-serif"
                | "cursive"
                | "fantasy"
                | "monospace"
                | "system-ui"
                | "emoji"
                | "math"
                | "fangsong"
                | "ui-serif"
                | "ui-sans-serif"
                | "ui-monospace"
                | "ui-rounded"
        )
    {
        return false;
    }
    let mut input = ParserInput::new(value);
    let mut parser = Parser::new(&mut input);
    matches!(parser.next().ok(), Some(Token::Ident(_))) && parser.is_exhausted()
}

fn container_query_start(source: &str) -> usize {
    let lower = source.to_ascii_lowercase();
    if source.starts_with('(') || lower.starts_with("style(") || lower.starts_with("scroll-state(")
    {
        return 0;
    }
    source.find(is_css_whitespace).unwrap_or(source.len())
}

fn consume_parenthesized(source: &str) -> Option<(&str, &str)> {
    if !source.starts_with('(') {
        return None;
    }
    let mut depth = 0usize;
    let mut quote = None;
    let mut escaped = false;
    for (index, character) in source.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        if character == '\\' {
            escaped = true;
            continue;
        }
        if let Some(active_quote) = quote {
            if character == active_quote {
                quote = None;
            }
            continue;
        }
        if matches!(character, '\'' | '"') {
            quote = Some(character);
            continue;
        }
        match character {
            '(' => depth += 1,
            ')' => {
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    return Some((&source[1..index], &source[index + 1..]));
                }
            }
            _ => {}
        }
    }
    None
}

fn format_condition_text(source: &str) -> String {
    let mut output = String::with_capacity(source.len());
    let mut quote = None;
    let mut escaped = false;
    let mut comment = false;
    let mut pending_space = false;
    let characters = source.chars().collect::<Vec<_>>();
    let mut index = 0usize;
    while index < characters.len() {
        let character = characters[index];
        let next = characters.get(index + 1).copied();
        if comment {
            if character == '*' && next == Some('/') {
                comment = false;
                pending_space = true;
                index += 2;
                continue;
            }
            index += 1;
            continue;
        }
        if escaped {
            output.push(character);
            escaped = false;
            index += 1;
            continue;
        }
        if let Some(active_quote) = quote {
            output.push(character);
            if character == '\\' {
                escaped = true;
            } else if character == active_quote {
                quote = None;
            }
            index += 1;
            continue;
        }
        if character == '/' && next == Some('*') {
            comment = true;
            index += 2;
            continue;
        }
        if matches!(character, '\'' | '"') {
            push_pending_space(&mut output, &mut pending_space);
            quote = Some(character);
            output.push(character);
            index += 1;
            continue;
        }
        if is_css_whitespace(character) {
            pending_space = true;
            index += 1;
            continue;
        }
        match character {
            '(' | '[' => {
                push_pending_space(&mut output, &mut pending_space);
                output.push(character);
                pending_space = false;
            }
            ')' | ']' | ',' => {
                trim_trailing_space(&mut output);
                output.push(character);
                pending_space = character == ',';
            }
            ':' => {
                trim_trailing_space(&mut output);
                output.push(':');
                pending_space = true;
            }
            '<' | '>' | '=' => {
                trim_trailing_space(&mut output);
                if !output.is_empty() {
                    output.push(' ');
                }
                output.push(character);
                if matches!(next, Some('=')) && character != '=' {
                    output.push('=');
                    index += 1;
                }
                pending_space = true;
            }
            _ => {
                push_pending_space(&mut output, &mut pending_space);
                output.push(character);
            }
        }
        index += 1;
    }
    trim_css_whitespace(&output).to_owned()
}

fn push_pending_space(output: &mut String, pending: &mut bool) {
    if *pending && !output.is_empty() && !output.ends_with(['(', '[', ' ']) {
        output.push(' ');
    }
    *pending = false;
}

fn trim_trailing_space(output: &mut String) {
    while output.ends_with(' ') {
        output.pop();
    }
}

fn trim_css_whitespace(value: &str) -> &str {
    value.trim_matches(is_css_whitespace)
}

fn trim_start_css_whitespace(value: &str) -> &str {
    value.trim_start_matches(is_css_whitespace)
}

fn is_css_whitespace(character: char) -> bool {
    matches!(character, ' ' | '\t' | '\n' | '\r' | '\u{000c}')
}

fn is_css_whitespace_byte(byte: u8) -> bool {
    matches!(byte, b' ' | b'\t' | b'\n' | b'\r' | b'\x0c')
}

/// Consumes top-level CSS Syntax rules while preserving their exact source.
pub fn scan_top_level_rules(source: &str) -> Result<Vec<String>, EngineError> {
    scan_top_level_rules_with_limits(source, ResourceLimits::default())
}

pub fn scan_top_level_rules_with_limits(
    source: &str,
    limits: ResourceLimits,
) -> Result<Vec<String>, EngineError> {
    with_resource_limits(limits, || scan_top_level_rules_active(source))
}

fn scan_top_level_rules_active(source: &str) -> Result<Vec<String>, EngineError> {
    run_parser_operation(source, || scan_top_level_rules_inner(source))
}

fn scan_top_level_rules_inner(source: &str) -> Result<Vec<String>, EngineError> {
    validate_stylesheet_budget(source)?;
    let mut input = ParserInput::new(source);
    let mut parser = Parser::new(&mut input);
    let mut output = Vec::new();

    while !parser.is_exhausted() {
        let start = parser.position();
        let first = match parser.next_including_whitespace_and_comments() {
            Ok(token) => token.clone(),
            Err(_) => break,
        };
        if is_rule_trivia(&first) {
            continue;
        }

        let mut token = first;
        loop {
            let boundary = match token {
                Token::CurlyBracketBlock => {
                    consume_nested_block(&mut parser)?;
                    true
                }
                Token::Function(_) | Token::ParenthesisBlock | Token::SquareBracketBlock => {
                    consume_nested_block(&mut parser)?;
                    false
                }
                Token::Semicolon => true,
                _ => false,
            };
            if boundary {
                break;
            }

            token = match parser.next_including_whitespace_and_comments() {
                Ok(token) => token.clone(),
                Err(_) => break,
            };
        }

        push_source_slice(&parser, start, &mut output);
        let limits = current_resource_limits();
        if output.len() > limits.max_rules {
            return Err(EngineError::RuleLimitExceeded {
                actual: output.len(),
                limit: limits.max_rules,
            });
        }
    }

    Ok(output)
}

fn validate_stylesheet_budget(source: &str) -> Result<(), EngineError> {
    let limits = current_resource_limits();
    if source.len() > limits.max_stylesheet_bytes {
        return Err(EngineError::InputLimitExceeded {
            actual: source.len(),
            limit: limits.max_stylesheet_bytes,
        });
    }
    let metrics = scan_safety_metrics(source);
    if metrics.maximum_depth > limits.max_nesting_depth {
        return Err(EngineError::NestingLimitExceeded {
            actual: metrics.maximum_depth,
            limit: limits.max_nesting_depth,
        });
    }
    Ok(())
}

fn with_internal_wrapper_budget<T>(
    source: &str,
    wrapper: &str,
    operation: impl FnOnce() -> Result<T, EngineError>,
) -> Result<T, EngineError> {
    validate_stylesheet_budget(source)?;
    let limits = current_resource_limits();
    let wrapper_depth = scan_safety_metrics(wrapper).maximum_depth;
    let internal_limits = ResourceLimits {
        max_stylesheet_bytes: limits.max_stylesheet_bytes.max(wrapper.len()),
        max_nesting_depth: limits.max_nesting_depth.max(wrapper_depth),
        ..limits
    };
    with_resource_limits(internal_limits, operation)
}

fn is_rule_trivia(token: &Token<'_>) -> bool {
    matches!(
        token,
        Token::WhiteSpace(_)
            | Token::Comment(_)
            | Token::CDO
            | Token::CDC
            | Token::Semicolon
            | Token::CloseCurlyBracket
    )
}

fn consume_nested_block<'i>(parser: &mut Parser<'i, '_>) -> Result<(), EngineError> {
    parser
        .parse_nested_block(|nested| {
            while nested.next_including_whitespace_and_comments().is_ok() {}
            Ok::<(), cssparser::ParseError<'i, ()>>(())
        })
        .map_err(|error| EngineError::Parse(format!("{:?}", error.kind)))
}

fn push_source_slice(parser: &Parser<'_, '_>, start: SourcePosition, output: &mut Vec<String>) {
    let source = trim_css_whitespace(parser.slice(start..parser.position()));
    if !source.is_empty() {
        output.push(source.to_owned());
    }
}

fn split_outer_block(source: &str) -> Option<(&str, &str)> {
    let mut input = ParserInput::new(source);
    let mut parser = Parser::new(&mut input);
    let start = parser.position();
    loop {
        let token_start = parser.position();
        let token = parser
            .next_including_whitespace_and_comments()
            .ok()?
            .clone();
        match token {
            Token::CurlyBracketBlock => {
                let header = trim_css_whitespace(parser.slice(start..token_start));
                let body_start = parser.position();
                consume_nested_block(&mut parser).ok()?;
                let mut body = parser.slice(body_start..parser.position());
                if let Some(without_close) = body.strip_suffix('}') {
                    body = without_close;
                }
                while let Ok(token) = parser.next_including_whitespace_and_comments() {
                    if !matches!(token, Token::WhiteSpace(_) | Token::Comment(_)) {
                        return None;
                    }
                }
                return Some((header, body));
            }
            Token::Function(_) | Token::ParenthesisBlock | Token::SquareBracketBlock => {
                consume_nested_block(&mut parser).ok()?;
            }
            Token::Semicolon => return None,
            _ => {}
        }
    }
}

pub fn parse_rule_tree(source: &str) -> Result<ParsedRule, EngineError> {
    parse_rule_tree_with_limits(source, ResourceLimits::default())
}

pub fn parse_rule_tree_with_limits(
    source: &str,
    limits: ResourceLimits,
) -> Result<ParsedRule, EngineError> {
    with_resource_limits(limits, || parse_rule_tree_active(source))
}

fn parse_rule_tree_active(source: &str) -> Result<ParsedRule, EngineError> {
    let mut rules = parse_stylesheet_tree_active(source, false)?;
    if rules.len() != 1 {
        return Err(EngineError::Parse(
            "a rule mutation must contain exactly one rule".to_owned(),
        ));
    }
    rules
        .pop()
        .ok_or_else(|| EngineError::Parse("the rule is empty".to_owned()))
}

/// Parses one rule with browser-style declaration recovery.
pub fn parse_recovered_rule_tree(source: &str) -> Result<ParsedRule, EngineError> {
    parse_recovered_rule_tree_with_limits(source, ResourceLimits::default())
}

pub fn parse_recovered_rule_tree_with_limits(
    source: &str,
    limits: ResourceLimits,
) -> Result<ParsedRule, EngineError> {
    with_resource_limits(limits, || parse_recovered_rule_tree_active(source))
}

fn parse_recovered_rule_tree_active(source: &str) -> Result<ParsedRule, EngineError> {
    run_parser_operation(source, || {
        let parsed = parse_recovered_rule_tree_inner(source, 0)?;
        let count = parsed_rule_node_count(&parsed);
        let limits = current_resource_limits();
        if count > limits.max_rules {
            return Err(EngineError::RuleLimitExceeded {
                actual: count,
                limit: limits.max_rules,
            });
        }
        Ok(parsed)
    })
}

/// Parses exactly one outer rule while allowing browser-style recovery inside
/// its block. Unlike full stylesheet error recovery, trailing invalid tokens
/// cannot disappear and make an `insertRule()` mutation look successful.
pub fn parse_recovered_single_rule_tree(source: &str) -> Result<ParsedRule, EngineError> {
    parse_recovered_single_rule_tree_with_limits(source, ResourceLimits::default())
}

pub fn parse_recovered_single_rule_tree_with_limits(
    source: &str,
    limits: ResourceLimits,
) -> Result<ParsedRule, EngineError> {
    with_resource_limits(limits, || parse_recovered_single_rule_tree_active(source))
}

fn parse_recovered_single_rule_tree_active(source: &str) -> Result<ParsedRule, EngineError> {
    run_parser_operation(source, || {
        if !contains_exactly_one_rule(source)? {
            return Err(EngineError::Parse(
                "a recovered rule mutation must contain exactly one rule".to_owned(),
            ));
        }
        let mut rules = parse_stylesheet_tree_inner(source, true)?;
        if rules.len() != 1 {
            return Err(EngineError::Parse(
                "a recovered rule mutation must produce exactly one rule".to_owned(),
            ));
        }
        rules
            .pop()
            .ok_or_else(|| EngineError::Parse("the recovered rule is empty".to_owned()))
    })
}

fn contains_exactly_one_rule(source: &str) -> Result<bool, EngineError> {
    let mut input = ParserInput::new(source);
    let mut parser = Parser::new(&mut input);
    let mut token = loop {
        let Ok(token) = parser.next_including_whitespace_and_comments() else {
            return Ok(false);
        };
        if matches!(token, Token::WhiteSpace(_) | Token::Comment(_)) {
            continue;
        }
        break token.clone();
    };

    loop {
        match token {
            Token::CurlyBracketBlock => {
                consume_nested_block(&mut parser)?;
                break;
            }
            Token::Function(_) | Token::ParenthesisBlock | Token::SquareBracketBlock => {
                consume_nested_block(&mut parser)?;
            }
            Token::Semicolon => break,
            Token::CloseCurlyBracket | Token::CDO | Token::CDC => return Ok(false),
            _ => {}
        }
        token = match parser.next_including_whitespace_and_comments() {
            Ok(token) => token.clone(),
            Err(_) => return Ok(false),
        };
    }

    while let Ok(token) = parser.next_including_whitespace_and_comments() {
        if !matches!(token, Token::WhiteSpace(_) | Token::Comment(_)) {
            return Ok(false);
        }
    }
    Ok(true)
}

fn parse_recovered_rule_tree_inner(source: &str, depth: usize) -> Result<ParsedRule, EngineError> {
    let limits = current_resource_limits();
    if depth > limits.max_nesting_depth {
        return Err(EngineError::NestingLimitExceeded {
            actual: depth,
            limit: limits.max_nesting_depth,
        });
    }
    if scan_safety_metrics(source).maximum_depth > LARGE_STACK_DEPTH_THRESHOLD {
        if let Some(parsed) = recover_function_tree_iterative(source, depth) {
            return parsed;
        }
        if let Some(parsed) = recover_single_child_rule_chain_from_source(source, depth) {
            return parsed;
        }
        if let Some(parsed) = recover_grouping_tree_iterative(source, depth) {
            return parsed;
        }
    }
    parse_recovered_rule_tree_noniterative(source, depth)
}

fn recover_single_child_rule_chain_from_source(
    source: &str,
    depth: usize,
) -> Option<Result<ParsedRule, EngineError>> {
    let limits = current_resource_limits();
    let mut current_source = trim_css_whitespace(source);
    let mut current_depth = depth;
    let mut ancestors = Vec::<ParsedRule>::new();

    loop {
        if current_depth > limits.max_nesting_depth {
            return Some(Err(EngineError::NestingLimitExceeded {
                actual: current_depth,
                limit: limits.max_nesting_depth,
            }));
        }
        let (header, body) = split_single_rule_block(current_source)?;
        if is_function_rule_header(header) {
            return None;
        }
        let probe_source = format!("{header}{{}}");
        let mut probe = parse_rule_tree_active(&probe_source).ok()?;
        if probe.kind != "style" && !is_grouping_rule(&probe) {
            return None;
        }
        preserve_source_prelude(&mut probe, header);
        probe.css_text.clear();
        probe.declarations.clear();
        probe.children.clear();

        let child_source = trim_css_whitespace(body);
        let Some((child_header, _)) = split_single_rule_block(child_source) else {
            if contains_curly_block(child_source) {
                return None;
            }
            let mut parsed = match recover_block_rule(probe, body, current_depth) {
                Ok(parsed) => parsed,
                Err(error) => return Some(Err(error)),
            };
            while let Some(mut ancestor) = ancestors.pop() {
                ancestor.children.push(parsed);
                parsed = ancestor;
            }
            return Some(Ok(parsed));
        };
        let child_probe_source = format!("{child_header}{{}}");
        let Ok(child_probe) = parse_rule_tree_active(&child_probe_source) else {
            return None;
        };
        if child_probe.kind != "style" && !is_grouping_rule(&child_probe) {
            return None;
        }
        ancestors.push(probe);
        current_source = child_source;
        current_depth = current_depth.saturating_add(1);
    }
}

fn contains_curly_block(source: &str) -> bool {
    let mut tokenizer = TokenizerWithSpans::new(source);
    while let Ok(token) = tokenizer.next_token() {
        if matches!(token.token, Token::CurlyBracketBlock) {
            return true;
        }
    }
    false
}

enum FunctionBodyPart {
    Rule(ParsedRule),
    Nested,
}

enum FunctionTreeWork {
    ParseBody {
        wrapper: ParsedRule,
        body: String,
        depth: usize,
    },
    Assemble {
        wrapper: ParsedRule,
        parts: Vec<FunctionBodyPart>,
        child_count: usize,
    },
}

fn recover_function_tree_iterative(
    source: &str,
    depth: usize,
) -> Option<Result<ParsedRule, EngineError>> {
    let (header, body) = split_outer_block(source)?;
    if !is_function_rule_header(header) {
        return None;
    }
    let prelude = match parse_function_prelude(header) {
        Some(prelude) => prelude,
        None => {
            return Some(Err(EngineError::Parse(
                "invalid @function prelude".to_owned(),
            )))
        }
    };
    let wrapper = ParsedRule {
        kind: "function".to_owned(),
        prelude: prelude.name,
        declarations: prelude.return_type,
        children: prelude
            .parameters
            .iter()
            .map(function_parameter_metadata)
            .collect(),
        css_text: String::new(),
    };
    let mut pending = vec![FunctionTreeWork::ParseBody {
        wrapper,
        body: body.to_owned(),
        depth: depth.saturating_add(1),
    }];
    let mut completed = Vec::<ParsedRule>::new();
    let limits = current_resource_limits();

    while let Some(work) = pending.pop() {
        match work {
            FunctionTreeWork::ParseBody {
                mut wrapper,
                body,
                depth,
            } => {
                if depth > limits.max_nesting_depth {
                    return Some(Err(EngineError::NestingLimitExceeded {
                        actual: depth,
                        limit: limits.max_nesting_depth,
                    }));
                }
                let items = match scan_function_block_items(&body) {
                    Ok(items) => items,
                    Err(error) => return Some(Err(error)),
                };
                let mut parts = std::mem::take(&mut wrapper.children)
                    .into_iter()
                    .map(FunctionBodyPart::Rule)
                    .collect::<Vec<_>>();
                let mut declarations = Vec::<String>::new();
                let mut nested = Vec::<(ParsedRule, String)>::new();
                for item in items {
                    if let Some((conditional, conditional_body)) =
                        prepare_function_conditional(&item)
                    {
                        push_function_declaration_part(&mut parts, &mut declarations);
                        parts.push(FunctionBodyPart::Nested);
                        nested.push((conditional, conditional_body));
                    } else if !is_at_rule_source(&item) {
                        declarations.push(item);
                    }
                }
                push_function_declaration_part(&mut parts, &mut declarations);

                if nested.is_empty() {
                    wrapper.children = parts
                        .into_iter()
                        .filter_map(|part| match part {
                            FunctionBodyPart::Rule(rule) => Some(rule),
                            FunctionBodyPart::Nested => None,
                        })
                        .collect();
                    completed.push(wrapper);
                    continue;
                }

                pending.push(FunctionTreeWork::Assemble {
                    wrapper,
                    parts,
                    child_count: nested.len(),
                });
                for (conditional, conditional_body) in nested.into_iter().rev() {
                    pending.push(FunctionTreeWork::ParseBody {
                        wrapper: conditional,
                        body: conditional_body,
                        depth: depth.saturating_add(1),
                    });
                }
            }
            FunctionTreeWork::Assemble {
                mut wrapper,
                parts,
                child_count,
            } => {
                if completed.len() < child_count {
                    return Some(Err(EngineError::UnexpectedPanic));
                }
                let first_child = completed.len() - child_count;
                let drained = completed.drain(first_child..).collect::<Vec<_>>();
                let mut nested = drained.into_iter();
                let mut children = Vec::with_capacity(parts.len());
                for part in parts {
                    match part {
                        FunctionBodyPart::Rule(rule) => children.push(rule),
                        FunctionBodyPart::Nested => {
                            let Some(rule) = nested.next() else {
                                return Some(Err(EngineError::UnexpectedPanic));
                            };
                            children.push(rule);
                        }
                    }
                }
                if nested.next().is_some() {
                    return Some(Err(EngineError::UnexpectedPanic));
                }
                wrapper.children = children;
                completed.push(wrapper);
            }
        }
    }

    if completed.len() != 1 {
        return Some(Err(EngineError::UnexpectedPanic));
    }
    completed.pop().map(Ok)
}

fn prepare_function_conditional(source: &str) -> Option<(ParsedRule, String)> {
    let (header, body) = split_outer_block(source)?;
    let probe_source = format!("{header}{{}}");
    let mut probe = parse_rule_tree_active(&probe_source).ok()?;
    if !matches!(probe.kind.as_str(), "media" | "supports" | "container") {
        return None;
    }
    preserve_source_prelude(&mut probe, header);
    probe.declarations.clear();
    probe.children.clear();
    probe.css_text.clear();
    Some((probe, body.to_owned()))
}

fn push_function_declaration_part(
    parts: &mut Vec<FunctionBodyPart>,
    declarations: &mut Vec<String>,
) {
    if declarations.is_empty() {
        return;
    }
    parts.push(FunctionBodyPart::Rule(ParsedRule {
        kind: "function-declarations".to_owned(),
        prelude: String::new(),
        declarations: declarations.join(" "),
        children: Vec::new(),
        css_text: String::new(),
    }));
    declarations.clear();
}

fn parse_recovered_rule_tree_noniterative(
    source: &str,
    depth: usize,
) -> Result<ParsedRule, EngineError> {
    let function_rule = is_function_rule_header(source);
    let Some((header, body)) = split_outer_block(source) else {
        if function_rule {
            return Err(EngineError::Parse(
                "invalid @function rule block".to_owned(),
            ));
        }
        let strict = parse_rule_tree_active(source).ok();
        return strict
            .map(|mut parsed| {
                preserve_source_text(&mut parsed, source);
                parsed
            })
            .ok_or_else(|| EngineError::Parse("the rule has no recoverable block".to_owned()));
    };
    if function_rule {
        let prelude = parse_function_prelude(header)
            .ok_or_else(|| EngineError::Parse("invalid @function prelude".to_owned()))?;
        return parse_function_rule(prelude, body, depth);
    }
    let strict = parse_rule_tree_active(source).ok();
    if let Some(parsed) = strict.as_ref() {
        if parsed.kind == "property" {
            let mut parsed = parsed.clone();
            preserve_property_descriptor_values(&mut parsed, body);
            return Ok(parsed);
        }
        if let Some(recovered) = recover_single_child_group_chain(parsed.clone(), source, depth) {
            return recovered;
        }
        let raw_items = scan_recovered_block_items(body).len();
        let recover_from_source = matches!(
            parsed.kind.as_str(),
            "style"
                | "font-face"
                | "position-try"
                | "counter-style"
                | "font-feature-values"
                | "page"
                | "media"
                | "supports"
                | "container"
                | "layer-block"
                | "scope"
                | "starting-style"
                | "keyframes"
        );
        let child_count_matches = !matches!(
            parsed.kind.as_str(),
            "media"
                | "supports"
                | "container"
                | "layer-block"
                | "scope"
                | "starting-style"
                | "keyframes"
        ) || parsed.children.len() == raw_items;
        if !recover_from_source && child_count_matches {
            let mut parsed = parsed.clone();
            preserve_source_prelude(&mut parsed, header);
            preserve_source_text(&mut parsed, source);
            return Ok(parsed);
        }
    }

    let probe_source = format!("{header}{{}}");
    let mut probe = parse_rule_tree_active(&probe_source)?;
    preserve_source_prelude(&mut probe, header);
    recover_block_rule(probe, body, depth)
}

enum GroupingTreeWork {
    Parse {
        range: Range<usize>,
        depth: usize,
    },
    Assemble {
        probe: ParsedRule,
        child_count: usize,
    },
}

fn recover_grouping_tree_iterative(
    source: &str,
    depth: usize,
) -> Option<Result<ParsedRule, EngineError>> {
    let limits = current_resource_limits();
    let curly_closes = matching_curly_closes(source);
    let mut pending = vec![GroupingTreeWork::Parse {
        range: trim_css_range(source, 0..source.len()),
        depth,
    }];
    let mut completed = Vec::<ParsedRule>::new();

    while let Some(work) = pending.pop() {
        match work {
            GroupingTreeWork::Parse { range, depth } => {
                if depth > limits.max_nesting_depth {
                    return Some(Err(EngineError::NestingLimitExceeded {
                        actual: depth,
                        limit: limits.max_nesting_depth,
                    }));
                }
                let rule_source = source.get(range.clone())?;
                let Some(open) = outer_block_open_index(rule_source) else {
                    let parsed = parse_recovered_rule_tree_noniterative(rule_source, depth).ok()?;
                    completed.push(parsed);
                    continue;
                };
                let absolute_open = range.start.saturating_add(open);
                let close = *curly_closes.get(&absolute_open)?;
                if close > range.end || close <= absolute_open {
                    return None;
                }
                let header = rule_source.get(..open)?;
                let body_range = absolute_open.saturating_add(1)..close.saturating_sub(1);
                let probe_source = format!("{header}{{}}");
                let mut probe = parse_rule_tree_active(&probe_source).ok()?;
                if !is_grouping_rule(&probe) {
                    let parsed = parse_recovered_rule_tree_noniterative(rule_source, depth).ok()?;
                    completed.push(parsed);
                    continue;
                }

                let children = scan_recovered_item_ranges(source, body_range, &curly_closes)?;
                preserve_source_prelude(&mut probe, header);
                probe.css_text.clear();
                probe.declarations.clear();
                probe.children.clear();
                if children.is_empty() {
                    completed.push(probe);
                    continue;
                }

                pending.push(GroupingTreeWork::Assemble {
                    probe,
                    child_count: children.len(),
                });
                for range in children.into_iter().rev() {
                    pending.push(GroupingTreeWork::Parse {
                        range,
                        depth: depth.saturating_add(1),
                    });
                }
            }
            GroupingTreeWork::Assemble {
                mut probe,
                child_count,
            } => {
                if completed.len() < child_count {
                    return Some(Err(EngineError::UnexpectedPanic));
                }
                let first_child = completed.len() - child_count;
                probe.children.extend(completed.drain(first_child..));
                completed.push(probe);
            }
        }
    }

    if completed.len() != 1 {
        return Some(Err(EngineError::UnexpectedPanic));
    }
    completed.pop().map(Ok)
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum LexicalBlock {
    Parenthesis,
    Square,
    Curly(usize),
}

fn matching_curly_closes(source: &str) -> HashMap<usize, usize> {
    let mut tokenizer = TokenizerWithSpans::new(source);
    let mut blocks = Vec::<LexicalBlock>::new();
    let mut closes = HashMap::<usize, usize>::new();
    while let Ok(token) = tokenizer.next_token() {
        match token.token {
            Token::Function(_) | Token::ParenthesisBlock => {
                blocks.push(LexicalBlock::Parenthesis);
            }
            Token::SquareBracketBlock => blocks.push(LexicalBlock::Square),
            Token::CurlyBracketBlock => {
                blocks.push(LexicalBlock::Curly(token.start.byte_index()));
            }
            Token::CloseParenthesis => {
                if blocks.last() == Some(&LexicalBlock::Parenthesis) {
                    blocks.pop();
                }
            }
            Token::CloseSquareBracket => {
                if blocks.last() == Some(&LexicalBlock::Square) {
                    blocks.pop();
                }
            }
            Token::CloseCurlyBracket => {
                if let Some(LexicalBlock::Curly(open)) = blocks.last().copied() {
                    blocks.pop();
                    closes.insert(open, token.end.byte_index());
                }
            }
            _ => {}
        }
    }
    closes
}

fn scan_recovered_item_ranges(
    source: &str,
    range: Range<usize>,
    curly_closes: &HashMap<usize, usize>,
) -> Option<Vec<Range<usize>>> {
    let bytes = source.as_bytes();
    let mut output = Vec::<Range<usize>>::new();
    let mut start = None;
    let mut index = range.start;
    let mut component_depth = 0usize;
    let mut quote = None;
    let mut in_comment = false;

    while index < range.end {
        let byte = *bytes.get(index)?;
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
                index = (index + 2).min(range.end);
                continue;
            }
            if byte == delimiter {
                quote = None;
            }
            index += 1;
            continue;
        }
        if byte == b'/' && bytes.get(index + 1) == Some(&b'*') {
            if start.is_none() {
                start = Some(index);
            }
            in_comment = true;
            index += 2;
            continue;
        }
        if matches!(byte, b'\'' | b'"') {
            if start.is_none() {
                start = Some(index);
            }
            quote = Some(byte);
            index += 1;
            continue;
        }
        if byte == b'\\' {
            if start.is_none() {
                start = Some(index);
            }
            index = (index + 2).min(range.end);
            continue;
        }
        if start.is_none() {
            if is_css_whitespace_byte(byte) || matches!(byte, b';' | b'}') {
                index += 1;
                continue;
            }
            start = Some(index);
        }

        match byte {
            b'(' | b'[' => component_depth = component_depth.saturating_add(1),
            b')' | b']' => component_depth = component_depth.saturating_sub(1),
            b'{' => {
                let close = *curly_closes.get(&index)?;
                if close > range.end {
                    return None;
                }
                if component_depth == 0 {
                    push_recovered_range(source, start.take(), close, &mut output);
                }
                index = close;
                continue;
            }
            b';' if component_depth == 0 => {
                push_recovered_range(source, start.take(), index + 1, &mut output);
            }
            _ => {}
        }
        index += 1;
    }
    push_recovered_range(source, start, range.end, &mut output);
    Some(output)
}

fn push_recovered_range(
    source: &str,
    start: Option<usize>,
    end: usize,
    output: &mut Vec<Range<usize>>,
) {
    let Some(start) = start else {
        return;
    };
    let range = trim_css_range(source, start..end);
    if range.start < range.end {
        output.push(range);
    }
}

fn trim_css_range(source: &str, mut range: Range<usize>) -> Range<usize> {
    let bytes = source.as_bytes();
    while range.start < range.end
        && bytes
            .get(range.start)
            .is_some_and(|byte| is_css_whitespace_byte(*byte))
    {
        range.start += 1;
    }
    while range.start < range.end
        && bytes
            .get(range.end - 1)
            .is_some_and(|byte| is_css_whitespace_byte(*byte))
    {
        range.end -= 1;
    }
    range
}

fn is_grouping_rule(rule: &ParsedRule) -> bool {
    matches!(
        rule.kind.as_str(),
        "media" | "supports" | "container" | "layer-block" | "scope" | "starting-style"
    )
}

/// Recovers a valid, deeply nested grouping-rule chain without reparsing every
/// suffix of the same source. The general recovery path remains responsible
/// for branching or structurally ambiguous blocks; this fast path only applies
/// while the strict parser and the source both describe exactly one child.
fn recover_single_child_group_chain(
    mut parsed: ParsedRule,
    source: &str,
    depth: usize,
) -> Option<Result<ParsedRule, EngineError>> {
    let mut current = &mut parsed;
    let mut current_source = trim_css_whitespace(source);
    let mut current_depth = depth;
    let mut traversed = 0usize;

    while is_single_child_group(current) {
        let (header, body) = split_single_rule_block(current_source)?;
        preserve_source_prelude(current, header);
        current.css_text.clear();
        current.declarations.clear();

        let child_source = trim_css_whitespace(body);
        if child_source.is_empty() {
            return None;
        }
        let child = current.children.first_mut()?;
        current = child;
        current_source = child_source;
        current_depth = current_depth.saturating_add(1);
        traversed = traversed.saturating_add(1);
    }

    if traversed < 2 {
        return None;
    }
    match parse_recovered_rule_tree_inner(current_source, current_depth) {
        Ok(leaf) => {
            *current = leaf;
            Some(Ok(parsed))
        }
        Err(error) => Some(Err(error)),
    }
}

fn is_single_child_group(rule: &ParsedRule) -> bool {
    matches!(
        rule.kind.as_str(),
        "media" | "supports" | "container" | "layer-block" | "scope" | "starting-style"
    ) && rule.children.len() == 1
}

fn split_single_rule_block(source: &str) -> Option<(&str, &str)> {
    let source = trim_css_whitespace(source);
    let index = outer_block_open_index(source)?;
    let body_with_close = source.get(index + 1..)?.strip_suffix('}')?;
    Some((trim_css_whitespace(&source[..index]), body_with_close))
}

fn outer_block_open_index(source: &str) -> Option<usize> {
    let bytes = source.as_bytes();
    let mut index = 0usize;
    let mut quote = None;
    let mut in_comment = false;
    let mut component_depth = 0usize;

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
            index += 1;
            continue;
        }
        if byte == b'\\' {
            index = (index + 2).min(bytes.len());
            continue;
        }
        match byte {
            b'(' | b'[' => component_depth = component_depth.saturating_add(1),
            b')' | b']' => component_depth = component_depth.saturating_sub(1),
            b'{' if component_depth == 0 => return Some(index),
            b';' if component_depth == 0 => return None,
            _ => {}
        }
        index += 1;
    }
    None
}

fn parse_function_rule(
    prelude: ParsedFunctionPrelude,
    body: &str,
    depth: usize,
) -> Result<ParsedRule, EngineError> {
    let mut children = prelude
        .parameters
        .iter()
        .map(function_parameter_metadata)
        .collect::<Vec<_>>();
    children.extend(parse_function_body(body, depth + 1)?);
    Ok(ParsedRule {
        kind: "function".to_owned(),
        prelude: prelude.name,
        declarations: prelude.return_type,
        children,
        css_text: String::new(),
    })
}

fn is_function_rule_header(header: &str) -> bool {
    let mut input = ParserInput::new(header);
    let mut parser = Parser::new(&mut input);
    matches!(
        parser.next().ok(),
        Some(Token::AtKeyword(name)) if name.eq_ignore_ascii_case("function")
    )
}

fn function_parameter_metadata(parameter: &ParsedFunctionParameter) -> ParsedRule {
    let children = parameter
        .default_value
        .as_ref()
        .map(|value| vec![property_descriptor("default-value", value)])
        .unwrap_or_default();
    ParsedRule {
        kind: "function-parameter".to_owned(),
        prelude: parameter.name.clone(),
        declarations: parameter.value_type.clone(),
        children,
        css_text: String::new(),
    }
}

fn parse_function_body(body: &str, depth: usize) -> Result<Vec<ParsedRule>, EngineError> {
    let limits = current_resource_limits();
    if depth > limits.max_nesting_depth {
        return Err(EngineError::NestingLimitExceeded {
            actual: depth,
            limit: limits.max_nesting_depth,
        });
    }
    let mut children = Vec::new();
    let mut declarations = Vec::new();
    for fragment in scan_function_block_items(body)? {
        if let Some(rule) = parse_function_conditional_rule(&fragment, depth)? {
            flush_function_declarations(&mut children, &mut declarations);
            children.push(rule);
        } else if !is_at_rule_source(&fragment) {
            declarations.push(fragment);
        }
    }
    flush_function_declarations(&mut children, &mut declarations);
    Ok(children)
}

fn is_at_rule_source(source: &str) -> bool {
    let mut input = ParserInput::new(source);
    let mut parser = Parser::new(&mut input);
    matches!(parser.next().ok(), Some(Token::AtKeyword(_)))
}

fn parse_function_conditional_rule(
    source: &str,
    depth: usize,
) -> Result<Option<ParsedRule>, EngineError> {
    let Some((header, body)) = split_outer_block(source) else {
        return Ok(None);
    };
    let probe_source = format!("{header}{{}}");
    let Ok(mut probe) = parse_rule_tree_active(&probe_source) else {
        return Ok(None);
    };
    if !matches!(probe.kind.as_str(), "media" | "supports" | "container") {
        return Ok(None);
    }
    preserve_source_prelude(&mut probe, header);
    probe.declarations.clear();
    probe.children = parse_function_body(body, depth + 1)?;
    probe.css_text.clear();
    Ok(Some(probe))
}

fn flush_function_declarations(children: &mut Vec<ParsedRule>, declarations: &mut Vec<String>) {
    if declarations.is_empty() {
        return;
    }
    let source = declarations.join(" ");
    declarations.clear();
    children.push(ParsedRule {
        kind: "function-declarations".to_owned(),
        prelude: String::new(),
        declarations: source,
        children: Vec::new(),
        css_text: String::new(),
    });
}

/// Splits a custom-function body according to the CSS Syntax block structure.
///
/// A function body mixes declaration runs with conditional rules. Semicolons
/// inside component-value blocks (including `if()`, arbitrary functions,
/// square blocks, and curly blocks) belong to the declaration value rather
/// than terminating it. Keeping this scanner function-specific avoids
/// weakening nested-style-rule recovery, where a top-level curly block has a
/// different meaning.
fn scan_function_block_items(source: &str) -> Result<Vec<String>, EngineError> {
    let mut input = ParserInput::new(source);
    let mut parser = Parser::new(&mut input);
    let mut output = Vec::new();

    while !parser.is_exhausted() {
        let start = parser.position();
        let first = match parser.next_including_whitespace_and_comments() {
            Ok(token) => token.clone(),
            Err(_) => break,
        };
        if is_rule_trivia(&first) {
            continue;
        }
        let at_rule = matches!(first, Token::AtKeyword(_));
        let mut token = first;
        loop {
            let boundary = match token {
                Token::CurlyBracketBlock => {
                    consume_nested_block(&mut parser)?;
                    at_rule
                }
                Token::Function(_) | Token::ParenthesisBlock | Token::SquareBracketBlock => {
                    consume_nested_block(&mut parser)?;
                    false
                }
                Token::Semicolon => true,
                _ => false,
            };
            if boundary {
                break;
            }
            token = match parser.next_including_whitespace_and_comments() {
                Ok(token) => token.clone(),
                Err(_) => break,
            };
        }
        push_source_slice(&parser, start, &mut output);
        let limits = current_resource_limits();
        if output.len() > limits.max_rules {
            return Err(EngineError::RuleLimitExceeded {
                actual: output.len(),
                limit: limits.max_rules,
            });
        }
    }
    Ok(output)
}

fn preserve_property_descriptor_values(rule: &mut ParsedRule, body: &str) {
    let declarations = parse_declaration_list(body);
    let last_value = |name: &str| {
        declarations
            .iter()
            .rev()
            .find(|declaration| declaration.name.eq_ignore_ascii_case(name))
            .map(|declaration| trim_css_whitespace(declaration.value))
    };
    let Some(syntax) = last_value("syntax") else {
        return;
    };
    let Some(inherits) = last_value("inherits") else {
        return;
    };
    let mut descriptors = vec![
        format!("syntax: {syntax};"),
        format!("inherits: {};", inherits.to_ascii_lowercase()),
    ];
    if let Some(initial_value) = last_value("initial-value") {
        descriptors.push(format!("initial-value: {initial_value};"));
    }
    let syntax_value = parse_css_string(syntax).unwrap_or_default();
    rule.children = vec![
        property_descriptor("syntax", &syntax_value),
        property_descriptor("inherits", &inherits.to_ascii_lowercase()),
    ];
    if let Some(initial_value) = last_value("initial-value") {
        rule.children
            .push(property_descriptor("initial-value", initial_value));
    }
    rule.declarations = descriptors.join(" ");
    rule.css_text = format!("@property {} {{ {} }}", rule.prelude, descriptors.join(" "));
}

fn property_descriptor(name: &str, value: &str) -> ParsedRule {
    ParsedRule {
        kind: "property-descriptor".to_owned(),
        prelude: name.to_owned(),
        declarations: value.to_owned(),
        children: Vec::new(),
        css_text: String::new(),
    }
}

fn metadata_item(kind: &str, value: &str) -> ParsedRule {
    ParsedRule {
        kind: kind.to_owned(),
        prelude: value.to_owned(),
        declarations: String::new(),
        children: Vec::new(),
        css_text: String::new(),
    }
}

fn parse_css_string(value: &str) -> Option<String> {
    let mut input = ParserInput::new(value);
    let mut parser = Parser::new(&mut input);
    let value = match parser.next().ok()? {
        Token::QuotedString(value) => value.to_string(),
        _ => return None,
    };
    parser.is_exhausted().then_some(value)
}

fn preserve_source_text(rule: &mut ParsedRule, source: &str) {
    if matches!(rule.kind.as_str(), "generic" | "font-feature-values") {
        rule.css_text = trim_css_whitespace(source).to_owned();
    }
}

fn preserve_source_prelude(rule: &mut ParsedRule, header: &str) {
    let prefix = match rule.kind.as_str() {
        "media" => "@media",
        "supports" => "@supports",
        "container" => "@container",
        "layer-block" => "@layer",
        "scope" => "@scope",
        "page" => "@page",
        "position-try" => "@position-try",
        "keyframes" => {
            if trim_start_css_whitespace(header)
                .to_ascii_lowercase()
                .starts_with("@-webkit-keyframes")
            {
                "@-webkit-keyframes"
            } else {
                "@keyframes"
            }
        }
        "counter-style" => "@counter-style",
        "font-feature-values" => "@font-feature-values",
        _ => return,
    };
    if let Some(prelude) = trim_css_whitespace(header).get(prefix.len()..) {
        rule.prelude = if rule.kind == "font-feature-values" {
            format_condition_text(prelude)
        } else {
            trim_css_whitespace(prelude).to_owned()
        };
    }
}

fn recover_block_rule(
    mut probe: ParsedRule,
    body: &str,
    depth: usize,
) -> Result<ParsedRule, EngineError> {
    probe.css_text.clear();
    match probe.kind.as_str() {
        "style" => recover_style_body(&mut probe, body, depth)?,
        "media" | "supports" | "container" | "layer-block" | "scope" | "starting-style" => {
            probe.children = recover_child_rules(body, depth + 1)?;
        }
        "font-face" | "position-try" | "counter-style" => {
            probe.declarations = trim_css_whitespace(body).to_owned();
        }
        "font-feature-values" => recover_font_feature_values_body(&mut probe, body),
        "page" => recover_page_body(&mut probe, body, depth)?,
        "keyframes" => recover_keyframes_body(&mut probe, body, depth)?,
        _ => {
            return Err(EngineError::Parse(format!(
                "{} rules do not support declaration recovery",
                probe.kind
            )))
        }
    }
    Ok(probe)
}

fn recover_font_feature_values_body(probe: &mut ParsedRule, body: &str) {
    probe.children.clear();
    for fragment in scan_recovered_block_items(body) {
        let Some((header, declarations)) = split_outer_block(fragment) else {
            continue;
        };
        let Some(name) = font_feature_subrule_name(header) else {
            continue;
        };
        let Some(entries) = parse_font_feature_entries(declarations) else {
            continue;
        };
        let map_index = probe.children.iter().position(|candidate| {
            candidate.kind == "font-feature-map" && candidate.prelude == name
        });
        let map = if let Some(index) = map_index {
            &mut probe.children[index]
        } else {
            probe.children.push(ParsedRule {
                kind: "font-feature-map".to_owned(),
                prelude: name.clone(),
                declarations: String::new(),
                children: Vec::new(),
                css_text: String::new(),
            });
            let Some(map) = probe.children.last_mut() else {
                continue;
            };
            map
        };
        for mut entry in entries {
            if let Some(existing) = map
                .children
                .iter_mut()
                .find(|candidate| candidate.prelude == entry.prelude)
            {
                existing.declarations = std::mem::take(&mut entry.declarations);
                existing.css_text = std::mem::take(&mut entry.css_text);
            } else {
                map.children.push(entry);
            }
        }
    }
}

fn font_feature_subrule_name(header: &str) -> Option<String> {
    let mut input = ParserInput::new(header);
    let mut parser = Parser::new(&mut input);
    let name = match parser.next().ok()? {
        Token::AtKeyword(name) => name.to_ascii_lowercase(),
        _ => return None,
    };
    if !parser.is_exhausted()
        || !matches!(
            name.as_str(),
            "annotation" | "ornaments" | "stylistic" | "swash" | "character-variant" | "styleset"
        )
    {
        return None;
    }
    Some(name)
}

fn parse_font_feature_entries(source: &str) -> Option<Vec<ParsedRule>> {
    let mut entries = Vec::<ParsedRule>::new();
    for fragment in scan_recovered_block_items(source) {
        let declarations = parse_declaration_list(fragment);
        let [declaration] = declarations.as_slice() else {
            return None;
        };
        if declaration.important {
            return None;
        }
        let components = split_top_level_whitespace(declaration.value)?;
        if components.is_empty() {
            return None;
        }
        let mut values = Vec::with_capacity(components.len());
        for component in components {
            let value = component.parse::<i32>().ok()?;
            if value < 0 {
                return None;
            }
            values.push(value.to_string());
        }
        let entry = ParsedRule {
            kind: "font-feature-entry".to_owned(),
            prelude: declaration.name.to_string(),
            declarations: values.join(" "),
            children: Vec::new(),
            css_text: serialize_identifier(&declaration.name),
        };
        if let Some(existing) = entries
            .iter_mut()
            .find(|candidate| candidate.prelude == declaration.name)
        {
            *existing = entry;
        } else {
            entries.push(entry);
        }
    }
    Some(entries)
}

fn recover_style_body(probe: &mut ParsedRule, body: &str, depth: usize) -> Result<(), EngineError> {
    let fragments = scan_recovered_block_items(body);
    let mut declarations = Vec::new();
    let mut found_child = false;
    for fragment in fragments {
        match parse_recovered_rule_tree_inner(fragment, depth + 1) {
            Ok(child) => {
                flush_nested_declarations(probe, &mut declarations, found_child);
                found_child = true;
                probe.children.push(child);
            }
            Err(_) => declarations.push(fragment),
        }
    }
    flush_nested_declarations(probe, &mut declarations, found_child);
    Ok(())
}

fn flush_nested_declarations(probe: &mut ParsedRule, declarations: &mut Vec<&str>, nested: bool) {
    if declarations.is_empty() {
        return;
    }
    let source = declarations.join(" ");
    declarations.clear();
    if !nested {
        probe.declarations = source;
        return;
    }
    probe.children.push(ParsedRule {
        kind: "nested-declarations".to_owned(),
        prelude: String::new(),
        declarations: source,
        children: Vec::new(),
        css_text: String::new(),
    });
}

fn recover_child_rules(body: &str, depth: usize) -> Result<Vec<ParsedRule>, EngineError> {
    let mut children = Vec::new();
    for fragment in scan_recovered_block_items(body) {
        children.push(parse_recovered_rule_tree_inner(fragment, depth)?);
    }
    Ok(children)
}

fn recover_page_body(probe: &mut ParsedRule, body: &str, depth: usize) -> Result<(), EngineError> {
    let mut declarations = Vec::new();
    for fragment in scan_recovered_block_items(body) {
        if trim_start_css_whitespace(fragment).starts_with('@') {
            let (_, margin_body) = split_outer_block(fragment)
                .ok_or_else(|| EngineError::Parse("invalid page margin rule".to_owned()))?;
            let name = trim_start_css_whitespace(fragment)
                .strip_prefix('@')
                .and_then(|value| {
                    value
                        .split(|character: char| is_css_whitespace(character) || character == '{')
                        .next()
                })
                .unwrap_or_default();
            probe.children.push(ParsedRule {
                kind: "margin".to_owned(),
                prelude: name.to_ascii_lowercase(),
                declarations: trim_css_whitespace(margin_body).to_owned(),
                children: Vec::new(),
                css_text: String::new(),
            });
        } else {
            declarations.push(fragment);
        }
    }
    probe.declarations = declarations.join(" ");
    let limits = current_resource_limits();
    if depth > limits.max_nesting_depth {
        return Err(EngineError::NestingLimitExceeded {
            actual: depth,
            limit: limits.max_nesting_depth,
        });
    }
    Ok(())
}

fn recover_keyframes_body(
    probe: &mut ParsedRule,
    body: &str,
    depth: usize,
) -> Result<(), EngineError> {
    for fragment in scan_recovered_block_items(body) {
        let (header, declarations) = split_outer_block(fragment)
            .ok_or_else(|| EngineError::Parse("invalid keyframe rule".to_owned()))?;
        let wrapper = format!("@keyframes sheetom{{{header}{{}}}}");
        let mut parsed = parse_rule_tree_active(&wrapper)?;
        let mut keyframe = parsed
            .children
            .pop()
            .ok_or_else(|| EngineError::Parse("invalid keyframe selector".to_owned()))?;
        keyframe.declarations = trim_css_whitespace(declarations).to_owned();
        keyframe.css_text.clear();
        probe.children.push(keyframe);
    }
    let limits = current_resource_limits();
    if depth > limits.max_nesting_depth {
        return Err(EngineError::NestingLimitExceeded {
            actual: depth,
            limit: limits.max_nesting_depth,
        });
    }
    Ok(())
}

fn scan_recovered_block_items(source: &str) -> Vec<&str> {
    let bytes = source.as_bytes();
    let mut output = Vec::new();
    let mut start = None;
    let mut index = 0usize;
    let mut curly_depth = 0usize;
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
            if start.is_none() {
                start = Some(index);
            }
            in_comment = true;
            index += 2;
            continue;
        }
        if matches!(byte, b'\'' | b'"') {
            if start.is_none() {
                start = Some(index);
            }
            quote = Some(byte);
            index += 1;
            continue;
        }
        if start.is_none() {
            if byte.is_ascii_whitespace() || matches!(byte, b';' | b'}') {
                index += 1;
                continue;
            }
            start = Some(index);
        }

        match byte {
            b'{' => curly_depth = curly_depth.saturating_add(1),
            b'}' if curly_depth > 0 => {
                curly_depth -= 1;
                if curly_depth == 0 {
                    push_recovered_item(source, start.take(), index + 1, &mut output);
                }
            }
            b';' if curly_depth == 0 => {
                push_recovered_item(source, start.take(), index + 1, &mut output);
            }
            _ => {}
        }
        index += 1;
    }
    push_recovered_item(source, start, bytes.len(), &mut output);
    output
}

fn push_recovered_item<'a>(
    source: &'a str,
    start: Option<usize>,
    end: usize,
    output: &mut Vec<&'a str>,
) {
    let Some(start) = start else {
        return;
    };
    let item = trim_css_whitespace(&source[start..end]);
    if !item.is_empty() {
        output.push(item);
    }
}

pub fn parse_stylesheet_tree(
    source: &str,
    error_recovery: bool,
) -> Result<Vec<ParsedRule>, EngineError> {
    parse_stylesheet_tree_with_limits(source, error_recovery, ResourceLimits::default())
}

pub fn parse_stylesheet_tree_with_limits(
    source: &str,
    error_recovery: bool,
    limits: ResourceLimits,
) -> Result<Vec<ParsedRule>, EngineError> {
    with_resource_limits(limits, || {
        parse_stylesheet_tree_active(source, error_recovery)
    })
}

fn parse_stylesheet_tree_active(
    source: &str,
    error_recovery: bool,
) -> Result<Vec<ParsedRule>, EngineError> {
    run_parser_operation(source, || {
        parse_stylesheet_tree_inner(source, error_recovery)
    })
}

fn parse_stylesheet_tree_inner(
    source: &str,
    error_recovery: bool,
) -> Result<Vec<ParsedRule>, EngineError> {
    validate_stylesheet_budget(source)?;

    let sheet = StyleSheet::parse(
        source,
        ParserOptions {
            error_recovery,
            ..ParserOptions::default()
        },
    )
    .map_err(|error| EngineError::Parse(error.to_string()))?;

    let mut count = 0usize;
    let rules = convert_rule_list(&sheet.rules, &mut count)?;
    let limits = current_resource_limits();
    if count > limits.max_rules {
        return Err(EngineError::RuleLimitExceeded {
            actual: count,
            limit: limits.max_rules,
        });
    }
    Ok(rules)
}

fn convert_rule_list(
    rules: &CssRuleList<'_>,
    count: &mut usize,
) -> Result<Vec<ParsedRule>, EngineError> {
    let mut output = Vec::with_capacity(rules.0.len());
    for rule in &rules.0 {
        *count = count.saturating_add(1);
        let limits = current_resource_limits();
        if *count > limits.max_rules {
            return Err(EngineError::RuleLimitExceeded {
                actual: *count,
                limit: limits.max_rules,
            });
        }
        if let Some(rule) = convert_rule(rule, count)? {
            output.push(rule);
        }
    }
    Ok(output)
}

fn convert_rule(rule: &CssRule<'_>, count: &mut usize) -> Result<Option<ParsedRule>, EngineError> {
    let css_text = serialize(rule)?;
    let parsed = match rule {
        CssRule::Style(rule) => ParsedRule {
            kind: "style".to_owned(),
            prelude: serialize(&rule.selectors)?,
            declarations: serialize(&rule.declarations)?,
            children: convert_rule_list(&rule.rules, count)?,
            css_text,
        },
        CssRule::Media(rule) => ParsedRule {
            kind: "media".to_owned(),
            prelude: serialize(&rule.query)?,
            declarations: String::new(),
            children: convert_rule_list(&rule.rules, count)?,
            css_text,
        },
        CssRule::Supports(rule) => ParsedRule {
            kind: "supports".to_owned(),
            prelude: serialize(&rule.condition)?,
            declarations: String::new(),
            children: convert_rule_list(&rule.rules, count)?,
            css_text,
        },
        CssRule::Container(rule) => ParsedRule {
            kind: "container".to_owned(),
            prelude: block_prelude(&css_text, "@container"),
            declarations: String::new(),
            children: convert_rule_list(&rule.rules, count)?,
            css_text,
        },
        CssRule::LayerBlock(rule) => ParsedRule {
            kind: "layer-block".to_owned(),
            prelude: block_prelude(&css_text, "@layer"),
            declarations: String::new(),
            children: convert_rule_list(&rule.rules, count)?,
            css_text,
        },
        CssRule::LayerStatement(rule) => {
            let names = rule
                .names
                .iter()
                .map(serialize)
                .collect::<Result<Vec<_>, _>>()?;
            ParsedRule {
                kind: "layer-statement".to_owned(),
                prelude: String::new(),
                declarations: String::new(),
                children: names
                    .iter()
                    .map(|name| metadata_item("layer-name", name))
                    .collect(),
                css_text: format!("@layer {};", names.join(", ")),
            }
        }
        CssRule::Scope(rule) => ParsedRule {
            kind: "scope".to_owned(),
            prelude: block_prelude(&css_text, "@scope"),
            declarations: String::new(),
            children: convert_rule_list(&rule.rules, count)?,
            css_text,
        },
        CssRule::StartingStyle(rule) => ParsedRule {
            kind: "starting-style".to_owned(),
            prelude: String::new(),
            declarations: String::new(),
            children: convert_rule_list(&rule.rules, count)?,
            css_text,
        },
        CssRule::Import(_) => leaf("import", &css_text),
        CssRule::Namespace(rule) => {
            let prefix = rule
                .prefix
                .as_ref()
                .map(|prefix| prefix.0.as_ref())
                .unwrap_or_default();
            let namespace_uri = rule.url.0.as_ref();
            let mut serialized_uri = String::new();
            serialize_string(namespace_uri, &mut serialized_uri)
                .map_err(|error| EngineError::Serialize(error.to_string()))?;
            let serialized_prefix = rule
                .prefix
                .as_ref()
                .map(serialize)
                .transpose()?
                .map_or_else(String::new, |value| format!("{value} "));
            ParsedRule {
                kind: "namespace".to_owned(),
                prelude: String::new(),
                declarations: String::new(),
                children: vec![
                    property_descriptor("prefix", prefix),
                    property_descriptor("namespace-uri", namespace_uri),
                ],
                css_text: format!("@namespace {serialized_prefix}url({serialized_uri});"),
            }
        }
        CssRule::FontFace(_) => block_leaf("font-face", "@font-face", &css_text),
        CssRule::Page(rule) => {
            let mut children = Vec::with_capacity(rule.rules.len());
            for margin in &rule.rules {
                *count = count.saturating_add(1);
                children.push(block_leaf("margin", "", &serialize(margin)?));
            }
            ParsedRule {
                kind: "page".to_owned(),
                prelude: block_prelude(&css_text, "@page"),
                declarations: serialize(&rule.declarations)?,
                children,
                css_text,
            }
        }
        CssRule::PositionTry(rule) => ParsedRule {
            kind: "position-try".to_owned(),
            prelude: serialize(&rule.name)?,
            declarations: serialize(&rule.declarations)?,
            children: Vec::new(),
            css_text,
        },
        CssRule::Keyframes(rule) => {
            let mut children = Vec::with_capacity(rule.keyframes.len());
            for keyframe in &rule.keyframes {
                *count = count.saturating_add(1);
                children.push(ParsedRule {
                    kind: "keyframe".to_owned(),
                    prelude: keyframe
                        .selectors
                        .iter()
                        .map(serialize)
                        .collect::<Result<Vec<_>, _>>()?
                        .join(", "),
                    declarations: serialize(&keyframe.declarations)?,
                    children: Vec::new(),
                    css_text: serialize(keyframe)?,
                });
            }
            ParsedRule {
                kind: "keyframes".to_owned(),
                prelude: serialize(&rule.name)?,
                declarations: String::new(),
                children,
                css_text,
            }
        }
        CssRule::CounterStyle(rule) => ParsedRule {
            kind: "counter-style".to_owned(),
            prelude: serialize(&rule.name)?,
            declarations: serialize(&rule.declarations)?,
            children: Vec::new(),
            css_text,
        },
        CssRule::FontFeatureValues(rule) => {
            let mut children = Vec::with_capacity(rule.rules.len());
            for subrule in rule.rules.values() {
                if subrule
                    .declarations
                    .values()
                    .flatten()
                    .any(|value| *value < 0)
                {
                    continue;
                }
                let mut entries = Vec::with_capacity(subrule.declarations.len());
                for (name, values) in &subrule.declarations {
                    entries.push(ParsedRule {
                        kind: "font-feature-entry".to_owned(),
                        prelude: name.0.as_ref().to_owned(),
                        declarations: values
                            .iter()
                            .map(i32::to_string)
                            .collect::<Vec<_>>()
                            .join(" "),
                        children: Vec::new(),
                        css_text: serialize_identifier(name.0.as_ref()),
                    });
                }
                children.push(ParsedRule {
                    kind: "font-feature-map".to_owned(),
                    prelude: serialize(&subrule.name)?,
                    declarations: String::new(),
                    children: entries,
                    css_text: String::new(),
                });
            }
            ParsedRule {
                kind: "font-feature-values".to_owned(),
                prelude: block_prelude(&css_text, "@font-feature-values"),
                declarations: String::new(),
                children,
                css_text,
            }
        }
        CssRule::FontPaletteValues(rule) => {
            let mut font_family = None;
            let mut base_palette = None;
            let mut override_colors = None;
            for property in &rule.properties {
                match property {
                    FontPaletteValuesProperty::FontFamily(value) => {
                        font_family = Some(
                            value
                                .iter()
                                .map(serialize)
                                .collect::<Result<Vec<_>, _>>()?
                                .join(", "),
                        );
                    }
                    FontPaletteValuesProperty::BasePalette(value) => {
                        base_palette = Some(serialize(value)?);
                    }
                    FontPaletteValuesProperty::OverrideColors(values) => {
                        let mut serialized_values = Vec::with_capacity(values.len());
                        for value in values {
                            let serialized = serialize(value)?;
                            let parts = split_top_level_whitespace(&serialized)
                                .unwrap_or_else(|| vec![serialized.as_str()]);
                            let Some((index, color_parts)) = parts.split_first() else {
                                continue;
                            };
                            let color = color_parts.join(" ");
                            let observable_color = serialize_observable_color(&color);
                            serialized_values.push(format!("{index} {observable_color}"));
                        }
                        override_colors = Some(serialized_values.join(", "));
                    }
                    FontPaletteValuesProperty::Custom(_) => {}
                }
            }
            let mut descriptors = Vec::new();
            let mut children = Vec::new();
            for (name, value) in [
                ("font-family", font_family),
                ("base-palette", base_palette),
                ("override-colors", override_colors),
            ] {
                let Some(value) = value else {
                    continue;
                };
                descriptors.push(format!("{name}: {value};"));
                children.push(property_descriptor(name, &value));
            }
            let name = serialize(&rule.name)?;
            let contents = if descriptors.is_empty() {
                String::new()
            } else {
                format!(" {}", descriptors.join(" "))
            };
            ParsedRule {
                kind: "font-palette-values".to_owned(),
                prelude: name.clone(),
                declarations: descriptors.join(" "),
                children,
                css_text: format!("@font-palette-values {name} {{{contents} }}"),
            }
        }
        CssRule::ViewTransition(rule) => {
            let mut navigation = None;
            let mut types = None;
            let mut type_names = Vec::new();
            for property in &rule.properties {
                match property {
                    ViewTransitionProperty::Navigation(value) => {
                        navigation = Some(serialize(value)?);
                    }
                    ViewTransitionProperty::Types(value) => {
                        types = Some(serialize(value)?);
                        type_names = match value {
                            NoneOrCustomIdentList::None => Vec::new(),
                            NoneOrCustomIdentList::Idents(values) => values
                                .iter()
                                .map(|value| value.0.as_ref().to_owned())
                                .collect(),
                        };
                    }
                    ViewTransitionProperty::Custom(_) => {}
                }
            }
            let mut descriptors = Vec::new();
            let mut children = Vec::new();
            if let Some(value) = navigation {
                descriptors.push(format!("navigation: {value};"));
                children.push(property_descriptor("navigation", &value));
            }
            if let Some(value) = types {
                descriptors.push(format!("types: {value};"));
                children.push(property_descriptor("types", &value));
                children.extend(
                    type_names
                        .iter()
                        .map(|name| metadata_item("view-transition-type", name)),
                );
            }
            let contents = if descriptors.is_empty() {
                String::new()
            } else {
                format!(" {}", descriptors.join(" "))
            };
            ParsedRule {
                kind: "view-transition".to_owned(),
                prelude: String::new(),
                declarations: descriptors.join(" "),
                children,
                css_text: format!("@view-transition {{{contents} }}"),
            }
        }
        CssRule::Property(rule) => {
            let name = serialize(&rule.name)?;
            let syntax = serialize(&rule.syntax)?;
            let mut descriptors = vec![
                format!("syntax: {syntax};"),
                format!("inherits: {};", rule.inherits),
            ];
            let mut children = vec![
                property_descriptor("syntax", &parse_css_string(&syntax).unwrap_or_default()),
                property_descriptor("inherits", if rule.inherits { "true" } else { "false" }),
            ];
            if let Some(initial_value) = rule.initial_value.as_ref() {
                let initial_value = serialize(initial_value)?;
                descriptors.push(format!("initial-value: {initial_value};"));
                children.push(property_descriptor("initial-value", &initial_value));
            }
            let declarations = descriptors.join(" ");
            ParsedRule {
                kind: "property".to_owned(),
                prelude: name.clone(),
                declarations,
                children,
                css_text: format!("@property {name} {{ {} }}", descriptors.join(" ")),
            }
        }
        CssRule::NestedDeclarations(rule) => ParsedRule {
            kind: "nested-declarations".to_owned(),
            prelude: String::new(),
            declarations: serialize(&rule.declarations)?,
            children: Vec::new(),
            css_text,
        },
        CssRule::Ignored => return Ok(None),
        _ => {
            if let Some((header, body)) = split_outer_block(&css_text) {
                if is_function_rule_header(header) {
                    let Some(prelude) = parse_function_prelude(header) else {
                        return Ok(None);
                    };
                    let parsed_function = parse_function_rule(prelude, body, 0)?;
                    add_parsed_descendant_count(&parsed_function, count)?;
                    parsed_function
                } else {
                    leaf("generic", &css_text)
                }
            } else if is_function_rule_header(&css_text) {
                return Ok(None);
            } else {
                leaf("generic", &css_text)
            }
        }
    };
    Ok(Some(parsed))
}

fn add_parsed_descendant_count(rule: &ParsedRule, count: &mut usize) -> Result<(), EngineError> {
    let mut descendants = 0usize;
    let mut pending = rule.children.iter().collect::<Vec<_>>();
    while let Some(descendant) = pending.pop() {
        if parsed_rule_counts_as_node(descendant) {
            descendants = descendants.saturating_add(1);
        }
        pending.extend(descendant.children.iter());
    }
    *count = count.saturating_add(descendants);
    let limits = current_resource_limits();
    if *count > limits.max_rules {
        return Err(EngineError::RuleLimitExceeded {
            actual: *count,
            limit: limits.max_rules,
        });
    }
    Ok(())
}

fn parsed_rule_node_count(rule: &ParsedRule) -> usize {
    let mut count = 0usize;
    let mut pending = vec![rule];
    while let Some(current) = pending.pop() {
        if parsed_rule_counts_as_node(current) {
            count = count.saturating_add(1);
        }
        pending.extend(current.children.iter());
    }
    count
}

fn parsed_rule_counts_as_node(rule: &ParsedRule) -> bool {
    !matches!(
        rule.kind.as_str(),
        "function-parameter" | "property-descriptor" | "view-transition-type" | "layer-name"
    )
}

fn serialize<T: ToCss + ?Sized>(value: &T) -> Result<String, EngineError> {
    value
        .to_css_string(PrinterOptions {
            minify: true,
            ..PrinterOptions::default()
        })
        .map_err(|error| EngineError::Serialize(error.to_string()))
}

fn leaf(kind: &str, css_text: &str) -> ParsedRule {
    ParsedRule {
        kind: kind.to_owned(),
        prelude: String::new(),
        declarations: String::new(),
        children: Vec::new(),
        css_text: css_text.to_owned(),
    }
}

fn block_leaf(kind: &str, prefix: &str, css_text: &str) -> ParsedRule {
    ParsedRule {
        kind: kind.to_owned(),
        prelude: block_prelude(css_text, prefix),
        declarations: block_contents(css_text),
        children: Vec::new(),
        css_text: css_text.to_owned(),
    }
}

fn block_prelude(css_text: &str, prefix: &str) -> String {
    let Some(block) = css_text.find('{') else {
        return String::new();
    };
    trim_css_whitespace(
        css_text[..block]
            .strip_prefix(prefix)
            .unwrap_or(&css_text[..block]),
    )
    .to_owned()
}

fn block_contents(css_text: &str) -> String {
    let Some(start) = css_text.find('{') else {
        return String::new();
    };
    let Some(end) = css_text.rfind('}') else {
        return String::new();
    };
    if end <= start {
        return String::new();
    }
    trim_css_whitespace(&css_text[start + 1..end]).to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn returns_an_owned_rule_tree() {
        let rules = parse_stylesheet_tree(
            "@media screen {.x {width:1px;} @supports (display:grid) {.y {color:red;}}}",
            false,
        )
        .unwrap();
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].kind, "media");
        assert_eq!(rules[0].prelude, "screen");
        assert_eq!(rules[0].children[0].kind, "style");
        assert_eq!(rules[0].children[0].declarations, "width:1px");
        assert_eq!(rules[0].children[1].kind, "supports");
    }

    #[test]
    fn rejects_multiple_rules_in_strict_rule_mode() {
        let error = parse_rule_tree(".a{} .b{}").unwrap_err();
        assert!(error.to_string().contains("exactly one rule"));
    }

    #[test]
    fn serializes_without_borrowing_the_input() {
        let parsed = {
            let source = String::from("@font-face{font-family:Test;src:local(Test)}");
            parse_rule_tree(&source).unwrap()
        };
        assert_eq!(parsed.kind, "font-face");
        assert!(parsed.declarations.contains("font-family:Test"));
        assert!(parsed.declarations.contains("local(\"Test\")"));
    }

    #[test]
    fn classifies_the_public_authoring_rule_surface() {
        let source = r#"
            @import "theme.css" layer(theme) screen;
            @namespace svg url("urn:svg");
            @font-face { font-family: Test; src: local(Test); }
            @page :first { margin: 1cm; @top-left { content: "x"; } }
            @position-try --fallback { top: 1px; }
            @keyframes spin { from { opacity: 0; } to { opacity: 1; } }
            @counter-style icons { system: cyclic; symbols: "x"; }
            @font-feature-values Test { @styleset { nice: 1; } }
            @media screen { .media { color: red; } }
            @supports (display: grid) { .supports { display: grid; } }
            @container card (width > 1px) { .container { color: red; } }
            @layer reset, theme;
            @layer theme { .layer { color: red; } }
            @font-palette-values --brand { font-family: "A B", Test; base-palette: 2; override-colors: 0 red, 3 #00ff00; }
            @view-transition { navigation: auto; types: slide fade; }
            @scope (.start) to (.end) { .scope { color: red; } }
            @starting-style { .starting { opacity: 0; } }
            @property --space { syntax: "<length>"; inherits: false; initial-value: 0px; }
        "#;
        let parsed = parse_stylesheet_tree(source, false).unwrap();
        let kinds = parsed
            .iter()
            .map(|rule| rule.kind.as_str())
            .collect::<Vec<_>>();
        assert_eq!(
            kinds,
            [
                "import",
                "namespace",
                "font-face",
                "page",
                "position-try",
                "keyframes",
                "counter-style",
                "font-feature-values",
                "media",
                "supports",
                "container",
                "layer-statement",
                "layer-block",
                "font-palette-values",
                "view-transition",
                "scope",
                "starting-style",
                "property",
            ]
        );
        assert_eq!(parsed[1].css_text, "@namespace svg url(\"urn:svg\");");
        assert_eq!(parsed[3].children[0].kind, "margin");
        assert_eq!(parsed[5].children.len(), 2);
        assert_eq!(parsed[7].prelude, "Test");
        assert_eq!(parsed[7].children[0].kind, "font-feature-map");
        assert_eq!(parsed[7].children[0].prelude, "styleset");
        assert_eq!(parsed[7].children[0].children[0].prelude, "nice");
        assert_eq!(parsed[7].children[0].children[0].declarations, "1");
        assert_eq!(parsed[12].children[0].kind, "style");
        assert_eq!(parsed[13].prelude, "--brand");
        assert_eq!(parsed[13].children[0].declarations, "\"A B\", Test");
        assert_eq!(
            parsed[13].children[2].declarations,
            "0 red, 3 rgb(0, 255, 0)"
        );
        assert_eq!(parsed[14].children[0].declarations, "auto");
        assert_eq!(parsed[14].children[2].prelude, "slide");
    }

    #[test]
    fn applies_browser_descriptor_winners_and_drops_invalid_descriptors() {
        let font_palette = parse_rule_tree(
            "@font-palette-values --brand { font-family: Test; font-family: serif; base-palette: invalid; base-palette: dark; override-colors: 1 hsl(120 100% 50%); unknown: x; }",
        )
        .unwrap();
        assert_eq!(
            font_palette.css_text,
            "@font-palette-values --brand { font-family: Test; base-palette: dark; override-colors: 1 rgb(0, 255, 0); }"
        );

        let view_transition = parse_rule_tree(
            "@view-transition { navigation: bad; navigation: none; types: first; types: none; unknown: x; }",
        )
        .unwrap();
        assert_eq!(
            view_transition.css_text,
            "@view-transition { navigation: none; types: none; }"
        );
        assert_eq!(view_transition.children.len(), 2);
    }

    #[test]
    fn parses_custom_functions_into_parameters_and_declaration_runs() {
        let parsed = parse_recovered_rule_tree(
            "@function --mix(--x <number>: 1, --color <color>, --rest type(*)) returns <number> { --local: if(style(--x: 1): red; else: blue); result: calc(var(--x) * 2); @supports (width: 100px) { result: 100px; } --tail: 2; }",
        )
        .unwrap();
        assert_eq!(parsed.kind, "function");
        assert_eq!(parsed.prelude, "--mix");
        assert_eq!(parsed.declarations, "<number>");
        assert_eq!(parsed.children[0].kind, "function-parameter");
        assert_eq!(parsed.children[0].prelude, "--x");
        assert_eq!(parsed.children[0].declarations, "<number>");
        assert_eq!(parsed.children[0].children[0].declarations, "1");
        assert_eq!(parsed.children[2].declarations, "*");
        assert_eq!(parsed.children[3].kind, "function-declarations");
        assert!(parsed.children[3].declarations.contains("else: blue"));
        assert_eq!(parsed.children[4].kind, "supports");
        assert_eq!(parsed.children[4].children[0].kind, "function-declarations");
        assert_eq!(parsed.children[5].kind, "function-declarations");
    }

    #[test]
    fn function_body_scanner_preserves_nested_component_value_semicolons() {
        let parsed = parse_recovered_rule_tree(
            "@function --tokens() { --fn: foo(a;b); --square: [a;b]; --curly: {a;b} tail; --choice: if(style(--theme: dark): red; else: blue); result: ok; }",
        )
        .unwrap();
        assert_eq!(parsed.children.len(), 1);
        let declarations = &parsed.children[0].declarations;
        assert!(declarations.contains("foo(a;b)"));
        assert!(declarations.contains("[a;b]"));
        assert!(declarations.contains("{a;b} tail"));
        assert!(declarations.contains("else: blue"));
        assert!(declarations.contains("result: ok"));
    }

    #[test]
    fn strict_stylesheet_parser_classifies_custom_functions() {
        let parsed = parse_stylesheet_tree(
            "@function --value(--x <length>: 1px) returns <length> { result: var(--x); }",
            false,
        )
        .unwrap();
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].kind, "function");
        assert_eq!(parsed[0].prelude, "--value");
        assert_eq!(parsed[0].children[0].kind, "function-parameter");
        assert_eq!(parsed[0].children[1].kind, "function-declarations");
    }

    #[test]
    fn recovered_single_rule_parser_rejects_trailing_stylesheet_recovery() {
        let valid = parse_recovered_single_rule_tree(
            "/* before */ @media (width:1px){result:3px;} /* recovered EOF",
        )
        .unwrap();
        assert_eq!(valid.kind, "media");
        assert!(valid.children.is_empty());

        for trailing in ["junk", ";", "color:red", ".x{}", "@unknown"] {
            let source = format!("@media(width:1px){{result:3px;}} {trailing}");
            assert!(
                parse_recovered_single_rule_tree(&source).is_err(),
                "{trailing}"
            );
        }
    }

    #[test]
    fn invalid_known_function_rules_are_not_retained_as_generic_rules() {
        for source in [
            "@function --value () {}",
            "@function --value(--x <dino>) {}",
            "@function --value(--x <length>: 10deg) {}",
            "@function --value(--x) returns * {}",
        ] {
            assert!(
                parse_stylesheet_tree(source, false).unwrap().is_empty(),
                "{source}"
            );
            assert!(parse_recovered_rule_tree(source).is_err(), "{source}");
        }
    }

    #[test]
    fn recovers_font_feature_maps_with_browser_winners() {
        let parsed = parse_recovered_rule_tree(
            "@font-feature-values Test { @styleset { a: 1; } @styleset { b: 2; a: 3; } @annotation { mark: 0; } @swash { good: 1; bad: -1; } }",
        )
        .unwrap();
        assert_eq!(parsed.prelude, "Test");
        assert_eq!(parsed.children.len(), 2);
        assert_eq!(parsed.children[0].prelude, "styleset");
        assert_eq!(parsed.children[0].children[0].prelude, "a");
        assert_eq!(parsed.children[0].children[0].declarations, "3");
        assert_eq!(parsed.children[0].children[1].prelude, "b");
        assert_eq!(parsed.children[1].prelude, "annotation");
        assert_eq!(parsed.children[1].children[0].css_text, "mark");
    }

    #[test]
    fn serializes_font_family_setter_values_like_chromium() {
        for (input, expected) in [
            ("Other", "Other"),
            ("A B", "\"A B\""),
            ("\"A B\", Test", "\"\\\"A B\\\"\", Test"),
            ("A,,B", "A, B"),
            ("serif", "\"serif\""),
            ("--foo", "\"--foo\""),
            ("é", "é"),
        ] {
            assert_eq!(serialize_font_family_setter(input), expected, "{input}");
        }
    }

    #[test]
    fn scans_exact_top_level_rule_sources() {
        let source = r#"
            /* before */ ; }
            .é { --x: "a;b}"; color: red; }
            @media screen { .nested { content: "}"; } }
            @unknown fn(a;b) [x;y];
            .recovered { padding: 72px var(--space, var(--space,
        "#;
        let rules = scan_top_level_rules(source).unwrap();
        assert_eq!(rules.len(), 4);
        assert_eq!(rules[0], ".é { --x: \"a;b}\"; color: red; }");
        assert_eq!(rules[1], "@media screen { .nested { content: \"}\"; } }");
        assert_eq!(rules[2], "@unknown fn(a;b) [x;y];");
        assert_eq!(
            rules[3],
            ".recovered { padding: 72px var(--space, var(--space,"
        );
    }

    #[test]
    fn scanner_rejects_excessive_nesting_before_tokenization() {
        let source = format!("{}x{}", "fn(".repeat(4097), ")".repeat(4097));
        assert!(matches!(
            scan_top_level_rules(&source),
            Err(EngineError::NestingLimitExceeded { .. })
        ));
    }

    #[test]
    fn recovers_a_grouping_chain_at_the_default_nesting_boundary() {
        let group_depth = crate::DEFAULT_MAX_NESTING_DEPTH - 1;
        let source = format!(
            "{}.x{{color:red}}{}",
            "@media all{".repeat(group_depth),
            "}".repeat(group_depth)
        );
        let mut parsed = parse_recovered_rule_tree(&source).unwrap();
        let mut recovered_depth = 0usize;
        while parsed.kind == "media" {
            assert_eq!(parsed.children.len(), 1);
            parsed = parsed.children.pop().unwrap();
            recovered_depth += 1;
        }
        assert_eq!(recovered_depth, group_depth);
        assert_eq!(parsed.kind, "style");
        assert_eq!(parsed.declarations, "color:red");
    }

    #[test]
    fn recovers_a_nested_style_chain_at_the_default_nesting_boundary() {
        let style_depth = crate::DEFAULT_MAX_NESTING_DEPTH - 1;
        let source = format!(
            "{}color:red{}",
            ".x{".repeat(style_depth),
            "}".repeat(style_depth)
        );
        let mut parsed = parse_recovered_rule_tree(&source).unwrap();
        let mut recovered_depth = 0usize;
        while parsed.kind == "style" {
            recovered_depth += 1;
            if parsed.children.is_empty() {
                break;
            }
            assert_eq!(parsed.children.len(), 1);
            parsed = parsed.children.pop().unwrap();
        }
        assert_eq!(recovered_depth, style_depth);
        assert_eq!(parsed.declarations, "color:red");
    }

    #[test]
    fn recovers_a_branching_grouping_tree_at_the_default_nesting_boundary() {
        let group_depth = crate::DEFAULT_MAX_NESTING_DEPTH - 1;
        let mut source = String::with_capacity(group_depth * 32);
        for _ in 0..group_depth {
            source.push_str("@media all{");
        }
        source.push_str(".leaf{color:red}");
        for _ in 0..group_depth {
            source.push_str(".sibling{color:blue}}");
        }

        let mut parsed = parse_recovered_rule_tree(&source).unwrap();
        let cloned = parsed.clone();
        assert_eq!(cloned, parsed);
        let json = serialize_parsed_rule_json(&parsed).unwrap();
        assert_eq!(json.matches("\"kind\":\"media\"").count(), group_depth);
        assert!(json.starts_with("{\"kind\":\"media\""));
        assert!(json.ends_with("}"));
        let mut recovered_depth = 0usize;
        while parsed.kind == "media" {
            assert_eq!(parsed.children.len(), 2);
            parsed = parsed.children.remove(0);
            recovered_depth += 1;
        }
        assert_eq!(recovered_depth, group_depth);
        assert_eq!(parsed.kind, "style");
        assert_eq!(parsed.declarations, "color:red");
    }

    #[test]
    fn recovers_deep_function_conditionals_without_native_recursion() {
        let conditional_depth = LARGE_STACK_DEPTH_THRESHOLD + 1;
        let source = format!(
            "@function --nested() {{ {}result: 1;{} }}",
            "@supports (display:grid) {".repeat(conditional_depth),
            "}".repeat(conditional_depth)
        );
        let mut parsed = parse_recovered_rule_tree(&source).unwrap();
        assert_eq!(parsed.kind, "function");
        let mut recovered_depth = 0usize;
        while parsed.kind == "function" || parsed.kind == "supports" {
            if parsed.kind == "supports" {
                recovered_depth += 1;
            }
            assert_eq!(parsed.children.len(), 1);
            parsed = parsed.children.pop().unwrap();
        }
        assert_eq!(recovered_depth, conditional_depth);
        assert_eq!(parsed.kind, "function-declarations");
        assert_eq!(parsed.declarations, "result: 1;");
    }

    #[test]
    fn iterative_rule_json_matches_serde_for_shallow_forests() {
        let rules = parse_stylesheet_tree(
            "@media screen { .x { color: red } .y { padding: 1px } } @layer theme;",
            false,
        )
        .unwrap();
        assert_eq!(
            serialize_parsed_rules_json(&rules).unwrap(),
            serde_json::to_string(&rules).unwrap()
        );
        assert_eq!(
            serialize_parsed_rule_json(&rules[0]).unwrap(),
            serde_json::to_string(&rules[0]).unwrap()
        );
    }

    #[test]
    fn recovers_browser_style_malformed_declarations() {
        let parsed =
            parse_recovered_rule_tree(".x { padding: 72px var(--space, var(--space,; }").unwrap();
        assert_eq!(parsed.kind, "style");
        assert_eq!(
            parsed.declarations,
            "padding: 72px var(--space, var(--space,;"
        );
    }

    #[test]
    fn recovers_malformed_declarations_through_grouping_rules() {
        let parsed = parse_recovered_rule_tree(
            "@layer theme { .x { padding: 72px var(--space, var(--space,; } @media screen { .y { color: red; } } }",
        )
        .unwrap();
        assert_eq!(parsed.kind, "layer-block");
        assert_eq!(parsed.children[0].kind, "style");
        assert!(parsed.children[0].declarations.contains("var(--space"));
        assert_eq!(parsed.children[1].kind, "media");
    }

    #[test]
    fn recovers_nested_declaration_order() {
        let parsed =
            parse_recovered_rule_tree(".x { color: red; .child { width: 1px; } height: 2px; }")
                .unwrap();
        assert_eq!(parsed.declarations, "color: red;");
        assert_eq!(parsed.children[0].kind, "style");
        assert_eq!(parsed.children[1].kind, "nested-declarations");
        assert_eq!(parsed.children[1].declarations, "height: 2px;");
    }

    #[test]
    fn recovers_page_and_keyframe_declaration_blocks() {
        let page = parse_recovered_rule_tree(
            "@page :first { margin: 72px var(--x,; @top-left { content: var(--x,; } }",
        )
        .unwrap();
        assert!(page.declarations.contains("margin:"));
        assert_eq!(page.children[0].kind, "margin");
        assert!(page.children[0].declarations.contains("content:"));

        let keyframes = parse_recovered_rule_tree(
            "@keyframes spin { from { opacity: var(--x,; } to { opacity: 1; } }",
        )
        .unwrap();
        assert_eq!(keyframes.children.len(), 2);
        assert!(keyframes.children[0].declarations.contains("var(--x"));
    }

    #[test]
    fn normalizes_public_rule_preludes_without_modernizing_legacy_queries() {
        assert_eq!(normalize_selector_text(".a,.b").unwrap(), ".a, .b");
        assert_eq!(
            normalize_media_text("screen and (max-width:767px),print").unwrap(),
            "screen and (max-width: 767px), print"
        );
        assert_eq!(
            normalize_supports_text("(display:grid)").unwrap(),
            "(display:grid)"
        );

        let container = parse_container_prelude("card (max-width:767px)").unwrap();
        assert_eq!(container.condition_text, "card (max-width: 767px)");
        assert_eq!(container.name, "card");
        assert_eq!(container.query, "(max-width: 767px)");
    }

    #[test]
    fn parses_and_canonicalizes_scope_boundaries() {
        assert_eq!(
            parse_scope_prelude("(.a,.b) to (.c)").unwrap(),
            ParsedScopePrelude {
                start: Some(".a, .b".to_owned()),
                end: Some(".c".to_owned()),
            }
        );
        assert_eq!(
            parse_scope_prelude("").unwrap(),
            ParsedScopePrelude {
                start: None,
                end: None,
            }
        );
    }
}
