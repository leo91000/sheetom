use crate::{
    catalog::{canonical_property_name, shorthand_longhands},
    shorthand::{parse_value, synthesize_shorthand},
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
            return synthesize_shorthand(&self.records, &name).unwrap_or_default();
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
}

#[cfg(test)]
mod tests {
    use super::{DeclarationState, MutationOutcome};
    use serde_json::Value;

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

    #[test]
    fn sheetom_border_codecs_expand_arbitrary_valid_components() {
        let mut column_rule = DeclarationState::new();
        assert_eq!(
            column_rule.set_property("column-rule", "2px dashed red", ""),
            MutationOutcome::Applied
        );
        assert_eq!(column_rule.get_property_value("column-rule-width"), "2px");
        assert_eq!(
            column_rule.get_property_value("column-rule-style"),
            "dashed"
        );
        assert_eq!(column_rule.get_property_value("column-rule-color"), "red");

        let mut legacy_border = DeclarationState::new();
        assert_eq!(
            legacy_border.set_property("-webkit-border-after", "thick double blue", ""),
            MutationOutcome::Applied
        );
        assert_eq!(
            legacy_border.get_property_value("-webkit-border-after-width"),
            "thick"
        );
        assert_eq!(
            legacy_border.get_property_value("-webkit-border-after-style"),
            "double"
        );
        assert_eq!(
            legacy_border.get_property_value("-webkit-border-after-color"),
            "blue"
        );

        let mut text_stroke = DeclarationState::new();
        assert_eq!(
            text_stroke.set_property("-webkit-text-stroke", "1px green", ""),
            MutationOutcome::Applied
        );
        assert_eq!(
            text_stroke.get_property_value("-webkit-text-stroke-width"),
            "1px"
        );
        assert_eq!(
            text_stroke.get_property_value("-webkit-text-stroke-color"),
            "green"
        );
    }

    #[test]
    fn structural_codecs_expand_by_validated_cardinality() {
        let mut state = DeclarationState::new();
        assert_eq!(
            state.set_property("overscroll-behavior", "contain none", ""),
            MutationOutcome::Applied
        );
        assert_eq!(state.get_property_value("overscroll-behavior-x"), "contain");
        assert_eq!(state.get_property_value("overscroll-behavior-y"), "none");

        assert_eq!(
            state.set_property("corner-shape", "round bevel scoop notch", ""),
            MutationOutcome::Applied
        );
        assert_eq!(state.get_property_value("corner-top-left-shape"), "round");
        assert_eq!(state.get_property_value("corner-top-right-shape"), "bevel");
        assert_eq!(
            state.get_property_value("corner-bottom-right-shape"),
            "scoop"
        );
        assert_eq!(
            state.get_property_value("corner-bottom-left-shape"),
            "notch"
        );

        assert_eq!(
            state.set_property("rule-color", "red blue", ""),
            MutationOutcome::Applied
        );
        assert_eq!(state.get_property_value("column-rule-color"), "red");
        assert_eq!(state.get_property_value("row-rule-color"), "blue");

        assert_eq!(
            state.set_property("contain-intrinsic-size", "none", ""),
            MutationOutcome::Applied
        );
        assert_eq!(state.get_property_value("contain-intrinsic-width"), "none");
        assert_eq!(state.get_property_value("contain-intrinsic-height"), "none");
    }

    #[test]
    fn structural_codecs_reject_invalid_neighbors_atomically() {
        let mut state = DeclarationState::new();
        state.set_property("overscroll-behavior", "contain none", "");
        assert_eq!(
            state.set_property("overscroll-behavior", "contain none auto", ""),
            MutationOutcome::InvalidValue
        );
        assert_eq!(state.get_property_value("overscroll-behavior-x"), "contain");
        assert_eq!(state.get_property_value("overscroll-behavior-y"), "none");

        state.set_property("corner-shape", "round bevel scoop notch", "");
        assert_eq!(
            state.set_property("corner-shape", "round bevel scoop notch square", ""),
            MutationOutcome::InvalidValue
        );
        assert_eq!(state.get_property_value("corner-top-left-shape"), "round");
        assert_eq!(
            state.get_property_value("corner-bottom-left-shape"),
            "notch"
        );
    }

