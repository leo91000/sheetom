use lightningcss::{
    declaration::DeclarationBlock,
    stylesheet::{ParserOptions, PrinterOptions},
    traits::ToCss,
};
use napi_derive::napi;

const ENGINE_REVISION: &str = "lightningcss-1.33.0-c6a0c3ce";

fn canonicalize(source: &str) -> Result<String, String> {
    let declarations = DeclarationBlock::parse_string(source, ParserOptions::default())
        .map_err(|error| error.to_string())?;

    declarations
        .to_css_string(PrinterOptions::default())
        .map_err(|error| error.to_string())
}

/// Identifies the exact parser baseline compiled into the native addon.
#[napi]
pub fn native_engine_revision() -> &'static str {
    ENGINE_REVISION
}

/// Exercises the native string boundary while the engine runs in shadow mode.
///
/// This deliberately accepts and returns owned strings. Lightning CSS AST nodes
/// never cross Node-API and therefore cannot be deserialized back into Rust.
#[napi]
pub fn canonicalize_declaration_block(source: String) -> napi::Result<String> {
    canonicalize(&source).map_err(napi::Error::from_reason)
}

#[cfg(test)]
mod tests {
    use super::{canonicalize, native_engine_revision};

    #[test]
    fn reports_the_vendored_engine_revision() {
        assert_eq!(native_engine_revision(), "lightningcss-1.33.0-c6a0c3ce");
    }

    #[test]
    fn image_set_never_crosses_an_ast_boundary() {
        let css = canonicalize(
            "background: image-set(url(a.png) 1x, url(b.png) 2x) center/cover no-repeat red",
        )
        .expect("valid Chromium background should parse");

        assert!(css.contains("image-set("));
        assert!(css.contains("background:"));
    }
}
