use crate::{
    catalog::{canonical_property_name, initial_longhand_value, shorthand_longhands},
    inspect_property, EngineError, PropertyParseKind,
};
use lightningcss::{
    declaration::DeclarationBlock,
    properties::{Property, PropertyId},
    stylesheet::{ParserOptions, PrinterOptions},
};

#[derive(Clone, Debug, PartialEq)]
pub struct DeclarationRecord {
    pub name: String,
    pub observable_value: String,
    pub safe_value: String,
    pub important: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MutationOutcome {
    Applied,
    InvalidName,
    InvalidPriority,
    InvalidValue,
    UnsupportedShorthand,
}

#[derive(Debug, Default, PartialEq)]
pub struct DeclarationState {
    records: Vec<DeclarationRecord>,
}

struct ParsedValue {
    observable_value: String,
    safe_value: String,
    longhands: Option<Vec<DeclarationRecord>>,
}

impl DeclarationState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.records.len()
    }

    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    pub fn item(&self, index: usize) -> &str {
        self.records
            .get(index)
            .map_or("", |record| record.name.as_str())
    }

    pub fn records(&self) -> &[DeclarationRecord] {
        &self.records
    }

    pub fn get_property_value(&self, name: &str) -> String {
        let Some(name) = canonical_property_name(name) else {
            return String::new();
        };

        if shorthand_longhands(&name).is_some() {
            return self.synthesize_shorthand(&name).unwrap_or_default();
        }

        self.records
            .iter()
            .find(|record| record.name == name)
            .map_or_else(String::new, |record| record.observable_value.clone())
    }

    pub fn get_property_priority(&self, name: &str) -> &'static str {
        let Some(name) = canonical_property_name(name) else {
            return "";
        };

        if let Some(longhands) = shorthand_longhands(&name) {
            let records = longhands
                .iter()
                .map(|longhand| self.find(longhand))
                .collect::<Option<Vec<_>>>();
            let Some(records) = records else {
                return "";
            };
            let Some(first) = records.first() else {
                return "";
            };
            return if first.important && records.iter().all(|record| record.important) {
                "important"
            } else {
                ""
            };
        }

        if self.find(&name).is_some_and(|record| record.important) {
            "important"
        } else {
            ""
        }
    }

    pub fn set_property(&mut self, name: &str, value: &str, priority: &str) -> MutationOutcome {
        let Some(name) = canonical_property_name(name) else {
            return MutationOutcome::InvalidName;
        };
        let priority = priority.to_ascii_lowercase();
        if !matches!(priority.as_str(), "" | "important") {
            return MutationOutcome::InvalidPriority;
        }
        if value.is_empty() {
            self.remove_property(&name);
            return MutationOutcome::Applied;
        }

        let important = priority == "important";
        let parsed = match parse_value(&name, value, important) {
            Ok(parsed) => parsed,
            Err(MutationOutcome::InvalidValue) => return MutationOutcome::InvalidValue,
            Err(MutationOutcome::UnsupportedShorthand) => {
                return MutationOutcome::UnsupportedShorthand;
            }
            Err(outcome) => return outcome,
        };

        if let Some(longhands) = parsed.longhands {
            for record in longhands {
                self.commit(record);
            }
            return MutationOutcome::Applied;
        }

        self.commit(DeclarationRecord {
            name,
            observable_value: parsed.observable_value,
            safe_value: parsed.safe_value,
            important,
        });
        MutationOutcome::Applied
    }

    pub fn remove_property(&mut self, name: &str) -> String {
        let Some(name) = canonical_property_name(name) else {
            return String::new();
        };
        let previous = self.get_property_value(&name);
        if let Some(longhands) = shorthand_longhands(&name) {
            self.records
                .retain(|record| !longhands.contains(&record.name.as_str()));
            return previous;
        }

        self.records.retain(|record| record.name != name);
        previous
    }

    pub fn serialize_longhands(&self) -> String {
        self.records
            .iter()
            .map(|record| {
                let suffix = if record.important { " !important" } else { "" };
                format!("{}: {}{};", record.name, record.safe_value, suffix)
            })
            .collect::<Vec<_>>()
            .join(" ")
    }

    fn find(&self, name: &str) -> Option<&DeclarationRecord> {
        self.records.iter().find(|record| record.name == name)
    }

    fn commit(&mut self, record: DeclarationRecord) {
        if let Some(existing) = self
            .records
            .iter_mut()
            .find(|existing| existing.name == record.name)
        {
            *existing = record;
            return;
        }
        self.records.push(record);
    }

    fn synthesize_shorthand(&self, name: &str) -> Option<String> {
        let longhands = shorthand_longhands(name)?;
        let records = longhands
            .iter()
            .map(|longhand| self.find(longhand))
            .collect::<Option<Vec<_>>>()?;
        let first = records.first()?;
        if records
            .iter()
            .any(|record| record.important != first.important)
        {
            return None;
        }

        let mut declarations = DeclarationBlock::new();
        for record in records {
            let property = parse_typed_property(&record.name, &record.safe_value).ok()?;
            declarations.set(property, record.important);
        }
        let shorthand = PropertyId::from(name);
        let (property, _) = declarations.get(&shorthand)?;
        property.value_to_css_string(PrinterOptions::default()).ok()
    }
}

