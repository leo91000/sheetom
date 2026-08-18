use lightningcss::properties::PropertyId;
use std::borrow::Cow;

use crate::{
    browser_longhand::has_browser_longhand_grammar, geometric_value::has_geometric_property_grammar,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum PropertyGrammarExtension {
    AspectRatio,
    BrowserLonghand,
    Content,
    GapRuleLonghand,
    Geometric,
    IntegerCalculation,
    LengthNumberCalculation,
    LengthPercentageNumberCalculation,
    LengthPercentageOrNumberCalculation,
    OffsetPosition,
    OffsetRotate,
    PageSize,
    WebkitBoxReflect,
    WebkitBorderImage,
    WebkitMaskBoxImageComponent,
    WebkitPerspective,
}

mod generated {
    include!("generated/chromium_properties.rs");
}

pub use generated::{CHROMIUM_BASELINE, INITIAL_VALUES_SOURCE_SHA256, SOURCE_SHA256};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum PropertyGrammarOwner {
    CustomTokenStream,
    Lightning,
    SheetomAlias,
    SheetomExtension,
    Unsupported,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct PropertyGrammar<'a> {
    canonical_name: Cow<'a, str>,
    parser_name: Option<&'static str>,
    owner: PropertyGrammarOwner,
    extensions: &'static [PropertyGrammarExtension],
}

impl PropertyGrammar<'_> {
    pub(crate) fn canonical_name(&self) -> &str {
        &self.canonical_name
    }

    pub(crate) fn parser_name(&self) -> &str {
        self.parser_name.unwrap_or(&self.canonical_name)
    }

    pub(crate) fn owner(&self) -> PropertyGrammarOwner {
        self.owner
    }

    pub(crate) fn extensions(&self) -> &'static [PropertyGrammarExtension] {
        self.extensions
    }

    pub(crate) fn has_standard_parser(&self) -> bool {
        matches!(
            self.owner,
            PropertyGrammarOwner::Lightning | PropertyGrammarOwner::SheetomAlias
        )
    }
}

pub(crate) fn ascii_lowercase(value: &str) -> Cow<'_, str> {
    if value.bytes().any(|byte| byte.is_ascii_uppercase()) {
        Cow::Owned(value.to_ascii_lowercase())
    } else {
        Cow::Borrowed(value)
    }
}

pub(crate) fn canonical_property_name(name: &str) -> Option<Cow<'_, str>> {
    if name.starts_with("--") {
        return (name.len() > 2).then_some(Cow::Borrowed(name));
    }

    let lower = ascii_lowercase(name);
    if lower == "grid-gap" {
        return Some(Cow::Borrowed("gap"));
    }
    if let Ok(index) =
        generated::PROPERTY_ALIASES.binary_search_by_key(&lower.as_ref(), |(alias, _)| *alias)
    {
        return Some(Cow::Borrowed(generated::PROPERTY_ALIASES[index].1));
    }
    if let Some(unprefixed) = lower.strip_prefix("-webkit-") {
        let prefixed_longhands = shorthand_longhands(&lower);
        let unprefixed_longhands = shorthand_longhands(unprefixed);
        if prefixed_longhands.is_some() && prefixed_longhands == unprefixed_longhands {
            return Some(if matches!(lower, Cow::Owned(_)) {
                Cow::Owned(unprefixed.to_owned())
            } else {
                Cow::Borrowed(name.strip_prefix("-webkit-").unwrap_or(name))
            });
        }
    }
    if lower.starts_with("-webkit-") {
        if let Some(longhands) = shorthand_longhands(&lower) {
            if let Some((canonical, _)) =
                generated::SHORTHAND_LONGHANDS
                    .iter()
                    .find(|(candidate, candidate_longhands)| {
                        !candidate.starts_with('-')
                            && *candidate != lower
                            && *candidate_longhands == longhands
                    })
            {
                return Some(Cow::Borrowed(canonical));
            }
        }
    }
    generated::SUPPORTED_PROPERTIES
        .binary_search(&lower.as_ref())
        .is_ok()
        .then_some(lower)
}

