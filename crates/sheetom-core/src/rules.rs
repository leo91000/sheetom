use crate::{
    observable::{serialize_observable_value, ObservableCategory},
    scan_safety_metrics,
    syntax::{parse_declaration_list, serialize_identifier, split_top_level_whitespace},
    EngineError, MAX_NESTING_DEPTH,
};
use cssparser::{serialize_string, Parser, ParserInput, SourcePosition, Token};
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
use std::panic::{catch_unwind, AssertUnwindSafe};

const MAX_STYLESHEET_BYTES: usize = 16 * 1024 * 1024;
const MAX_RULES: usize = 100_000;
const MAX_RULE_NESTING_DEPTH: usize = 256;

/// An owned, parser-independent description of one CSS rule.
///
/// This is the only rule representation allowed to cross Node-API. In
/// particular, Lightning CSS AST nodes always remain inside Rust.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ParsedRule {
    pub kind: String,
    pub prelude: String,
    pub declarations: String,
    pub children: Vec<ParsedRule>,
    pub css_text: String,
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
    catch_unwind(AssertUnwindSafe(|| {
        let rule_source = format!("{source}{{}} ");
        validate_stylesheet_budget(&rule_source)?;
        let sheet = StyleSheet::parse(&rule_source, ParserOptions::default())
            .map_err(|error| EngineError::Parse(error.to_string()))?;
        let Some(CssRule::Style(rule)) = sheet.rules.0.first() else {
            return Err(EngineError::Parse("invalid selector list".to_owned()));
        };
        rule.selectors
            .to_css_string(PrinterOptions::default())
            .map_err(|error| EngineError::Serialize(error.to_string()))
    }))
    .unwrap_or(Err(EngineError::UnexpectedPanic))
}

pub fn normalize_media_text(source: &str) -> Result<String, EngineError> {
    if source.trim().is_empty() {
        return Ok(String::new());
    }
    let parsed = parse_rule_tree(&format!("@media {source}{{}}"))?;
    if parsed.kind != "media" {
        return Err(EngineError::Parse("invalid media query list".to_owned()));
    }
    Ok(format_condition_text(source))
}

pub fn normalize_supports_text(source: &str) -> Result<String, EngineError> {
    let parsed = parse_rule_tree(&format!("@supports {source}{{}}"))?;
    if parsed.kind != "supports" {
        return Err(EngineError::Parse("invalid supports condition".to_owned()));
    }
    Ok(parsed.prelude)
}

pub fn parse_container_prelude(source: &str) -> Result<ParsedContainerPrelude, EngineError> {
    let parsed = parse_rule_tree(&format!("@container {source}{{}}"))?;
    if parsed.kind != "container" {
        return Err(EngineError::Parse("invalid container query".to_owned()));
    }
    let condition_text = format_condition_text(source);
    let query_start = container_query_start(&condition_text);
    let (name, query) = condition_text.split_at(query_start);
    let name = name.trim().to_owned();
    let query = query.trim().to_owned();
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
    let parsed = parse_rule_tree(&format!("@scope {source}{{}}"))?;
    if parsed.kind != "scope" {
        return Err(EngineError::Parse("invalid scope prelude".to_owned()));
    }
    let text = source.trim();
    if text.is_empty() {
        return Ok(ParsedScopePrelude {
            start: None,
            end: None,
        });
    }
    let Some((start, remainder)) = consume_parenthesized(text) else {
        return Err(EngineError::Parse("invalid scope start".to_owned()));
    };
    let remainder = remainder.trim();
    if remainder.is_empty() {
        return Ok(ParsedScopePrelude {
            start: Some(normalize_selector_text(start)?),
            end: None,
        });
    }
    let Some(after_to) = remainder.strip_prefix("to") else {
        return Err(EngineError::Parse("invalid scope boundary".to_owned()));
    };
    let Some((end, trailing)) = consume_parenthesized(after_to.trim()) else {
        return Err(EngineError::Parse("invalid scope end".to_owned()));
    };
    if !trailing.trim().is_empty() {
        return Err(EngineError::Parse(
            "invalid scope trailing input".to_owned(),
        ));
    }
    Ok(ParsedScopePrelude {
        start: Some(normalize_selector_text(start)?),
        end: Some(normalize_selector_text(end)?),
    })
}

