#![cfg_attr(
    not(test),
    deny(clippy::expect_used, clippy::panic, clippy::unwrap_used)
)]

#[cfg(panic = "abort")]
compile_error!("sheetom-core must be compiled with panic=unwind");

use lightningcss::{
    declaration::DeclarationBlock,
    stylesheet::{ParserOptions, PrinterOptions},
    traits::ToCss,
};
use std::{
    fmt::{Display, Formatter},
    panic::{catch_unwind, AssertUnwindSafe},
};

pub const ENGINE_REVISION: &str = "lightningcss-1.33.0-c6a0c3ce";
const MAX_DECLARATION_BYTES: usize = 1024 * 1024;
const MAX_DECLARATIONS_PER_BLOCK: usize = 100_000;
const MAX_NESTING_DEPTH: usize = 4096;

#[derive(Debug, PartialEq)]
pub enum EngineError {
    InputLimitExceeded { actual: usize, limit: usize },
    DeclarationLimitExceeded { actual: usize, limit: usize },
    NestingLimitExceeded { actual: usize, limit: usize },
    Parse(String),
    Serialize(String),
    UnexpectedPanic,
}

impl Display for EngineError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InputLimitExceeded { actual, limit } => write!(
                formatter,
                "SHEETOM_INPUT_LIMIT: declaration block is {actual} bytes; the limit is {limit} bytes"
            ),
            Self::DeclarationLimitExceeded { actual, limit } => write!(
                formatter,
                "SHEETOM_DECLARATION_LIMIT: declaration block has {actual} entries; the limit is {limit}"
            ),
            Self::NestingLimitExceeded { actual, limit } => write!(
                formatter,
                "SHEETOM_NESTING_LIMIT: CSS nesting depth is {actual}; the limit is {limit}"
            ),
            Self::Parse(message) => write!(formatter, "SHEETOM_PARSE_ERROR: {message}"),
            Self::Serialize(message) => write!(formatter, "SHEETOM_SERIALIZE_ERROR: {message}"),
            Self::UnexpectedPanic => formatter.write_str(
                "SHEETOM_NATIVE_PANIC: the CSS engine aborted the current operation safely",
            ),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct SafetyMetrics {
    declaration_count: usize,
    maximum_depth: usize,
}

fn scan_safety_metrics(source: &str) -> SafetyMetrics {
    let bytes = source.as_bytes();
    let mut metrics = SafetyMetrics::default();
    let mut depth = 0usize;
    let mut index = 0usize;
    let mut quote = None;
    let mut in_comment = false;
    let mut has_top_level_content = false;

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

        if matches!(byte, b'\'' | b'"') {
            quote = Some(byte);
            has_top_level_content |= depth == 0;
            index += 1;
            continue;
        }

        if byte == b'\\' {
            has_top_level_content |= depth == 0;
            index = (index + 2).min(bytes.len());
            continue;
        }

        if matches!(byte, b'(' | b'[' | b'{') {
            depth += 1;
            metrics.maximum_depth = metrics.maximum_depth.max(depth);
            has_top_level_content |= depth == 1;
            index += 1;
            continue;
        }

        if matches!(byte, b')' | b']' | b'}') {
            depth = depth.saturating_sub(1);
            has_top_level_content |= depth == 0;
            index += 1;
            continue;
        }

        if byte == b';' && depth == 0 {
            if has_top_level_content {
                metrics.declaration_count += 1;
                has_top_level_content = false;
            }
            index += 1;
            continue;
        }

        has_top_level_content |= depth == 0 && !byte.is_ascii_whitespace();
        index += 1;
    }

    if has_top_level_content {
        metrics.declaration_count += 1;
    }

    metrics
}

fn run_guarded<T>(operation: impl FnOnce() -> Result<T, EngineError>) -> Result<T, EngineError> {
    match catch_unwind(AssertUnwindSafe(operation)) {
        Ok(result) => result,
        Err(_) => Err(EngineError::UnexpectedPanic),
    }
}

fn canonicalize_unchecked(source: &str) -> Result<String, EngineError> {
    let declarations = DeclarationBlock::parse_string(source, ParserOptions::default())
        .map_err(|error| EngineError::Parse(error.to_string()))?;

    declarations
        .to_css_string(PrinterOptions::default())
        .map_err(|error| EngineError::Serialize(error.to_string()))
}

