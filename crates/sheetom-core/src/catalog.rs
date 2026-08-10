mod generated {
    include!("generated/chromium_properties.rs");
}

pub use generated::{CHROMIUM_BASELINE, SOURCE_SHA256};

pub(crate) fn canonical_property_name(name: &str) -> Option<String> {
    if name.starts_with("--") {
        return Some(name.to_owned());
    }

    let lower = name.to_ascii_lowercase();
    if let Ok(index) =
        generated::PROPERTY_ALIASES.binary_search_by_key(&lower.as_str(), |(alias, _)| *alias)
    {
        return Some(generated::PROPERTY_ALIASES[index].1.to_owned());
    }
    generated::SUPPORTED_PROPERTIES
        .binary_search(&lower.as_str())
        .is_ok()
        .then_some(lower)
}

pub(crate) fn shorthand_longhands(name: &str) -> Option<&'static [&'static str]> {
    let index = generated::SHORTHAND_LONGHANDS
        .binary_search_by_key(&name, |(shorthand, _)| *shorthand)
        .ok()?;
    Some(generated::SHORTHAND_LONGHANDS[index].1)
}

#[cfg(test)]
mod tests {
    use super::{canonical_property_name, shorthand_longhands, CHROMIUM_BASELINE, SOURCE_SHA256};

    #[test]
    fn catalog_is_pinned_to_the_chromium_manifest() {
        assert_eq!(
            SOURCE_SHA256,
            "6622a8e9731e13437f56221fac0def5e5c0fe75ceb190e7b5194ffb1e64983c1"
        );
        assert!(CHROMIUM_BASELINE.contains("Chrome/151."));
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
    }

    #[test]
    fn exposes_ordered_shorthand_longhands() {
        assert_eq!(
            shorthand_longhands("overflow"),
            Some(["overflow-x", "overflow-y"].as_slice())
        );
        assert_eq!(shorthand_longhands("width"), None);
    }
}