    #[test]
    fn every_chromium_shorthand_capability_expands() {
        let fixture: Value = serde_json::from_str(include_str!(
            "../../../compatibility/shorthand-capabilities.json"
        ))
        .expect("the checked-in Chromium shorthand corpus should be valid JSON");
        let cases = fixture["cases"]
            .as_array()
            .expect("the shorthand corpus should contain cases");
        let mut failures = Vec::new();
        for case in cases {
            let property = case["property"].as_str().unwrap_or_default();
            let input = case["input"].as_str().unwrap_or_default();
            let mut state = DeclarationState::new();
            let outcome = state.set_property(property, input, "");
            if outcome != MutationOutcome::Applied {
                failures.push(format!("{property}: {input} ({outcome:?})"));
                continue;
            }
            let expected = case["chromium"]["longhands"]
                .as_array()
                .expect("every capability should contain Chromium longhands");
            for (actual, expected) in state.records().iter().zip(expected) {
                let expected_name = expected["name"].as_str().unwrap_or_default();
                let expected_value = expected["value"].as_str().unwrap_or_default();
                if actual.name != expected_name || actual.observable_value != expected_value {
                    failures.push(format!(
                        "{property}: expected {expected_name}: {expected_value}, got {}: {}",
                        actual.name, actual.observable_value
                    ));
                }
            }
        }
        assert!(
            failures.is_empty(),
            "Chromium shorthands that did not expand:\n{}",
            failures.join("\n")
        );
    }

    #[test]
    fn every_reviewed_shorthand_grammar_branch_matches_chromium() {
        let fixture: Value = serde_json::from_str(include_str!(
            "../../../compatibility/shorthand-grammar-contracts.json"
        ))
        .expect("the checked-in shorthand grammar contracts should be valid JSON");
        let profiles = fixture["profiles"]
            .as_array()
            .expect("the grammar contracts should contain profiles");
        let observations: Value = serde_json::from_str(include_str!(
            "../../../compatibility/shorthand-grammar-observations.json"
        ))
        .expect("the checked-in Chromium grammar observations should be valid JSON");
        let observations = observations["cases"]
            .as_array()
            .expect("the Chromium grammar observations should contain cases");
        let contract_cases = profiles
            .iter()
            .flat_map(|profile| profile["cases"].as_array().into_iter().flatten())
            .collect::<Vec<_>>();
        let mut failures = Vec::new();
        for case in &contract_cases {
            let id = case["id"].as_str().unwrap_or("missing-id");
            let property = case["property"].as_str().unwrap_or_default();
            let input = case["input"].as_str().unwrap_or_default();
            let expected = case["accepted"].as_bool().unwrap_or(false);
            let mut state = DeclarationState::new();
            let previous = if let Some(preserves) = case["preserves"].as_str() {
                let Some(reference) = contract_cases
                    .iter()
                    .find(|candidate| candidate["id"].as_str() == Some(preserves))
                else {
                    failures.push(format!("{id}: missing preserved case {preserves}"));
                    continue;
                };
                let reference_property = reference["property"].as_str().unwrap_or_default();
                let reference_input = reference["input"].as_str().unwrap_or_default();
                if state.set_property(reference_property, reference_input, "")
                    != MutationOutcome::Applied
                {
                    failures.push(format!("{id}: preserved case {preserves} was not accepted"));
                    continue;
                }
                Some(state.records().to_vec())
            } else {
                None
            };
            let outcome = state.set_property(property, input, "");
            let accepted = outcome == MutationOutcome::Applied;
            if accepted != expected {
                failures.push(format!(
                    "{id}: expected accepted={expected}, got {outcome:?}"
                ));
                continue;
            }
            if !expected {
                if previous
                    .as_deref()
                    .is_some_and(|records| records != state.records())
                {
                    failures.push(format!("{id}: invalid mutation changed declaration state"));
                }
                continue;
            }
            let Some(observation) = observations
                .iter()
                .find(|observation| observation["id"].as_str() == Some(id))
            else {
                failures.push(format!("{id}: missing Chromium observation"));
                continue;
            };
            let expected_longhands = observation["longhands"]
                .as_array()
                .expect("accepted observations should contain longhands");
            if state.len() != expected_longhands.len() {
                failures.push(format!(
                    "{id}: expected {} longhands, got {}",
                    expected_longhands.len(),
                    state.len()
                ));
                continue;
            }
            for (actual, expected) in state.records().iter().zip(expected_longhands) {
                let expected_name = expected["name"].as_str().unwrap_or_default();
                let expected_value = expected["value"].as_str().unwrap_or_default();
                if actual.name != expected_name || actual.observable_value != expected_value {
                    failures.push(format!(
                        "{id}: expected {expected_name}: {expected_value}, got {}: {}",
                        actual.name, actual.observable_value
                    ));
                }
            }
        }
        assert!(
            failures.is_empty(),
            "Reviewed Chromium grammar branches that diverged:\n{}",
            failures.join("\n")
        );
    }
}