pub fn canonicalize_declaration_block(source: &str) -> Result<String, EngineError> {
    if source.len() > MAX_DECLARATION_BYTES {
        return Err(EngineError::InputLimitExceeded {
            actual: source.len(),
            limit: MAX_DECLARATION_BYTES,
        });
    }

    let metrics = scan_safety_metrics(source);
    if metrics.maximum_depth > MAX_NESTING_DEPTH {
        return Err(EngineError::NestingLimitExceeded {
            actual: metrics.maximum_depth,
            limit: MAX_NESTING_DEPTH,
        });
    }

    if metrics.declaration_count > MAX_DECLARATIONS_PER_BLOCK {
        return Err(EngineError::DeclarationLimitExceeded {
            actual: metrics.declaration_count,
            limit: MAX_DECLARATIONS_PER_BLOCK,
        });
    }

    run_guarded(|| canonicalize_unchecked(source))
}

#[doc(hidden)]
pub fn fuzz_declaration_block(source: &str) {
    let _ = canonicalize_declaration_block(source);
}

#[cfg(test)]
mod tests {
    use super::{
        canonicalize_declaration_block, run_guarded, scan_safety_metrics, EngineError,
        SafetyMetrics, ENGINE_REVISION, MAX_DECLARATIONS_PER_BLOCK, MAX_DECLARATION_BYTES,
        MAX_NESTING_DEPTH,
    };

    #[test]
    fn reports_the_vendored_engine_revision() {
        assert_eq!(ENGINE_REVISION, "lightningcss-1.33.0-c6a0c3ce");
    }

    #[test]
    fn image_set_never_crosses_an_ast_boundary() {
        let css = canonicalize_declaration_block(
            "background: image-set(url(a.png) 1x, url(b.png) 2x) center/cover no-repeat red",
        )
        .expect("valid Chromium background should parse");

        assert!(css.contains("image-set("));
        assert!(css.contains("background:"));
    }

    #[test]
    fn rejects_oversized_inputs_before_parsing() {
        let source = "x".repeat(MAX_DECLARATION_BYTES + 1);

        assert_eq!(
            canonicalize_declaration_block(&source),
            Err(EngineError::InputLimitExceeded {
                actual: MAX_DECLARATION_BYTES + 1,
                limit: MAX_DECLARATION_BYTES,
            })
        );
    }

    #[test]
    fn accepts_the_exact_declaration_budget() {
        let source = format!("--x: {}", "x".repeat(MAX_DECLARATION_BYTES - 5));

        assert!(canonicalize_declaration_block(&source).is_ok());
    }

    #[test]
    fn converts_panics_into_recoverable_internal_errors() {
        let result: Result<(), EngineError> = run_guarded(|| panic!("simulated parser panic"));

        assert_eq!(result, Err(EngineError::UnexpectedPanic));
    }

    #[test]
    fn safety_scanner_ignores_nested_and_quoted_delimiters() {
        assert_eq!(
            scan_safety_metrics(r#"--x: "a;b"; background: fn([a;b], {c;d}); color: red"#),
            SafetyMetrics {
                declaration_count: 3,
                maximum_depth: 2,
            }
        );
    }

    #[test]
    fn rejects_excessive_nesting_before_parsing() {
        let source = format!("--x: {}value", "fn(".repeat(MAX_NESTING_DEPTH + 1));

        assert_eq!(
            canonicalize_declaration_block(&source),
            Err(EngineError::NestingLimitExceeded {
                actual: MAX_NESTING_DEPTH + 1,
                limit: MAX_NESTING_DEPTH,
            })
        );
    }

    #[test]
    fn rejects_too_many_declarations_before_parsing() {
        let source = "x:;".repeat(MAX_DECLARATIONS_PER_BLOCK + 1);

        assert_eq!(
            canonicalize_declaration_block(&source),
            Err(EngineError::DeclarationLimitExceeded {
                actual: MAX_DECLARATIONS_PER_BLOCK + 1,
                limit: MAX_DECLARATIONS_PER_BLOCK,
            })
        );
    }
}
