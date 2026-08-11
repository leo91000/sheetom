use std::sync::Arc;

use lightningcss::values::syntax::SyntaxString;

use crate::{
    EngineError, RecoveredClosure, RecoveredComponentKind, RecoveredComponentValue,
    RecoveredTokenKind, RecoveredValue, SourceSpan,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SubstitutionFunctionKind {
    Var,
    Env,
    Attr,
    If,
    Custom,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SemanticSubstitutionFunction {
    name: Arc<str>,
    kind: SubstitutionFunctionKind,
    span: SourceSpan,
    closure: RecoveredClosure,
}

impl SemanticSubstitutionFunction {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn kind(&self) -> SubstitutionFunctionKind {
        self.kind
    }

    pub fn span(&self) -> SourceSpan {
        self.span
    }

    pub fn closure(&self) -> RecoveredClosure {
        self.closure
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SemanticSubstitutionValue {
    functions: Arc<[SemanticSubstitutionFunction]>,
}

impl SemanticSubstitutionValue {
    pub fn functions(&self) -> &[SemanticSubstitutionFunction] {
        &self.functions
    }
}

pub fn analyze_recovered_substitutions(
    recovered: &RecoveredValue,
) -> Result<Option<SemanticSubstitutionValue>, EngineError> {
    let mut functions = Vec::new();
    let mut pending = Vec::with_capacity(recovered.values().len());
    for component in recovered.values().iter().rev() {
        pending.push((component, true));
    }

    while let Some((component, top_level)) = pending.pop() {
        match &component.kind {
            RecoveredComponentKind::Token(token) => {
                if token.parse_error {
                    return Err(invalid_substitution("contains a CSS token parse error"));
                }
                if top_level && token.kind == RecoveredTokenKind::Semicolon {
                    return Err(invalid_substitution("contains a declaration delimiter"));
                }
            }
            RecoveredComponentKind::SimpleBlock { values, .. } => {
                for child in values.iter().rev() {
                    pending.push((child, false));
                }
            }
            RecoveredComponentKind::Function {
                name,
                values,
                closure,
                ..
            } => {
                if name == "--" {
                    return Err(invalid_substitution(
                        "custom function name must contain characters after --",
                    ));
                }
                if let Some(kind) = substitution_kind(name) {
                    validate_substitution_function(kind, values, recovered.source())?;
                    functions.push(SemanticSubstitutionFunction {
                        name: Arc::from(name.as_str()),
                        kind,
                        span: component.span,
                        closure: *closure,
                    });
                }
                for child in values.iter().rev() {
                    pending.push((child, false));
                }
            }
        }
    }

    if functions.is_empty() {
        return Ok(None);
    }
    Ok(Some(SemanticSubstitutionValue {
        functions: functions.into(),
    }))
}

fn substitution_kind(name: &str) -> Option<SubstitutionFunctionKind> {
    // `name` is the decoded value of a cssparser <function-token>, never raw CSS text.
    if name.starts_with("--") {
        return Some(SubstitutionFunctionKind::Custom);
    }
    if name.eq_ignore_ascii_case("var") {
        return Some(SubstitutionFunctionKind::Var);
    }
    if name.eq_ignore_ascii_case("env") {
        return Some(SubstitutionFunctionKind::Env);
    }
    if name.eq_ignore_ascii_case("attr") {
        return Some(SubstitutionFunctionKind::Attr);
    }
    if name.eq_ignore_ascii_case("if") {
        return Some(SubstitutionFunctionKind::If);
    }
    None
}

fn validate_substitution_function(
    kind: SubstitutionFunctionKind,
    values: &[RecoveredComponentValue],
    source: &str,
) -> Result<(), EngineError> {
    match kind {
        SubstitutionFunctionKind::Var => validate_var(values),
        SubstitutionFunctionKind::Env => validate_named_substitution(values),
        SubstitutionFunctionKind::Attr => validate_attr(values, source),
        SubstitutionFunctionKind::If => validate_if(values),
        SubstitutionFunctionKind::Custom => validate_custom_function(values),
    }
}

fn validate_attr(values: &[RecoveredComponentValue], source: &str) -> Result<(), EngineError> {
    reject_direct_delimiters(values)?;
    let first_segment = significant_components(segment_before_first_comma(values));
    let [name, optional_type @ ..] = first_segment.as_slice() else {
        return Err(invalid_substitution("attr() requires an attribute name"));
    };
    if !matches!(
        &name.kind,
        RecoveredComponentKind::Token(token)
            if matches!(token.kind, RecoveredTokenKind::Ident(_))
    ) {
        return Err(invalid_substitution(
            "attr() requires one unqualified attribute name",
        ));
    }
    match optional_type {
        [] => Ok(()),
        [component] if is_attr_type(component, source) => Ok(()),
        _ => Err(invalid_substitution(
            "attr() accepts at most one unit or type() after its name",
        )),
    }
}

fn is_attr_type(component: &RecoveredComponentValue, source: &str) -> bool {
    match &component.kind {
        RecoveredComponentKind::Token(token) => matches!(
            token.kind,
            RecoveredTokenKind::Ident(_) | RecoveredTokenKind::Delimiter('%')
        ),
        RecoveredComponentKind::Function { name, values, .. }
            if name.eq_ignore_ascii_case("type") =>
        {
            let (Some(first), Some(last)) = (values.first(), values.last()) else {
                return false;
            };
            let Some(syntax) = source.get(first.span.start..last.span.end) else {
                return false;
            };
            SyntaxString::parse_string(syntax).is_ok()
        }
        _ => false,
    }
}

fn validate_var(values: &[RecoveredComponentValue]) -> Result<(), EngineError> {
    reject_direct_delimiters(values)?;
    let first_segment = segment_before_first_comma(values);
    let significant = significant_components(first_segment);
    let valid_name = matches!(
        significant.as_slice(),
        [RecoveredComponentValue {
            kind: RecoveredComponentKind::Token(token),
            ..
        }] if matches!(&token.kind, RecoveredTokenKind::Ident(name) if name.starts_with("--") && name.len() > 2)
    );
    if valid_name {
        return Ok(());
    }
    Err(invalid_substitution(
        "var() requires one custom property name",
    ))
}

fn validate_named_substitution(values: &[RecoveredComponentValue]) -> Result<(), EngineError> {
    reject_direct_delimiters(values)?;
    let first_segment = significant_components(segment_before_first_comma(values));
    let valid_name = first_segment.first().is_some_and(|component| {
        matches!(
            &component.kind,
            RecoveredComponentKind::Token(token)
                if matches!(token.kind, RecoveredTokenKind::Ident(_))
        )
    });
    if valid_name {
        return Ok(());
    }
    Err(invalid_substitution("env() requires a leading identifier"))
}

fn validate_if(values: &[RecoveredComponentValue]) -> Result<(), EngineError> {
    let mut branch_start = 0;
    let mut branch_count = 0;
    for branch_end in values
        .iter()
        .enumerate()
        .filter_map(|(index, component)| is_semicolon(component).then_some(index))
        .chain(std::iter::once(values.len()))
    {
        let branch = significant_components(&values[branch_start..branch_end]);
        let Some(colon) = branch.iter().position(|component| is_colon(component)) else {
            return Err(invalid_substitution("if() branch requires a colon"));
        };
        if colon == 0 || colon + 1 == branch.len() {
            return Err(invalid_substitution(
                "if() branch requires a condition and a value",
            ));
        }
        branch_count += 1;
        branch_start = branch_end + 1;
    }
    if branch_count > 0 {
        return Ok(());
    }
    Err(invalid_substitution("if() requires a condition branch"))
}

fn validate_custom_function(values: &[RecoveredComponentValue]) -> Result<(), EngineError> {
    reject_direct_delimiters(values)?;
    let significant = significant_components(values);
    if significant.is_empty() {
        return Ok(());
    }

    let mut segment_has_value = false;
    for (index, component) in significant.into_iter().enumerate() {
        if is_comma(component) {
            // Chromium accepts one omitted leading argument (`--f(, value)`),
            // but rejects every other empty segment and a trailing comma.
            if !segment_has_value && index != 0 {
                return Err(invalid_substitution(
                    "custom function arguments cannot be empty",
                ));
            }
            segment_has_value = false;
            continue;
        }
        segment_has_value = true;
    }
    if segment_has_value {
        return Ok(());
    }
    Err(invalid_substitution(
        "custom function arguments cannot end with a comma",
    ))
}

fn reject_direct_delimiters(values: &[RecoveredComponentValue]) -> Result<(), EngineError> {
    let invalid = values.iter().any(|component| {
        matches!(
            &component.kind,
            RecoveredComponentKind::Token(token)
                if matches!(token.kind, RecoveredTokenKind::Semicolon | RecoveredTokenKind::Delimiter('!'))
        )
    });
    if invalid {
        return Err(invalid_substitution(
            "substitution arguments contain a declaration delimiter",
        ));
    }
    Ok(())
}

fn segment_before_first_comma(values: &[RecoveredComponentValue]) -> &[RecoveredComponentValue] {
    let end = values.iter().position(is_comma).unwrap_or(values.len());
    &values[..end]
}

fn significant_components(values: &[RecoveredComponentValue]) -> Vec<&RecoveredComponentValue> {
    values
        .iter()
        .filter(|component| !is_ignorable(component))
        .collect()
}

fn is_ignorable(component: &RecoveredComponentValue) -> bool {
    matches!(
        &component.kind,
        RecoveredComponentKind::Token(token)
            if matches!(token.kind, RecoveredTokenKind::Whitespace | RecoveredTokenKind::Comment)
    )
}

fn is_comma(component: &RecoveredComponentValue) -> bool {
    matches!(
        &component.kind,
        RecoveredComponentKind::Token(token) if token.kind == RecoveredTokenKind::Comma
    )
}

fn is_colon(component: &RecoveredComponentValue) -> bool {
    matches!(
        &component.kind,
        RecoveredComponentKind::Token(token) if token.kind == RecoveredTokenKind::Colon
    )
}

fn is_semicolon(component: &RecoveredComponentValue) -> bool {
    matches!(
        &component.kind,
        RecoveredComponentKind::Token(token) if token.kind == RecoveredTokenKind::Semicolon
    )
}

fn invalid_substitution(message: &str) -> EngineError {
    EngineError::Parse(format!("invalid substitution value: {message}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::recover_component_values;

    fn analyze(source: &str) -> Result<Option<SemanticSubstitutionValue>, EngineError> {
        analyze_recovered_substitutions(&recover_component_values(source).unwrap())
    }

    #[test]
    fn finds_nested_substitutions_without_scanning_strings_or_comments() {
        let analysis =
            analyze("\"var(--ignored)\" /* env(ignored) */ calc(var(--x) + --scale(2, 3))")
                .unwrap()
                .unwrap();

        assert_eq!(analysis.functions().len(), 2);
        assert_eq!(
            analysis.functions()[0].kind(),
            SubstitutionFunctionKind::Var
        );
        assert_eq!(
            analysis.functions()[1].kind(),
            SubstitutionFunctionKind::Custom
        );
    }

    #[test]
    fn preserves_implicit_eof_recovery_as_semantic_evidence() {
        let analysis = analyze("72px var(--space, var(--space,").unwrap().unwrap();

        assert_eq!(analysis.functions().len(), 2);
        assert!(analysis
            .functions()
            .iter()
            .all(|function| function.closure() == RecoveredClosure::ImplicitEof));
    }

    #[test]
    fn accepts_current_substitution_function_families() {
        for source in [
            "var(--x)",
            "var(--x,)",
            "env(safe-area-inset-top)",
            "attr(data-width type(<length>), 1px)",
            "attr(data-label raw-string)",
            "attr(data-ratio % , 0%)",
            "attr(data-size type(<length> | <percentage>), 1px)",
            "if(style(--theme: dark): white; else: black)",
            "--scale()",
            "--scale(, 2)",
            "--scale(1, 2)",
            "calc(--scale(1) + var(--x))",
        ] {
            assert!(analyze(source).unwrap().is_some(), "{source}");
        }
    }

    #[test]
    fn rejects_invalid_substitution_neighbors() {
        for source in [
            "var()",
            "var(foo)",
            "var(--x extra)",
            "env()",
            "attr()",
            "attr(data-width | namespace)",
            "attr(data-width px extra)",
            "attr(data-width type())",
            "attr(data-width type(<angle> #), 1deg)",
            "attr(data-width type(<unknown>), 1px)",
            "if()",
            "if(style(--theme: dark):)",
            "if(: white)",
            "if(style(--theme: dark): white;)",
            "--()",
            "--scale(,)",
            "--scale(1,)",
            "--scale(1,,2)",
            "--scale(1;2)",
            "--scale(!)",
            "var(--x; red)",
            "red; color: blue",
        ] {
            assert!(analyze(source).is_err(), "{source}");
        }
    }

    #[test]
    fn leaves_static_values_unclassified() {
        assert_eq!(analyze("calc(1px + 2px)").unwrap(), None);
    }
}