fn parse_value(name: &str, value: &str, important: bool) -> Result<ParsedValue, MutationOutcome> {
    if name.starts_with("--") {
        let property =
            Property::parse_string(PropertyId::from(name), value, ParserOptions::default())
                .map_err(|_| MutationOutcome::InvalidValue)?;
        let safe_value = property
            .value_to_css_string(PrinterOptions::default())
            .map_err(|_| MutationOutcome::InvalidValue)?;
        return Ok(ParsedValue {
            observable_value: value.trim().to_owned(),
            safe_value,
            longhands: None,
        });
    }

    let inspection = inspect_property(name, value).map_err(map_engine_error)?;
    if !matches!(
        inspection.kind,
        PropertyParseKind::Typed | PropertyParseKind::SheetomTyped
    ) {
        return Err(MutationOutcome::InvalidValue);
    }

    let Some(longhand_names) = shorthand_longhands(name) else {
        return Ok(ParsedValue {
            observable_value: inspection.canonical_value.clone(),
            safe_value: inspection.canonical_value,
            longhands: None,
        });
    };

    let property = parse_typed_property(name, value).map_err(|_| MutationOutcome::InvalidValue)?;
    let mut longhands = Vec::with_capacity(longhand_names.len());
    for longhand_name in longhand_names {
        let longhand_id = PropertyId::from(*longhand_name);
        let canonical_value = if let Some(longhand) = property.longhand(&longhand_id) {
            longhand
                .value_to_css_string(PrinterOptions::default())
                .map_err(|_| MutationOutcome::InvalidValue)?
        } else if let Some(initial_value) = initial_longhand_value(longhand_name) {
            initial_value.to_owned()
        } else {
            return Err(MutationOutcome::UnsupportedShorthand);
        };
        longhands.push(DeclarationRecord {
            name: (*longhand_name).to_owned(),
            observable_value: canonical_value.clone(),
            safe_value: canonical_value,
            important,
        });
    }

    Ok(ParsedValue {
        observable_value: inspection.canonical_value.clone(),
        safe_value: inspection.canonical_value,
        longhands: Some(longhands),
    })
}

fn parse_typed_property<'i>(name: &'i str, value: &'i str) -> Result<Property<'i>, EngineError> {
    if matches!(name, "row-rule" | "rule") {
        return Property::parse_string(PropertyId::from("border"), value, ParserOptions::default())
            .map_err(|error| EngineError::Parse(error.to_string()));
    }
    Property::parse_string(PropertyId::from(name), value, ParserOptions::default())
        .map_err(|error| EngineError::Parse(error.to_string()))
}

fn map_engine_error(_: EngineError) -> MutationOutcome {
    MutationOutcome::InvalidValue
}

