use std::sync::Arc;

use crate::{SemanticDeclaration, SemanticPropertyValue};

/// The owned value of one CSS declaration.
///
/// The canonical serialization is a cache derived when the value is created. An
/// observable override is kept only when CSSOM exposes different text from the
/// canonical, reparsable serialization. This keeps the semantic value as the
/// authority instead of maintaining two freely mutable strings.
#[derive(Clone, Debug, PartialEq)]
pub struct DeclarationValue {
    storage: DeclarationValueStorage,
    canonical: Arc<str>,
    observable_override: Option<Arc<str>>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DeclarationValueKind {
    Semantic,
    CssWideKeyword,
    Codec,
    Deferred,
}

#[derive(Clone, Debug, PartialEq)]
enum DeclarationValueStorage {
    Semantic(Arc<SemanticDeclaration>),
    CssWideKeyword,
    /// A canonical value produced by a shorthand or descriptor codec that has
    /// not yet moved to a dedicated semantic AST.
    Codec,
    /// An expanded shorthand member whose value is intentionally unobservable.
    Deferred {
        pending_substitution: bool,
    },
}

impl DeclarationValue {
    pub(crate) fn semantic(declaration: SemanticDeclaration) -> Result<Self, crate::EngineError> {
        let projection = crate::observable::project_declaration(&declaration)?;
        Ok(Self::semantic_with_canonical(
            declaration,
            projection.canonical,
            projection.observable,
        ))
    }

    pub(crate) fn semantic_with_canonical(
        mut declaration: SemanticDeclaration,
        canonical: String,
        observable: String,
    ) -> Self {
        declaration.compact_recovery();
        Self::with_storage(
            DeclarationValueStorage::Semantic(Arc::new(declaration)),
            canonical,
            observable,
        )
    }

    pub(crate) fn css_wide(keyword: String) -> Self {
        Self::with_storage(
            DeclarationValueStorage::CssWideKeyword,
            keyword.clone(),
            keyword,
        )
    }

    pub(crate) fn codec(canonical: String, observable: String) -> Self {
        Self::with_storage(DeclarationValueStorage::Codec, canonical, observable)
    }

    pub(crate) fn deferred(pending_substitution: bool) -> Self {
        Self {
            storage: DeclarationValueStorage::Deferred {
                pending_substitution,
            },
            canonical: Arc::from(""),
            observable_override: None,
        }
    }

    fn with_storage(
        storage: DeclarationValueStorage,
        canonical: String,
        observable: String,
    ) -> Self {
        let canonical: Arc<str> = Arc::from(canonical);
        let observable_override =
            (observable.as_str() != canonical.as_ref()).then(|| Arc::<str>::from(observable));
        Self {
            storage,
            canonical,
            observable_override,
        }
    }

    pub fn safe_css(&self) -> &str {
        &self.canonical
    }

    pub fn observable_css(&self) -> &str {
        self.observable_override
            .as_deref()
            .unwrap_or(&self.canonical)
    }

    pub fn semantic_value(&self) -> Option<&SemanticDeclaration> {
        match &self.storage {
            DeclarationValueStorage::Semantic(declaration) => Some(declaration),
            _ => None,
        }
    }

    pub fn kind(&self) -> DeclarationValueKind {
        match &self.storage {
            DeclarationValueStorage::Semantic(_) => DeclarationValueKind::Semantic,
            DeclarationValueStorage::CssWideKeyword => DeclarationValueKind::CssWideKeyword,
            DeclarationValueStorage::Codec => DeclarationValueKind::Codec,
            DeclarationValueStorage::Deferred { .. } => DeclarationValueKind::Deferred,
        }
    }

    pub fn is_pending_substitution(&self) -> bool {
        match &self.storage {
            DeclarationValueStorage::Semantic(declaration) => matches!(
                declaration.value(),
                SemanticPropertyValue::PendingSubstitution(_)
            ),
            DeclarationValueStorage::Deferred {
                pending_substitution,
            } => *pending_substitution,
            _ => false,
        }
    }

    pub(crate) fn replace_observable(&mut self, observable: String) {
        self.observable_override =
            (observable.as_str() != self.canonical.as_ref()).then(|| Arc::<str>::from(observable));
    }
}

#[cfg(test)]
mod tests {
    use super::DeclarationValue;
    use crate::{parse_semantic_property, SemanticPropertyValue};

    #[test]
    fn semantic_value_is_the_owned_authority_for_both_projections() {
        let semantic = parse_semantic_property("width", "calc(1px + 2px)").unwrap();
        let value = DeclarationValue::semantic(semantic).unwrap();

        assert_eq!(value.safe_css(), "3px");
        assert_eq!(value.observable_css(), "calc(3px)");
        assert!(matches!(
            value.semantic_value().unwrap().value(),
            SemanticPropertyValue::Standard(_)
        ));
    }

    #[test]
    fn observable_override_does_not_replace_the_canonical_value() {
        let semantic = parse_semantic_property("color", "white").unwrap();
        let value = DeclarationValue::semantic(semantic).unwrap();

        assert_eq!(value.safe_css(), "#fff");
        assert_eq!(value.observable_css(), "white");
    }
}