pub fn serialize_font_family_setter(value: &str) -> String {
    value
        .split(',')
        .map(str::trim)
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
    source.find(char::is_whitespace).unwrap_or(source.len())
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
        if character.is_whitespace() {
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
    output.trim().to_owned()
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

/// Consumes top-level CSS Syntax rules while preserving their exact source.
pub fn scan_top_level_rules(source: &str) -> Result<Vec<String>, EngineError> {
    catch_unwind(AssertUnwindSafe(|| scan_top_level_rules_inner(source)))
        .unwrap_or(Err(EngineError::UnexpectedPanic))
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
        if output.len() > MAX_RULES {
            return Err(EngineError::RuleLimitExceeded {
                actual: output.len(),
                limit: MAX_RULES,
            });
        }
    }

    Ok(output)
}

fn validate_stylesheet_budget(source: &str) -> Result<(), EngineError> {
    if source.len() > MAX_STYLESHEET_BYTES {
        return Err(EngineError::InputLimitExceeded {
            actual: source.len(),
            limit: MAX_STYLESHEET_BYTES,
        });
    }
    let metrics = scan_safety_metrics(source);
    if metrics.maximum_depth > MAX_NESTING_DEPTH {
        return Err(EngineError::NestingLimitExceeded {
            actual: metrics.maximum_depth,
            limit: MAX_NESTING_DEPTH,
        });
    }
    Ok(())
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
    let source = parser.slice(start..parser.position()).trim();
    if !source.is_empty() {
        output.push(source.to_owned());
    }
}

fn split_outer_block(source: &str) -> Option<(String, String)> {
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
                let header = parser.slice(start..token_start).trim().to_owned();
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
                return Some((header, body.to_owned()));
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
    let mut rules = parse_stylesheet_tree(source, false)?;
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
    catch_unwind(AssertUnwindSafe(|| {
        validate_stylesheet_budget(source)?;
        parse_recovered_rule_tree_inner(source, 0)
    }))
    .unwrap_or(Err(EngineError::UnexpectedPanic))
}

fn parse_recovered_rule_tree_inner(source: &str, depth: usize) -> Result<ParsedRule, EngineError> {
    if depth > MAX_RULE_NESTING_DEPTH {
        return Err(EngineError::NestingLimitExceeded {
            actual: depth,
            limit: MAX_RULE_NESTING_DEPTH,
        });
    }
    let strict = parse_rule_tree(source).ok();
    let Some((header, body)) = split_outer_block(source) else {
        return strict
            .map(|mut parsed| {
                preserve_source_text(&mut parsed, source);
                parsed
            })
            .ok_or_else(|| EngineError::Parse("the rule has no recoverable block".to_owned()));
    };
    if let Some(parsed) = strict.as_ref() {
        if parsed.kind == "property" {
            let mut parsed = parsed.clone();
            preserve_property_descriptor_values(&mut parsed, &body);
            return Ok(parsed);
        }
        let raw_items = scan_recovered_block_items(&body).len();
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
            preserve_source_prelude(&mut parsed, &header);
            preserve_source_text(&mut parsed, source);
            return Ok(parsed);
        }
    }

    let probe_source = format!("{header}{{}}");
    let mut probe = parse_rule_tree(&probe_source)?;
    preserve_source_prelude(&mut probe, &header);
    recover_block_rule(probe, &body, depth)
}

