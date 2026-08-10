use crate::{scan_safety_metrics, EngineError, MAX_NESTING_DEPTH};
use lightningcss::{
    rules::{CssRule, CssRuleList},
    stylesheet::{ParserOptions, PrinterOptions, StyleSheet},
    traits::ToCss,
};
use serde::Serialize;
use std::panic::{catch_unwind, AssertUnwindSafe};

const MAX_STYLESHEET_BYTES: usize = 16 * 1024 * 1024;
const MAX_RULES: usize = 100_000;

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
        CssRule::LayerStatement(_) => leaf("layer-statement", &css_text),
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
        CssRule::FontFeatureValues(_) => leaf("font-feature-values", &css_text),
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
                "scope",
                "starting-style",
                "generic",
            ]
        );
        assert_eq!(parsed[2].children[0].kind, "margin");
        assert_eq!(parsed[4].children.len(), 2);
        assert_eq!(parsed[11].children[0].kind, "style");
    }
}