#[cfg(test)]
mod tests {
    use super::{DeclarationState, MutationOutcome};

    #[test]
    fn preserves_order_and_updates_existing_longhands_in_place() {
        let mut state = DeclarationState::new();
        assert_eq!(
            state.set_property("color", "red", ""),
            MutationOutcome::Applied
        );
        assert_eq!(
            state.set_property("WIDTH", "10px", "important"),
            MutationOutcome::Applied
        );
        assert_eq!(state.item(0), "color");
        assert_eq!(state.item(1), "width");

        state.set_property("color", "blue", "important");
        assert_eq!(state.item(0), "color");
        assert_eq!(state.get_property_value("color"), "#00f");
        assert_eq!(state.get_property_priority("color"), "important");
    }

    #[test]
    fn rejects_invalid_mutations_atomically() {
        let mut state = DeclarationState::new();
        state.set_property("width", "10px", "");
        assert_eq!(
            state.set_property("width", "20px; color: red", ""),
            MutationOutcome::InvalidValue
        );
        assert_eq!(
            state.set_property("width", "20px", "urgent"),
            MutationOutcome::InvalidPriority
        );
        assert_eq!(state.get_property_value("width"), "10px");
        assert_eq!(state.len(), 1);
    }

    #[test]
    fn expands_synthesizes_and_removes_shorthands() {
        let mut state = DeclarationState::new();
        assert_eq!(
            state.set_property("overflow", "hidden auto", ""),
            MutationOutcome::Applied
        );
        assert_eq!(state.len(), 2);
        assert_eq!(state.item(0), "overflow-x");
        assert_eq!(state.item(1), "overflow-y");
        assert_eq!(state.get_property_value("overflow"), "hidden auto");

        state.set_property("overflow-x", "scroll", "");
        assert_eq!(state.get_property_value("overflow"), "scroll auto");
        assert_eq!(state.remove_property("overflow"), "scroll auto");
        assert!(state.is_empty());
    }

    #[test]
    fn ordinary_names_are_case_insensitive_and_custom_names_are_not() {
        let mut state = DeclarationState::new();
        state.set_property("--Theme", " red ", "");
        state.set_property("--theme", "blue", "");
        assert_eq!(state.get_property_value("--Theme"), "red");
        assert_eq!(state.get_property_value("--theme"), "blue");
        assert_eq!(state.len(), 2);
    }

    #[test]
    fn empty_value_removes_after_priority_validation() {
        let mut state = DeclarationState::new();
        state.set_property("width", "10px", "");
        assert_eq!(
            state.set_property("width", "", "bogus"),
            MutationOutcome::InvalidPriority
        );
        assert_eq!(state.get_property_value("width"), "10px");
        assert_eq!(
            state.set_property("width", "", ""),
            MutationOutcome::Applied
        );
        assert!(state.is_empty());
    }

    #[test]
    fn typed_shorthands_reset_every_chromium_longhand() {
        let cases = [
            ("animation", "1s ease slide", 11),
            ("border", "2px dashed blue", 17),
            ("font", "italic 700 16px / 1.5 serif", 19),
        ];

        for (property, input, expected_length) in cases {
            let mut state = DeclarationState::new();
            assert_eq!(
                state.set_property(property, input, ""),
                MutationOutcome::Applied,
                "{property} should expand"
            );
            assert_eq!(state.len(), expected_length, "{property} longhand count");
        }

        let mut animation = DeclarationState::new();
        animation.set_property("animation", "1s ease slide", "");
        assert_eq!(animation.get_property_value("animation-timeline"), "auto");
        assert_eq!(
            animation.get_property_value("animation-range-start"),
            "normal"
        );

        let mut border = DeclarationState::new();
        border.set_property("border", "2px dashed blue", "");
        assert_eq!(border.get_property_value("border-image-source"), "none");

        let mut font = DeclarationState::new();
        font.set_property("font", "italic 700 16px / 1.5 serif", "");
        assert_eq!(font.get_property_value("font-kerning"), "auto");
    }
}