pub(crate) fn property_alias_hides_value(name: &str) -> bool {
    name.eq_ignore_ascii_case("-webkit-column-break-after")
        || name.eq_ignore_ascii_case("-webkit-column-break-before")
        || name.eq_ignore_ascii_case("-webkit-column-break-inside")
}

pub(crate) fn property_alias_defers_pending_value(name: &str) -> bool {
    property_alias_hides_value(name)
        || name.eq_ignore_ascii_case("page-break-after")
        || name.eq_ignore_ascii_case("page-break-before")
        || name.eq_ignore_ascii_case("page-break-inside")
}

pub(crate) fn property_alias_observable_value<'a>(
    name: &str,
    canonical_value: &'a str,
) -> Option<Option<&'a str>> {
    let supported_css_wide = matches!(
        canonical_value,
        "initial" | "inherit" | "unset" | "revert" | "revert-layer" | "revert-rule"
    );
    if name.eq_ignore_ascii_case("page-break-before")
        || name.eq_ignore_ascii_case("page-break-after")
    {
        return Some(match canonical_value {
            "page" => Some("always"),
            "auto" | "avoid" | "left" | "right" => Some(canonical_value),
            _ if supported_css_wide => Some(canonical_value),
            _ => None,
        });
    }
    if name.eq_ignore_ascii_case("page-break-inside") {
        return Some(match canonical_value {
            "auto" | "avoid" => Some(canonical_value),
            _ if supported_css_wide => Some(canonical_value),
            _ => None,
        });
    }
    None
}

pub(crate) fn property_grammar(name: &str) -> Option<PropertyGrammar<'_>> {
    let canonical_name = canonical_property_name(name)?;
    if canonical_name.starts_with("--") {
        return Some(PropertyGrammar {
            parser_name: None,
            canonical_name,
            owner: PropertyGrammarOwner::CustomTokenStream,
            extensions: &[],
        });
    }

    let parser_name = sheetom_parser_property_name(&canonical_name);
    let extensions = if name.eq_ignore_ascii_case("-webkit-perspective") {
        &[PropertyGrammarExtension::WebkitPerspective][..]
    } else {
        property_grammar_extensions(&canonical_name)
    };
    let lightning_supports_property = !matches!(
        PropertyId::from(parser_name.unwrap_or(&canonical_name)),
        PropertyId::Custom(_)
    );
    let owner = if parser_name.is_some() {
        PropertyGrammarOwner::SheetomAlias
    } else if lightning_supports_property {
        PropertyGrammarOwner::Lightning
    } else if !extensions.is_empty() {
        PropertyGrammarOwner::SheetomExtension
    } else {
        PropertyGrammarOwner::Unsupported
    };

    Some(PropertyGrammar {
        canonical_name,
        parser_name,
        owner,
        extensions,
    })
}

fn property_grammar_extensions(name: &str) -> &'static [PropertyGrammarExtension] {
    if has_geometric_property_grammar(name) {
        return &[PropertyGrammarExtension::Geometric];
    }
    if has_browser_longhand_grammar(name) {
        return &[PropertyGrammarExtension::BrowserLonghand];
    }
    let Ok(index) = generated::PROPERTY_GRAMMAR_EXTENSIONS
        .binary_search_by_key(&name, |(property, _)| *property)
    else {
        return &[];
    };
    generated::PROPERTY_GRAMMAR_EXTENSIONS[index].1
}

pub(crate) fn sheetom_parser_property_name(name: &str) -> Option<&'static str> {
    if name == "-webkit-mask-position-x" {
        return Some("mask-position-x");
    }
    if name == "-webkit-mask-position-y" {
        return Some("mask-position-y");
    }
    if name == "offset-path" {
        return Some("clip-path");
    }
    if matches!(
        name,
        "-webkit-border-after"
            | "-webkit-border-before"
            | "-webkit-border-end"
            | "-webkit-border-start"
            | "-webkit-column-rule"
            | "-webkit-text-stroke"
            | "column-rule"
            | "row-rule"
            | "rule"
    ) {
        return Some("border");
    }
    if name == "grid-gap" {
        return Some("gap");
    }
    match name {
        "-webkit-column-rule-width"
        | "-webkit-text-stroke-width"
        | "column-rule-width"
        | "row-rule-width"
        | "rule-width" => return Some("border-top-width"),
        "-webkit-column-rule-style" | "column-rule-style" | "row-rule-style" | "rule-style" => {
            return Some("border-top-style");
        }
        "-webkit-column-rule-color"
        | "-webkit-text-stroke-color"
        | "column-rule-color"
        | "row-rule-color"
        | "rule-color" => return Some("border-top-color"),
        _ => {}
    }
    None
}

