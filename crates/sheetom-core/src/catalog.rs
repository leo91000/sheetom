use lightningcss::properties::PropertyId;

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
    WebkitMaskBoxImageSlice,
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
pub(crate) struct PropertyGrammar {
    canonical_name: String,
    parser_name: String,
    owner: PropertyGrammarOwner,
    extensions: &'static [PropertyGrammarExtension],
}

impl PropertyGrammar {
    pub(crate) fn canonical_name(&self) -> &str {
        &self.canonical_name
    }

    pub(crate) fn parser_name(&self) -> &str {
        &self.parser_name
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

pub(crate) fn canonical_property_name(name: &str) -> Option<String> {
    if name.starts_with("--") {
        return (name.len() > 2).then(|| name.to_owned());
    }

    let lower = name.to_ascii_lowercase();
    if lower == "grid-gap" {
        return Some("gap".to_owned());
    }
    if let Ok(index) =
        generated::PROPERTY_ALIASES.binary_search_by_key(&lower.as_str(), |(alias, _)| *alias)
    {
        return Some(generated::PROPERTY_ALIASES[index].1.to_owned());
    }
    if let Some(unprefixed) = lower.strip_prefix("-webkit-") {
        let prefixed_longhands = shorthand_longhands(&lower);
        let unprefixed_longhands = shorthand_longhands(unprefixed);
        if prefixed_longhands.is_some() && prefixed_longhands == unprefixed_longhands {
            return Some(unprefixed.to_owned());
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
                return Some((*canonical).to_owned());
            }
        }
    }
    generated::SUPPORTED_PROPERTIES
        .binary_search(&lower.as_str())
        .is_ok()
        .then_some(lower)
}

pub(crate) fn property_alias_hides_value(name: &str) -> bool {
    matches!(
        name.to_ascii_lowercase().as_str(),
        "-webkit-column-break-after"
            | "-webkit-column-break-before"
            | "-webkit-column-break-inside"
    )
}

pub(crate) fn property_alias_defers_pending_value(name: &str) -> bool {
    property_alias_hides_value(name)
        || matches!(
            name.to_ascii_lowercase().as_str(),
            "page-break-after" | "page-break-before" | "page-break-inside"
        )
}

pub(crate) fn property_grammar(name: &str) -> Option<PropertyGrammar> {
    let canonical_name = canonical_property_name(name)?;
    if canonical_name.starts_with("--") {
        return Some(PropertyGrammar {
            parser_name: canonical_name.clone(),
            canonical_name,
            owner: PropertyGrammarOwner::CustomTokenStream,
            extensions: &[],
        });
    }

    let alias_parser_name = sheetom_parser_property_name(&canonical_name);
    let parser_name = alias_parser_name.unwrap_or(&canonical_name).to_owned();
    let extensions = if name.eq_ignore_ascii_case("-webkit-perspective") {
        &[PropertyGrammarExtension::WebkitPerspective][..]
    } else {
        property_grammar_extensions(&canonical_name)
    };
    let lightning_supports_property = !matches!(
        PropertyId::from(parser_name.as_str()),
        PropertyId::Custom(_)
    );
    let owner = if alias_parser_name.is_some() {
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
    if name.ends_with("rule-width") || name == "-webkit-text-stroke-width" {
        return Some("border-top-width");
    }
    if name.ends_with("rule-style") {
        return Some("border-top-style");
    }
    if name.ends_with("rule-color") || name == "-webkit-text-stroke-color" {
        return Some("border-top-color");
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
        canonical_property_name, initial_longhand_value, property_grammar, shorthand_longhands,
        PropertyGrammarExtension, PropertyGrammarOwner, CHROMIUM_BASELINE,
        INITIAL_VALUES_SOURCE_SHA256, SOURCE_SHA256,
    };

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
    fn exposes_ordered_shorthand_longhands() {
        assert_eq!(
            shorthand_longhands("overflow"),
            Some(["overflow-x", "overflow-y"].as_slice())
        );
        assert_eq!(shorthand_longhands("width"), None);
        assert_eq!(initial_longhand_value("animation-timeline"), Some("auto"));
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