fn preserve_property_descriptor_values(rule: &mut ParsedRule, body: &str) {
    let declarations = parse_declaration_list(body);
    let last_value = |name: &str| {
        declarations
            .iter()
            .rev()
            .find(|declaration| declaration.name.eq_ignore_ascii_case(name))
            .map(|declaration| declaration.value.trim())
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
        rule.css_text = source.trim().to_owned();
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
            if header
                .trim_start()
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
    if let Some(prelude) = header.trim().get(prefix.len()..) {
        rule.prelude = if rule.kind == "font-feature-values" {
            format_condition_text(prelude)
        } else {
            prelude.trim().to_owned()
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
            probe.declarations = body.trim().to_owned();
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
        let Some((header, declarations)) = split_outer_block(&fragment) else {
            continue;
        };
        let Some(name) = font_feature_subrule_name(&header) else {
            continue;
        };
        let Some(entries) = parse_font_feature_entries(&declarations) else {
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
        for entry in entries {
            if let Some(existing) = map
                .children
                .iter_mut()
                .find(|candidate| candidate.prelude == entry.prelude)
            {
                existing.declarations = entry.declarations;
                existing.css_text = entry.css_text;
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
        let declarations = parse_declaration_list(&fragment);
        let [declaration] = declarations.as_slice() else {
            return None;
        };
        if declaration.important {
            return None;
        }
        let components = split_top_level_whitespace(&declaration.value)?;
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
            prelude: declaration.name.clone(),
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
        match parse_recovered_rule_tree_inner(&fragment, depth + 1) {
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

fn flush_nested_declarations(probe: &mut ParsedRule, declarations: &mut Vec<String>, nested: bool) {
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
        children.push(parse_recovered_rule_tree_inner(&fragment, depth)?);
    }
    Ok(children)
}

fn recover_page_body(probe: &mut ParsedRule, body: &str, depth: usize) -> Result<(), EngineError> {
    let mut declarations = Vec::new();
    for fragment in scan_recovered_block_items(body) {
        if fragment.trim_start().starts_with('@') {
            let (_, margin_body) = split_outer_block(&fragment)
                .ok_or_else(|| EngineError::Parse("invalid page margin rule".to_owned()))?;
            let name = fragment
                .trim_start()
                .strip_prefix('@')
                .and_then(|value| {
                    value
                        .split(|character: char| character.is_whitespace() || character == '{')
                        .next()
                })
                .unwrap_or_default();
            probe.children.push(ParsedRule {
                kind: "margin".to_owned(),
                prelude: name.to_ascii_lowercase(),
                declarations: margin_body.trim().to_owned(),
                children: Vec::new(),
                css_text: String::new(),
            });
        } else {
            declarations.push(fragment);
        }
    }
    probe.declarations = declarations.join(" ");
    if depth > MAX_RULE_NESTING_DEPTH {
        return Err(EngineError::NestingLimitExceeded {
            actual: depth,
            limit: MAX_RULE_NESTING_DEPTH,
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
        let (header, declarations) = split_outer_block(&fragment)
            .ok_or_else(|| EngineError::Parse("invalid keyframe rule".to_owned()))?;
        let wrapper = format!("@keyframes sheetom{{{header}{{}}}}");
        let parsed = parse_rule_tree(&wrapper)?;
        let keyframe = parsed
            .children
            .into_iter()
            .next()
            .ok_or_else(|| EngineError::Parse("invalid keyframe selector".to_owned()))?;
        probe.children.push(ParsedRule {
            declarations: declarations.trim().to_owned(),
            css_text: String::new(),
            ..keyframe
        });
    }
    if depth > MAX_RULE_NESTING_DEPTH {
        return Err(EngineError::NestingLimitExceeded {
            actual: depth,
            limit: MAX_RULE_NESTING_DEPTH,
        });
    }
    Ok(())
}

fn scan_recovered_block_items(source: &str) -> Vec<String> {
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

fn push_recovered_item(source: &str, start: Option<usize>, end: usize, output: &mut Vec<String>) {
    let Some(start) = start else {
        return;
    };
    let item = source[start..end].trim();
    if !item.is_empty() {
        output.push(item.to_owned());
    }
}

pub fn parse_stylesheet_tree(
    source: &str,
    error_recovery: bool,
) -> Result<Vec<ParsedRule>, EngineError> {
    catch_unwind(AssertUnwindSafe(|| {
        parse_stylesheet_tree_inner(source, error_recovery)
    }))
    .unwrap_or(Err(EngineError::UnexpectedPanic))
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
    if count > MAX_RULES {
        return Err(EngineError::RuleLimitExceeded {
            actual: count,
            limit: MAX_RULES,
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
        if *count > MAX_RULES {
            return Err(EngineError::RuleLimitExceeded {
                actual: *count,
                limit: MAX_RULES,
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
                            let observable_color = serialize_observable_value(
                                "color",
                                &color,
                                &color,
                                ObservableCategory::Typed,
                            );
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
        _ => leaf("generic", &css_text),
    };
    Ok(Some(parsed))
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
    css_text[..block]
        .strip_prefix(prefix)
        .unwrap_or(&css_text[..block])
        .trim()
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
    css_text[start + 1..end].trim().to_owned()
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