pub(crate) fn shorthand_longhands(name: &str) -> Option<&'static [&'static str]> {
    let index = generated::SHORTHAND_LONGHANDS
        .binary_search_by_key(&name, |(shorthand, _)| *shorthand)
        .ok()?;
    Some(generated::SHORTHAND_LONGHANDS[index].1)
}

pub(crate) fn observed_shorthand_longhands(name: &str) -> Option<&'static [&'static str]> {
    let index = generated::OBSERVED_SHORTHAND_LONGHANDS
        .binary_search_by_key(&name, |(shorthand, _)| *shorthand)
        .ok()?;
    Some(generated::OBSERVED_SHORTHAND_LONGHANDS[index].1)
}

pub(crate) fn shorthand_names() -> impl Iterator<Item = &'static str> {
    generated::SHORTHAND_LONGHANDS
        .iter()
        .map(|(shorthand, _)| *shorthand)
}

pub(crate) fn initial_longhand_value(name: &str) -> Option<&'static str> {
    let index = generated::INITIAL_LONGHAND_VALUES
        .binary_search_by_key(&name, |(longhand, _)| *longhand)
        .ok()?;
    Some(generated::INITIAL_LONGHAND_VALUES[index].1)
}

#[cfg(test)]
pub(crate) fn initial_longhand_values() -> impl Iterator<Item = (&'static str, &'static str)> {
    generated::INITIAL_LONGHAND_VALUES.iter().copied()
}

#[cfg(test)]
mod tests {
    use super::{
        canonical_property_name, initial_longhand_value, property_grammar,
        sheetom_parser_property_name, shorthand_longhands, PropertyGrammarExtension,
        PropertyGrammarOwner, CHROMIUM_BASELINE, INITIAL_VALUES_SOURCE_SHA256, SOURCE_SHA256,
    };
    use std::borrow::Cow;

    #[test]
    fn catalog_is_pinned_to_the_chromium_manifest() {
        assert_eq!(
            SOURCE_SHA256,
            "6622a8e9731e13437f56221fac0def5e5c0fe75ceb190e7b5194ffb1e64983c1"
        );
        assert!(CHROMIUM_BASELINE.contains("Chrome/151."));
        assert_eq!(
            INITIAL_VALUES_SOURCE_SHA256,
            "9effd1fbe4220206d95b664a58906759d0ee62d420ec6ff85244939b9dc79a33"
        );
    }

    #[test]
    fn canonicalizes_ordinary_names_and_preserves_custom_property_case() {
        assert_eq!(canonical_property_name("WIDTH").as_deref(), Some("width"));
        assert_eq!(
            canonical_property_name("word-wrap").as_deref(),
            Some("overflow-wrap")
        );
        assert_eq!(
            canonical_property_name("--Theme").as_deref(),
            Some("--Theme")
        );
        assert_eq!(canonical_property_name("not-a-browser-property"), None);
        assert_eq!(canonical_property_name("--"), None);
        assert_eq!(
            canonical_property_name("-webkit-animation").as_deref(),
            Some("animation")
        );
        assert_eq!(
            canonical_property_name("-webkit-border-after").as_deref(),
            Some("border-block-end")
        );
    }

    #[test]
    fn borrows_canonical_property_names_when_normalization_is_unnecessary() {
        assert!(matches!(
            canonical_property_name("width"),
            Some(Cow::Borrowed("width"))
        ));
        assert!(matches!(
            canonical_property_name("word-wrap"),
            Some(Cow::Borrowed("overflow-wrap"))
        ));
        assert!(matches!(
            canonical_property_name("WIDTH"),
            Some(Cow::Owned(name)) if name == "width"
        ));
    }

    #[test]
    fn exposes_ordered_shorthand_longhands() {
        assert_eq!(
            shorthand_longhands("overflow"),
            Some(["overflow-x", "overflow-y"].as_slice())
        );
        assert_eq!(shorthand_longhands("width"), None);
        assert_eq!(initial_longhand_value("animation-timeline"), Some("auto"));
    }

    #[test]
    fn parser_aliases_are_a_closed_property_mapping() {
        assert_eq!(
            sheetom_parser_property_name("column-rule-width"),
            Some("border-top-width")
        );
        assert_eq!(
            sheetom_parser_property_name("-webkit-text-stroke-color"),
            Some("border-top-color")
        );
        assert_eq!(sheetom_parser_property_name("imaginary-rule-width"), None);
        assert_eq!(sheetom_parser_property_name("not-a-rule-color"), None);
    }

    #[test]
    fn routes_every_manifested_property_through_an_explicit_grammar() {
        let mut owner_counts = [0usize; 5];
        for name in super::generated::SUPPORTED_PROPERTIES {
            let grammar = property_grammar(name).unwrap_or_else(|| panic!("missing {name}"));
            assert!(!grammar.canonical_name().is_empty(), "{name}");
            assert!(!grammar.parser_name().is_empty(), "{name}");
            let index = match grammar.owner() {
                PropertyGrammarOwner::CustomTokenStream => 0,
                PropertyGrammarOwner::Lightning => 1,
                PropertyGrammarOwner::SheetomAlias => 2,
                PropertyGrammarOwner::SheetomExtension => 3,
                PropertyGrammarOwner::Unsupported => 4,
            };
            owner_counts[index] += 1;
        }
        assert_eq!(owner_counts, [0, 429, 22, 218, 42]);
    }

    #[test]
    fn has_no_unowned_complex_ordinary_grammars() {
        let unsupported = super::generated::SUPPORTED_PROPERTIES
            .iter()
            .copied()
            .filter(|name| shorthand_longhands(name).is_none())
            .filter(|name| {
                property_grammar(name)
                    .is_some_and(|grammar| grammar.owner() == PropertyGrammarOwner::Unsupported)
            })
            .collect::<Vec<_>>();
        assert!(unsupported.is_empty());
    }

    #[test]
    fn distinguishes_standard_alias_extension_and_unsupported_owners() {
        let width = property_grammar("width").unwrap();
        assert_eq!(width.owner(), PropertyGrammarOwner::Lightning);
        assert_eq!(width.extensions(), &[]);
        assert!(width.has_standard_parser());

        for name in [
            "-webkit-tap-highlight-color",
            "-webkit-text-fill-color",
            "flood-color",
            "lighting-color",
            "scrollbar-color",
            "stop-color",
        ] {
            let color = property_grammar(name).unwrap();
            assert_eq!(color.owner(), PropertyGrammarOwner::Lightning, "{name}");
            assert_eq!(color.extensions(), &[], "{name}");
            assert!(color.has_standard_parser(), "{name}");
        }

        let alias = property_grammar("row-rule").unwrap();
        assert_eq!(alias.owner(), PropertyGrammarOwner::SheetomAlias);
        assert_eq!(alias.parser_name(), "border");

        let content = property_grammar("content").unwrap();
        assert_eq!(content.owner(), PropertyGrammarOwner::SheetomExtension);
        assert_eq!(content.extensions(), &[PropertyGrammarExtension::Content]);
        assert!(!content.has_standard_parser());

        let browser_longhand = property_grammar("-webkit-font-smoothing").unwrap();
        assert_eq!(
            browser_longhand.owner(),
            PropertyGrammarOwner::SheetomExtension
        );
        assert_eq!(
            browser_longhand.extensions(),
            &[PropertyGrammarExtension::BrowserLonghand]
        );

        let geometric = property_grammar("border-shape").unwrap();
        assert_eq!(geometric.owner(), PropertyGrammarOwner::SheetomExtension);
        assert_eq!(
            geometric.extensions(),
            &[PropertyGrammarExtension::Geometric]
        );

        let custom = property_grammar("--Theme").unwrap();
        assert_eq!(custom.owner(), PropertyGrammarOwner::CustomTokenStream);
    }
}
