use crate::function_rule::{canonical_function_descriptor_name, parse_function_descriptor_value};
use crate::{
    catalog::{
        ascii_lowercase, canonical_property_name as canonical_style_property_name,
        property_alias_defers_pending_value, property_alias_hides_value,
        property_alias_observable_value, shorthand_longhands as style_shorthand_longhands,
        shorthand_names,
    },
    font_face::{canonical_descriptor_name, parse_descriptor_value},
    shorthand::{
        parse_value_for_source_with_limits, synthesize_authored_shorthand, synthesize_shorthand,
    },
    syntax::{append_serialized_identifier, parse_declaration_list},
    validate_declaration_block_input, validate_declaration_value_input, DeclarationValue,
    EngineError, ResourceLimits,
};
use smallvec::{smallvec, SmallVec};
use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
    sync::Arc,
};

#[derive(Clone, Debug, PartialEq)]
pub struct PendingSubstitutionGroup {
    pub(crate) id: u64,
    pub(crate) shorthand: String,
    pub(crate) value: DeclarationValue,
    pub(crate) important: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DeclarationRecord {
    pub name: String,
    pub value: DeclarationValue,
    pub important: bool,
    pub pending_group: Option<Arc<PendingSubstitutionGroup>>,
    pub(crate) alias_value: Option<AliasDeclarationValue>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct AliasDeclarationValue {
    name: String,
    value: DeclarationValue,
}

impl DeclarationRecord {
    pub fn observable_value(&self) -> &str {
        self.value.observable_css()
    }

    pub fn safe_value(&self) -> &str {
        self.value.safe_css()
    }

    pub fn pending_substitution(&self) -> bool {
        self.value.is_pending_substitution()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ParsedDeclaration {
    pub name: String,
    pub value: String,
    pub important: bool,
}

#[derive(Clone, Copy)]
struct DeclarationInput<'a> {
    name: &'a str,
    value: &'a str,
    important: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SerializationIssue {
    pub shorthand: String,
    pub conflicting_longhands: Vec<String>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MutationOutcome {
    Applied,
    InvalidName,
    InvalidPriority,
    InvalidValue,
    UnsupportedShorthand,
}

#[derive(Clone, Debug, PartialEq)]
pub enum DeclarationMutation {
    Set {
        property: String,
        value: String,
        priority: String,
    },
    Remove {
        property: String,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub enum DeclarationMutationResult {
    Set(MutationOutcome),
    Remove(String),
}

#[derive(Clone, Copy, Debug, Default, Hash, PartialEq)]
pub enum DeclarationContext {
    #[default]
    Style,
    FontFace,
    Function,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct DeclarationState {
    records: Vec<DeclarationRecord>,
    next_pending_group_id: u64,
    context: DeclarationContext,
    limits: ResourceLimits,
}

#[derive(Clone)]
struct ShorthandSerializationCandidate {
    name: String,
    value: String,
    important: bool,
    longhands: &'static [&'static str],
}

#[derive(Clone)]
struct PendingShorthandSerializationCandidate {
    id: u64,
    anchor: usize,
    name: String,
    value: String,
    important: bool,
    longhands: &'static [&'static str],
}

struct PendingShorthandSerializationPlan {
    candidates: Vec<PendingShorthandSerializationCandidate>,
    promoted_longhands: HashSet<String>,
    issues: Vec<SerializationIssue>,
}

impl DeclarationState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn new_with_context(context: DeclarationContext) -> Self {
        Self::new_with_context_and_limits(context, ResourceLimits::default())
    }

    pub fn new_with_context_and_limits(
        context: DeclarationContext,
        limits: ResourceLimits,
    ) -> Self {
        Self {
            context,
            limits,
            ..Self::default()
        }
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
        let hides_semantic_value =
            self.context == DeclarationContext::Style && property_alias_hides_value(name);
        let queried_name = ascii_lowercase(name);
        let Some(name) = self.canonical_name(name) else {
            return String::new();
        };

        if self.shorthand_longhands(&name).is_some() {
            return synthesize_shorthand(&self.records, &name, false).unwrap_or_default();
        }

        self.records
            .iter()
            .find(|record| record.name == name)
            .map_or_else(String::new, |record| {
                if let Some(alias) = record
                    .alias_value
                    .as_ref()
                    .filter(|alias| alias.name == queried_name)
                {
                    return alias.value.observable_css().to_owned();
                }
                if hides_semantic_value
                    && record.value.kind() != crate::DeclarationValueKind::CssWideKeyword
                {
                    return String::new();
                }
                let observable = record.observable_value();
                if let Some(projected) = property_alias_observable_value(&queried_name, observable)
                {
                    return projected.unwrap_or_default().to_owned();
                }
                observable.to_owned()
            })
    }

    pub fn get_property_priority(&self, name: &str) -> &'static str {
        let Some(name) = self.canonical_name(name) else {
            return "";
        };

        if let Some(longhands) = self.shorthand_longhands(&name) {
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
        self.set_property_checked(name, value, priority)
            .unwrap_or(MutationOutcome::InvalidValue)
    }

    pub fn set_property_checked(
        &mut self,
        name: &str,
        value: &str,
        priority: &str,
    ) -> Result<MutationOutcome, EngineError> {
        self.set_property_checked_with_reserved_depth(name, value, priority, 0)
    }

    pub fn set_property_checked_with_reserved_depth(
        &mut self,
        name: &str,
        value: &str,
        priority: &str,
        reserved_depth: usize,
    ) -> Result<MutationOutcome, EngineError> {
        let parse_limits = self.limits_with_reserved_depth(reserved_depth)?;
        validate_declaration_value_input(value, parse_limits)?;
        self.apply_property_checked(name, value, priority, parse_limits)
    }

    pub fn apply_mutations_checked_with_reserved_depth(
        &mut self,
        mutations: Vec<DeclarationMutation>,
        reserved_depth: usize,
    ) -> Result<Vec<DeclarationMutationResult>, EngineError> {
        let mut results = Vec::with_capacity(mutations.len());
        for mutation in mutations {
            match mutation {
                DeclarationMutation::Set {
                    property,
                    value,
                    priority,
                } => {
                    let outcome = self.set_property_checked_with_reserved_depth(
                        &property,
                        &value,
                        &priority,
                        reserved_depth,
                    )?;
                    results.push(DeclarationMutationResult::Set(outcome));
                }
                DeclarationMutation::Remove { property } => {
                    results.push(DeclarationMutationResult::Remove(
                        self.remove_property(&property),
                    ));
                }
            }
        }
        Ok(results)
    }

    fn apply_property_checked(
        &mut self,
        name: &str,
        value: &str,
        priority: &str,
        parse_limits: ResourceLimits,
    ) -> Result<MutationOutcome, EngineError> {
        let source_name = ascii_lowercase(name);
        let Some(name) = self.canonical_name(name) else {
            return Ok(MutationOutcome::InvalidName);
        };
        if !priority.is_empty() && !priority.eq_ignore_ascii_case("important") {
            return Ok(MutationOutcome::InvalidPriority);
        }
        if value.is_empty() {
            self.remove_property(&name);
            return Ok(MutationOutcome::Applied);
        }
        if self.context == DeclarationContext::Function && name == "result" {
            return Ok(MutationOutcome::Applied);
        }

        let important = !priority.is_empty();
        let parsed =
            match self.parse_value_with_limits(&name, &source_name, value, important, parse_limits)
            {
                Ok(parsed) => parsed,
                Err(outcome) => return Ok(outcome),
            };
        let previous_group_id = self.next_pending_group_id;
        let records = self.records_for_parsed(name.into_owned(), parsed, important, &source_name);
        let additional_records = records
            .iter()
            .filter(|record| self.find(&record.name).is_none())
            .map(|record| record.name.as_str())
            .collect::<HashSet<_>>()
            .len();
        let projected_len = self.records.len() + additional_records;
        if projected_len > self.limits.max_declarations_per_block {
            self.next_pending_group_id = previous_group_id;
            return Err(EngineError::DeclarationLimitExceeded {
                actual: projected_len,
                limit: self.limits.max_declarations_per_block,
            });
        }
        for record in records {
            self.commit(record);
        }
        Ok(MutationOutcome::Applied)
    }

    pub fn replace_declarations(&mut self, declarations: &[ParsedDeclaration]) {
        self.replace_declaration_inputs(declarations.iter().map(|declaration| DeclarationInput {
            name: &declaration.name,
            value: &declaration.value,
            important: declaration.important,
        }));
    }

    fn replace_declaration_inputs<'a>(
        &mut self,
        declarations: impl IntoIterator<Item = DeclarationInput<'a>>,
    ) {
        let mut winners = HashMap::<String, (DeclarationRecord, usize, usize)>::new();

        for (source_index, declaration) in declarations.into_iter().enumerate() {
            if self.context == DeclarationContext::Function && declaration.important {
                continue;
            }
            let Some(name) = self.canonical_name(declaration.name) else {
                continue;
            };
            let source_name = ascii_lowercase(declaration.name);
            let Ok(parsed) = self.parse_value(
                &name,
                &source_name,
                declaration.value,
                declaration.important,
            ) else {
                continue;
            };
            let records = self.records_for_parsed(
                name.into_owned(),
                parsed,
                declaration.important,
                &source_name,
            );
            for (sub_index, record) in records.into_iter().enumerate() {
                if winners
                    .get(&record.name)
                    .is_some_and(|(current, _, _)| current.important && !record.important)
                {
                    continue;
                }
                winners.insert(record.name.clone(), (record, source_index, sub_index));
            }
        }

        let mut records = winners.into_values().collect::<Vec<_>>();
        records.sort_unstable_by_key(|(record, source_index, sub_index)| {
            (record.important, *source_index, *sub_index)
        });
        self.records = records.into_iter().map(|(record, _, _)| record).collect();
    }

    pub fn replace_css_text(&mut self, source: &str) {
        let _ = self.replace_css_text_checked(source);
    }

    pub fn replace_css_text_checked(&mut self, source: &str) -> Result<(), EngineError> {
        self.replace_css_text_checked_with_reserved_depth(source, 0)
    }

    pub fn replace_css_text_checked_with_reserved_depth(
        &mut self,
        source: &str,
        reserved_depth: usize,
    ) -> Result<(), EngineError> {
        let parse_limits = self.limits_with_reserved_depth(reserved_depth)?;
        validate_declaration_block_input(source, parse_limits)?;
        let source_declarations = parse_declaration_list(source);
        for declaration in &source_declarations {
            validate_declaration_value_input(declaration.value, parse_limits)?;
        }
        let declarations = source_declarations
            .iter()
            .map(|declaration| DeclarationInput {
                name: &declaration.name,
                value: declaration.value,
                important: declaration.important,
            });
        let mut candidate = Self {
            records: Vec::new(),
            next_pending_group_id: self.next_pending_group_id,
            context: self.context,
            limits: parse_limits,
        };
        candidate.replace_declaration_inputs(declarations);
        candidate.validate_record_limit()?;
        candidate.limits = self.limits;
        *self = candidate;
        Ok(())
    }

    fn limits_with_reserved_depth(
        &self,
        reserved_depth: usize,
    ) -> Result<ResourceLimits, EngineError> {
        if reserved_depth > self.limits.max_nesting_depth {
            return Err(EngineError::NestingLimitExceeded {
                actual: reserved_depth,
                limit: self.limits.max_nesting_depth,
            });
        }
        Ok(ResourceLimits {
            max_nesting_depth: self.limits.max_nesting_depth - reserved_depth,
            ..self.limits
        })
    }

    pub fn clear(&mut self) {
        self.records.clear();
    }

    pub fn remove_property(&mut self, name: &str) -> String {
        let previous =
            if self.context == DeclarationContext::Style && property_alias_hides_value(name) {
                String::new()
            } else {
                self.get_property_value(name)
            };
        let Some(name) = self.canonical_name(name) else {
            return String::new();
        };
        if let Some(longhands) = self.shorthand_longhands(&name) {
            self.records
                .retain(|record| !longhands.contains(&record.name.as_str()));
            return previous;
        }

        self.break_group_for_name(&name, None);
        self.records.retain(|record| record.name != name);
        previous
    }

    pub fn serialize_longhands(&self) -> String {
        self.records
            .iter()
            .map(|record| {
                let suffix = if record.important { " !important" } else { "" };
                format!("{}: {}{};", record.name, record.safe_value(), suffix)
            })
            .collect::<Vec<_>>()
            .join(" ")
    }

    pub fn css_text(&self) -> String {
        self.serialize_observable_with_format("", " ")
    }

    pub fn serialize_safe(&self) -> Result<String, EngineError> {
        self.serialize_safe_with_format("", " ", true)
    }

    pub fn serialize_safe_resilient(
        &self,
    ) -> Result<(String, Vec<SerializationIssue>), EngineError> {
        self.serialize_safe_resilient_with_format("", " ")
    }

    pub fn serialize_formatted(
        &self,
        safe: bool,
        indent: &str,
        separator: &str,
    ) -> Result<String, EngineError> {
        if safe {
            return self.serialize_safe_with_format(indent, separator, true);
        }
        Ok(self.serialize_observable_with_format(indent, separator))
    }

    pub fn serialize_formatted_resilient(
        &self,
        safe: bool,
        indent: &str,
        separator: &str,
    ) -> Result<(String, Vec<SerializationIssue>), EngineError> {
        if safe {
            return self.serialize_safe_resilient_with_format(indent, separator);
        }
        Ok((
            self.serialize_observable_with_format(indent, separator),
            Vec::new(),
        ))
    }

    fn find(&self, name: &str) -> Option<&DeclarationRecord> {
        self.records.iter().find(|record| record.name == name)
    }

    fn canonical_name<'a>(&self, name: &'a str) -> Option<Cow<'a, str>> {
        match self.context {
            DeclarationContext::Style => canonical_style_property_name(name),
            DeclarationContext::FontFace => canonical_descriptor_name(name).map(Cow::Owned),
            DeclarationContext::Function => {
                canonical_function_descriptor_name(name).map(Cow::Owned)
            }
        }
    }

    fn shorthand_longhands(&self, name: &str) -> Option<&'static [&'static str]> {
        (self.context == DeclarationContext::Style)
            .then(|| style_shorthand_longhands(name))
            .flatten()
    }

    fn parse_value(
        &self,
        name: &str,
        source_name: &str,
        value: &str,
        important: bool,
    ) -> Result<crate::shorthand::ParsedValue, MutationOutcome> {
        self.parse_value_with_limits(name, source_name, value, important, self.limits)
    }

    fn parse_value_with_limits(
        &self,
        name: &str,
        source_name: &str,
        value: &str,
        important: bool,
        limits: ResourceLimits,
    ) -> Result<crate::shorthand::ParsedValue, MutationOutcome> {
        match self.context {
            DeclarationContext::Style => {
                parse_value_for_source_with_limits(name, source_name, value, important, limits)
            }
            DeclarationContext::FontFace => {
                parse_descriptor_value(name, value, limits).ok_or(MutationOutcome::InvalidValue)
            }
            DeclarationContext::Function => {
                parse_function_descriptor_value(name, value).ok_or(MutationOutcome::InvalidValue)
            }
        }
    }

    fn commit(&mut self, record: DeclarationRecord) {
        self.break_group_for_name(
            &record.name,
            record.pending_group.as_ref().map(|group| group.id),
        );
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

    fn validate_record_limit(&self) -> Result<(), EngineError> {
        if self.records.len() <= self.limits.max_declarations_per_block {
            return Ok(());
        }
        Err(EngineError::DeclarationLimitExceeded {
            actual: self.records.len(),
            limit: self.limits.max_declarations_per_block,
        })
    }

    fn records_for_parsed(
        &mut self,
        name: String,
        mut parsed: crate::shorthand::ParsedValue,
        important: bool,
        source_name: &str,
    ) -> SmallVec<[DeclarationRecord; 1]> {
        if parsed.pending_substitution() {
            if let Some(longhands) = self.shorthand_longhands(&name) {
                let group = self.new_pending_group(name, parsed.value.clone(), important);
                return longhands
                    .iter()
                    .map(|longhand| DeclarationRecord {
                        name: (*longhand).to_owned(),
                        value: DeclarationValue::deferred(true),
                        important,
                        pending_group: Some(group.clone()),
                        alias_value: None,
                    })
                    .collect();
            }
        }
        if let Some(mut longhands) = parsed.longhands.take() {
            self.attach_static_group(&name, &parsed, &mut longhands);
            return longhands.into();
        }
        let alias_value = self.alias_value_for_record(source_name, &name, &parsed.value);
        let value = if alias_value.is_some() {
            DeclarationValue::deferred(true)
        } else {
            parsed.value
        };
        smallvec![DeclarationRecord {
            name,
            value,
            important,
            pending_group: None,
            alias_value,
        }]
    }

    fn alias_value_for_record(
        &self,
        source_name: &str,
        canonical_name: &str,
        value: &DeclarationValue,
    ) -> Option<AliasDeclarationValue> {
        (self.context == DeclarationContext::Style
            && source_name != canonical_name
            && property_alias_defers_pending_value(source_name)
            && value.is_pending_substitution())
        .then(|| AliasDeclarationValue {
            name: source_name.to_owned(),
            value: value.clone(),
        })
    }

    fn new_pending_group(
        &mut self,
        shorthand: String,
        value: DeclarationValue,
        important: bool,
    ) -> Arc<PendingSubstitutionGroup> {
        let id = self.next_pending_group_id;
        self.next_pending_group_id = self.next_pending_group_id.wrapping_add(1);
        Arc::new(PendingSubstitutionGroup {
            id,
            shorthand,
            value,
            important,
        })
    }

    fn attach_static_group(
        &mut self,
        name: &str,
        parsed: &crate::shorthand::ParsedValue,
        records: &mut [DeclarationRecord],
    ) {
        if self
            .shorthand_longhands(name)
            .is_none_or(|longhands| longhands.len() < 2)
        {
            return;
        }
        if name == "-webkit-mask-box-image" {
            return;
        }
        let default_synthesis =
            requires_default_shorthand_synthesis(name, parsed.observable_value());
        let observable_synthesis = prefers_synthesized_provenance(name)
            || requires_observable_shorthand_synthesis(name, parsed.observable_value())
            || default_synthesis;
        let observable_value = if observable_synthesis {
            let synthesized = synthesize_authored_shorthand(records, name, false);
            if synthesized.is_none() && default_synthesis {
                return;
            }
            synthesized.unwrap_or_else(|| parsed.observable_value().to_owned())
        } else {
            parsed.observable_value().to_owned()
        };
        let safe_value = if prefers_synthesized_safe_provenance(name) {
            synthesize_authored_shorthand(records, name, true)
                .unwrap_or_else(|| parsed.safe_value().to_owned())
        } else {
            parsed.safe_value().to_owned()
        };
        let safe_value = normalize_rgb_function_spacing(safe_value);
        let group_value = parsed.value.semantic_value().map_or_else(
            || parsed.value.clone(),
            |semantic| {
                DeclarationValue::semantic_with_canonical(
                    semantic.clone(),
                    safe_value,
                    observable_value,
                )
            },
        );
        let important = records.first().is_some_and(|record| record.important);
        let group = self.new_pending_group(name.to_owned(), group_value, important);
        for record in records {
            record.pending_group = Some(group.clone());
        }
    }

    fn break_group_for_name(&mut self, name: &str, replacement_group_id: Option<u64>) {
        let Some(group_id) = self
            .records
            .iter()
            .find(|record| record.name == name)
            .and_then(|record| record.pending_group.as_ref())
            .map(|group| group.id)
        else {
            return;
        };
        if replacement_group_id == Some(group_id) {
            return;
        }
        for record in &mut self.records {
            if record
                .pending_group
                .as_ref()
                .is_some_and(|group| group.id == group_id)
            {
                // Keep deferred members linked to their authored shorthand. CSSOM getters stop
                // synthesizing the shorthand as soon as the replacement record lands, while the
                // reparsable serializer still needs the original pending value to precede an
                // equal-priority longhand override.
                if record.pending_substitution() {
                    continue;
                }
                let observable = if is_gap_rule_longhand(&record.name)
                    || record.value.observable_survives_group_break()
                {
                    record.observable_value().to_owned()
                } else {
                    materialize_static_observable(record.observable_value(), record.safe_value())
                };
                record.value.replace_observable(observable);
                record.pending_group = None;
            }
        }
    }

    fn serialize_observable_with_format(&self, indent: &str, separator: &str) -> String {
        self.serialize_declarations_with_format(false, &[], &HashSet::new(), indent, separator)
    }

    fn serialize_safe_with_format(
        &self,
        indent: &str,
        separator: &str,
        strict: bool,
    ) -> Result<String, EngineError> {
        let pending = self.pending_shorthand_serialization_plan(strict)?;
        Ok(self.serialize_declarations_with_format(
            true,
            &pending.candidates,
            &pending.promoted_longhands,
            indent,
            separator,
        ))
    }

    fn serialize_safe_resilient_with_format(
        &self,
        indent: &str,
        separator: &str,
    ) -> Result<(String, Vec<SerializationIssue>), EngineError> {
        let pending = self.pending_shorthand_serialization_plan(false)?;
        let output = self.serialize_declarations_with_format(
            true,
            &pending.candidates,
            &pending.promoted_longhands,
            indent,
            separator,
        );
        Ok((output, pending.issues))
    }

    fn pending_shorthand_serialization_plan(
        &self,
        strict: bool,
    ) -> Result<PendingShorthandSerializationPlan, EngineError> {
        let mut surviving_groups = HashMap::<u64, (Arc<PendingSubstitutionGroup>, usize)>::new();
        for (index, record) in self.records.iter().enumerate() {
            let Some(group) = record.pending_group.as_ref() else {
                continue;
            };
            if !record.pending_substitution() {
                continue;
            }
            surviving_groups
                .entry(group.id)
                .and_modify(|(_, anchor)| *anchor = (*anchor).min(index))
                .or_insert_with(|| (Arc::clone(group), index));
        }
        if surviving_groups.is_empty() {
            return Ok(PendingShorthandSerializationPlan {
                candidates: Vec::new(),
                promoted_longhands: HashSet::new(),
                issues: Vec::new(),
            });
        }

        let record_indexes_by_name = self
            .records
            .iter()
            .enumerate()
            .map(|(index, record)| (record.name.as_str(), index))
            .collect::<HashMap<_, _>>();

        let mut surviving_groups = surviving_groups.into_values().collect::<Vec<_>>();
        surviving_groups.sort_unstable_by_key(|(group, anchor)| (*anchor, group.id));

        let mut candidates = Vec::new();
        let mut promoted_longhands = HashSet::new();
        let mut issues = Vec::new();
        for (group, fallback_anchor) in surviving_groups {
            let Some(longhands) = style_shorthand_longhands(&group.shorthand) else {
                return Err(EngineError::Serialize(format!(
                    "pending shorthand provenance references unsupported property {}",
                    group.shorthand
                )));
            };
            let surviving_count = longhands
                .iter()
                .filter(|longhand| {
                    record_indexes_by_name
                        .get(**longhand)
                        .and_then(|index| self.records[*index].pending_group.as_ref())
                        .is_some_and(|candidate| candidate.id == group.id)
                })
                .count();
            if surviving_count == longhands.len() {
                continue;
            }

            let mut conflicting_longhands = Vec::new();
            let mut anchor = fallback_anchor;
            for longhand in longhands {
                let Some(index) = record_indexes_by_name.get(*longhand).copied() else {
                    conflicting_longhands.push((*longhand).to_owned());
                    continue;
                };
                anchor = anchor.min(index);
                if group.important && !self.records[index].important {
                    conflicting_longhands.push((*longhand).to_owned());
                    promoted_longhands.insert((*longhand).to_owned());
                }
            }
            if !conflicting_longhands.is_empty() {
                if strict {
                    return Err(EngineError::UnrepresentablePendingShorthand {
                        shorthand: group.shorthand.clone(),
                        conflicting_longhands,
                    });
                }
                issues.push(SerializationIssue {
                    shorthand: group.shorthand.clone(),
                    conflicting_longhands,
                });
            }
            candidates.push(PendingShorthandSerializationCandidate {
                id: group.id,
                anchor,
                name: group.shorthand.clone(),
                value: group.value.safe_css().to_owned(),
                important: group.important,
                longhands,
            });
        }
        candidates.sort_unstable_by_key(|candidate| (candidate.anchor, candidate.id));
        Ok(PendingShorthandSerializationPlan {
            candidates,
            promoted_longhands,
            issues,
        })
    }

    fn serialize_declarations_with_format(
        &self,
        safe: bool,
        pending_candidates: &[PendingShorthandSerializationCandidate],
        promoted_longhands: &HashSet<String>,
        indent: &str,
        separator: &str,
    ) -> String {
        let estimated_declaration_bytes = self
            .records
            .iter()
            .map(|record| {
                record.name.len()
                    + if safe {
                        record.safe_value().len()
                    } else {
                        record.observable_value().len()
                    }
                    + if record.important { 14 } else { 3 }
                    + indent.len()
                    + separator.len()
            })
            .sum();
        let mut output = String::with_capacity(estimated_declaration_bytes);
        let mut first = true;
        if matches!(
            self.context,
            DeclarationContext::FontFace | DeclarationContext::Function
        ) {
            for record in &self.records {
                let value = if safe
                    || self.context == DeclarationContext::FontFace && record.name == "font-variant"
                {
                    record.safe_value()
                } else {
                    record.observable_value()
                };
                append_declaration(
                    &mut output,
                    &mut first,
                    indent,
                    separator,
                    &record.name,
                    value,
                    record.important,
                );
            }
            return output;
        }

        if self.records.len() <= 1 && pending_candidates.is_empty() {
            for record in &self.records {
                append_declaration(
                    &mut output,
                    &mut first,
                    indent,
                    separator,
                    &record.name,
                    if safe {
                        record.safe_value()
                    } else {
                        record.observable_value()
                    },
                    record.important || promoted_longhands.contains(&record.name),
                );
            }
            return output;
        }

        let records_by_name = self
            .records
            .iter()
            .map(|record| (record.name.as_str(), record))
            .collect::<HashMap<_, _>>();
        let mut candidates = shorthand_names()
            .filter_map(|name| {
                if canonical_style_property_name(name).as_deref() != Some(name) {
                    return None;
                }
                let longhands = style_shorthand_longhands(name)?;
                if longhands
                    .iter()
                    .any(|longhand| !records_by_name.contains_key(longhand))
                {
                    return None;
                }
                let value = synthesize_shorthand(&self.records, name, safe)?;
                let important = records_by_name.get(longhands.first()?)?.important;
                Some(ShorthandSerializationCandidate {
                    name: name.to_owned(),
                    value,
                    important,
                    longhands,
                })
            })
            .collect::<Vec<_>>();
        candidates.sort_by_key(|candidate| {
            (
                std::cmp::Reverse(candidate.longhands.len()),
                shorthand_serialization_priority(&candidate.name),
                candidate.name.starts_with('-'),
            )
        });

        let mut claimed = HashSet::new();
        let mut by_longhand = HashMap::new();
        for (index, candidate) in candidates.iter().enumerate() {
            if candidate
                .longhands
                .iter()
                .any(|longhand| claimed.contains(longhand))
            {
                continue;
            }
            for longhand in candidate.longhands {
                claimed.insert(*longhand);
                by_longhand.insert(*longhand, index);
            }
        }

        let mut written = HashSet::new();
        let reconstructed_group_ids = pending_candidates
            .iter()
            .map(|candidate| candidate.id)
            .collect::<HashSet<_>>();
        let mut pending_candidates_by_anchor = HashMap::<usize, Vec<usize>>::new();
        for (index, candidate) in pending_candidates.iter().enumerate() {
            pending_candidates_by_anchor
                .entry(candidate.anchor)
                .or_default()
                .push(index);
        }
        for (record_index, record) in self.records.iter().enumerate() {
            if let Some(indexes) = pending_candidates_by_anchor.get(&record_index) {
                for index in indexes {
                    let candidate = &pending_candidates[*index];
                    append_declaration(
                        &mut output,
                        &mut first,
                        indent,
                        separator,
                        &candidate.name,
                        &candidate.value,
                        candidate.important
                            || candidate
                                .longhands
                                .iter()
                                .any(|longhand| promoted_longhands.contains(*longhand)),
                    );
                }
            }
            if record.pending_substitution()
                && record
                    .pending_group
                    .as_ref()
                    .is_some_and(|group| reconstructed_group_ids.contains(&group.id))
            {
                continue;
            }
            if let Some(index) = by_longhand.get(record.name.as_str()) {
                let candidate = &candidates[*index];
                if written.insert(candidate.name.as_str()) {
                    let important = candidate.important
                        || candidate
                            .longhands
                            .iter()
                            .any(|longhand| promoted_longhands.contains(*longhand));
                    append_declaration(
                        &mut output,
                        &mut first,
                        indent,
                        separator,
                        &candidate.name,
                        &candidate.value,
                        important,
                    );
                }
                continue;
            }
            let value = if safe {
                record.safe_value()
            } else {
                record.observable_value()
            };
            let important = record.important || promoted_longhands.contains(&record.name);
            append_declaration(
                &mut output,
                &mut first,
                indent,
                separator,
                &record.name,
                value,
                important,
            );
        }
        output
    }
}

fn shorthand_serialization_priority(name: &str) -> u8 {
    match name {
        "column-rule-inset" => 0,
        "rule-inset-cap" | "rule-inset-junction" => 1,
        "row-rule-inset" => 2,
        _ => 3,
    }
}

fn is_gap_rule_longhand(name: &str) -> bool {
    matches!(
        name,
        "column-rule-width"
            | "column-rule-style"
            | "column-rule-color"
            | "row-rule-width"
            | "row-rule-style"
            | "row-rule-color"
    )
}

fn prefers_synthesized_provenance(name: &str) -> bool {
    if name.contains("rule-inset") {
        return true;
    }
    matches!(
        name,
        "animation"
            | "animation-range"
            | "background"
            | "border"
            | "border-block"
            | "border-block-end"
            | "border-block-start"
            | "border-bottom"
            | "column-rule"
            | "border-image"
            | "border-inline"
            | "border-inline-end"
            | "border-inline-start"
            | "border-left"
            | "border-right"
            | "border-top"
            | "border-radius"
            | "-webkit-border-radius"
            | "columns"
            | "contain-intrinsic-size"
            | "container"
            | "flex"
            | "flex-flow"
            | "font-synthesis"
            | "font-variant"
            | "gap"
            | "grid-gap"
            | "grid-area"
            | "grid-column"
            | "grid-row"
            | "place-self"
            | "place-content"
            | "place-items"
            | "grid"
            | "grid-template"
            | "mask"
            | "mask-position"
            | "-webkit-mask-position"
            | "background-position"
            | "border-spacing"
            | "border-color"
            | "border-style"
            | "border-width"
            | "interest-delay"
            | "margin"
            | "margin-block"
            | "margin-inline"
            | "overflow"
            | "padding"
            | "padding-block"
            | "padding-inline"
            | "scroll-margin"
            | "scroll-margin-block"
            | "scroll-margin-inline"
            | "scroll-padding"
            | "scroll-padding-block"
            | "scroll-padding-inline"
            | "list-style"
            | "offset"
            | "position-try"
            | "row-rule"
            | "rule"
            | "rule-color"
            | "rule-style"
            | "rule-width"
            | "scroll-timeline"
            | "text-emphasis"
            | "text-box"
            | "text-decoration"
            | "-webkit-text-stroke"
            | "text-wrap"
            | "timeline-trigger"
            | "timeline-trigger-activation-range"
            | "timeline-trigger-active-range"
            | "transition"
            | "view-timeline"
            | "white-space"
    )
}

fn prefers_synthesized_safe_provenance(name: &str) -> bool {
    matches!(
        name,
        "animation-range"
            | "border"
            | "border-block"
            | "border-block-end"
            | "border-block-start"
            | "border-bottom"
            | "border-image"
            | "border-inline"
            | "border-inline-end"
            | "border-inline-start"
            | "border-left"
            | "border-right"
            | "border-top"
            | "contain-intrinsic-size"
            | "font-variant"
            | "flex-flow"
            | "grid-area"
            | "grid-column"
            | "grid-row"
            | "place-self"
            | "text-decoration"
            | "-webkit-text-stroke"
            | "timeline-trigger"
            | "timeline-trigger-activation-range"
            | "timeline-trigger-active-range"
    ) || matches!(
        name,
        "border-block-color" | "border-color" | "border-inline-color" | "rule-color"
    )
}

fn materialize_static_observable(observable: &str, safe: &str) -> String {
    let observable_parts = crate::syntax::split_top_level_delimiter(observable, b',');
    let safe_parts = crate::syntax::split_top_level_delimiter(safe, b',');
    let (Some(observable_parts), Some(safe_parts)) = (observable_parts, safe_parts) else {
        return if observable == "initial" {
            observable.to_owned()
        } else {
            safe.to_owned()
        };
    };
    if observable_parts.len() != safe_parts.len() {
        return safe.to_owned();
    }
    let mut materialized = String::with_capacity(safe.len());
    for (observable, safe) in observable_parts.iter().zip(safe_parts) {
        if !materialized.is_empty() {
            materialized.push_str(", ");
        }
        materialized.push_str(if *observable == "initial" {
            "initial"
        } else {
            safe
        });
    }
    materialized
}

fn normalize_rgb_function_spacing(value: String) -> String {
    let function = if value
        .get(..4)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("rgb("))
    {
        "rgb"
    } else if value
        .get(..5)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("rgba("))
    {
        "rgba"
    } else {
        return value;
    };
    let Some(body) = value
        .get(function.len() + 1..)
        .and_then(|body| body.strip_suffix(')'))
    else {
        return value;
    };
    if !body.contains(',') {
        return value;
    }
    let mut normalized = String::with_capacity(value.len());
    normalized.push_str(function);
    normalized.push('(');
    for (index, channel) in body.split(',').enumerate() {
        if index > 0 {
            normalized.push_str(", ");
        }
        normalized.push_str(channel.trim());
    }
    normalized.push(')');
    normalized
}

fn requires_default_shorthand_synthesis(name: &str, observable: &str) -> bool {
    let border_like = matches!(
        name,
        "border"
            | "border-block"
            | "border-bottom"
            | "border-inline"
            | "border-left"
            | "border-right"
            | "border-top"
            | "column-rule"
            | "row-rule"
            | "rule"
    );
    border_like && matches!(observable, "medium" | "none" | "currentcolor")
        || name == "text-decoration" && matches!(observable, "auto" | "solid" | "currentcolor")
}

fn requires_observable_shorthand_synthesis(name: &str, observable: &str) -> bool {
    if name == "font" {
        return !matches!(
            observable,
            "caption" | "icon" | "menu" | "message-box" | "small-caption" | "status-bar"
        );
    }
    if matches!(name, "column-rule-inset" | "row-rule-inset" | "rule-inset") {
        return !observable.contains('/')
            && crate::syntax::split_top_level_whitespace(observable)
                .is_some_and(|components| components.len() == 2);
    }
    if matches!(name, "inset" | "inset-block" | "inset-inline") {
        if observable.contains("anchor(") || observable.contains("anchor-size(") {
            return true;
        }
        return crate::syntax::split_top_level_whitespace(observable).is_some_and(|components| {
            components.len() > 1 && components.iter().all(|component| *component == "auto")
        });
    }
    name == "outline"
        && crate::syntax::split_top_level_whitespace(observable)
            .is_some_and(|components| components.len() > 1)
}

fn append_declaration(
    output: &mut String,
    first: &mut bool,
    indent: &str,
    separator: &str,
    name: &str,
    value: &str,
    important: bool,
) {
    if !*first {
        output.push_str(separator);
    }
    *first = false;
    output.push_str(indent);
    if name.starts_with("--") {
        append_serialized_identifier(output, name);
    } else {
        output.push_str(name);
    }
    output.push_str(": ");
    output.push_str(value);
    if important {
        output.push_str(" !important");
    }
    output.push(';');
}

#[cfg(test)]
mod tests {
    use super::{
        canonical_style_property_name, shorthand_names, style_shorthand_longhands,
        DeclarationContext, DeclarationMutation, DeclarationMutationResult, DeclarationState,
        MutationOutcome, ParsedDeclaration, SerializationIssue,
    };
    use crate::{DeclarationValueKind, EngineError, ResourceLimits};
    use serde_json::Value;
    use std::sync::Arc;

    #[test]
    fn ordered_mutation_batches_match_sequential_cssom_state() {
        let mutations = vec![
            DeclarationMutation::Set {
                property: "padding".to_owned(),
                value: "1px 2px".to_owned(),
                priority: "important".to_owned(),
            },
            DeclarationMutation::Set {
                property: "padding-left".to_owned(),
                value: "3px".to_owned(),
                priority: "important".to_owned(),
            },
            DeclarationMutation::Set {
                property: "width".to_owned(),
                value: "20px; color: red".to_owned(),
                priority: String::new(),
            },
            DeclarationMutation::Remove {
                property: "padding-right".to_owned(),
            },
        ];
        let mut batched = DeclarationState::new();
        let results = batched
            .apply_mutations_checked_with_reserved_depth(mutations, 0)
            .unwrap_or_default();

        let mut sequential = DeclarationState::new();
        sequential.set_property("padding", "1px 2px", "important");
        sequential.set_property("padding-left", "3px", "important");
        sequential.set_property("width", "20px; color: red", "");
        let removed = sequential.remove_property("padding-right");

        assert_eq!(
            results,
            vec![
                DeclarationMutationResult::Set(MutationOutcome::Applied),
                DeclarationMutationResult::Set(MutationOutcome::Applied),
                DeclarationMutationResult::Set(MutationOutcome::InvalidValue),
                DeclarationMutationResult::Remove(removed),
            ]
        );
        assert_eq!(batched, sequential);
    }

    #[test]
    fn cached_shorthand_parses_do_not_share_mutable_provenance() {
        let mut first = DeclarationState::new();
        let mut second = DeclarationState::new();
        first.set_property("background", "center / cover no-repeat red", "");
        second.set_property("background", "center / cover no-repeat red", "");

        first.set_property("background-color", "blue", "");

        assert_eq!(second.get_property_value("background-color"), "red");
        assert!(!second.get_property_value("background").is_empty());
        assert_eq!(first.get_property_value("background-color"), "blue");
    }

    #[test]
    fn batch_resource_errors_keep_prior_commits_like_sequential_calls() {
        let mut state = DeclarationState::new_with_context_and_limits(
            DeclarationContext::Style,
            ResourceLimits {
                max_declarations_per_block: 1,
                ..ResourceLimits::default()
            },
        );
        let result = state.apply_mutations_checked_with_reserved_depth(
            vec![
                DeclarationMutation::Set {
                    property: "width".to_owned(),
                    value: "1px".to_owned(),
                    priority: String::new(),
                },
                DeclarationMutation::Set {
                    property: "height".to_owned(),
                    value: "2px".to_owned(),
                    priority: String::new(),
                },
            ],
            0,
        );

        assert!(matches!(
            result,
            Err(EngineError::DeclarationLimitExceeded {
                actual: 2,
                limit: 1
            })
        ));
        assert_eq!(state.css_text(), "width: 1px;");
    }

    #[test]
    fn every_manifested_longhand_initial_value_has_semantic_state() {
        let failures = crate::catalog::initial_longhand_values()
            .filter_map(|(name, value)| {
                let mut state = DeclarationState::new();
                let outcome = state.set_property(name, value, "");
                let kind = state.records().first().map(|record| record.value.kind());
                (outcome != MutationOutcome::Applied
                    || !matches!(
                        kind,
                        Some(DeclarationValueKind::Semantic | DeclarationValueKind::CssWideKeyword)
                    ))
                .then(|| format!("{name}: {value}: outcome={outcome:?}, kind={kind:?}"))
            })
            .collect::<Vec<_>>();

        assert!(failures.is_empty(), "{}", failures.join("\n"));
    }

    #[test]
    fn preserves_order_and_updates_existing_longhands_in_place() {
        let mut state = DeclarationState::new();
        assert_eq!(
            state.set_property("color", "red", ""),
            MutationOutcome::Applied
        );
        assert_eq!(
            state.set_property("WIDTH", "10px", "IMPORTANT"),
            MutationOutcome::Applied
        );
        assert_eq!(state.item(0), "color");
        assert_eq!(state.item(1), "width");
        assert_eq!(state.get_property_value("WIDTH"), "10px");
        assert_eq!(state.get_property_priority("WIDTH"), "important");

        state.set_property("color", "blue", "important");
        assert_eq!(state.item(0), "color");
        assert_eq!(state.get_property_value("color"), "blue");
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
    fn preserves_the_write_only_value_behavior_of_legacy_column_break_aliases() {
        for (alias, canonical, invalid) in [
            ("-webkit-column-break-after", "break-after", "page"),
            ("-webkit-column-break-before", "break-before", "page"),
            ("-webkit-column-break-inside", "break-inside", "avoid-page"),
        ] {
            let mut state = DeclarationState::new();
            assert_eq!(
                state.set_property(alias, "auto", "important"),
                MutationOutcome::Applied,
                "{alias}"
            );
            assert_eq!(state.item(0), canonical, "{alias} item");
            let before = state.clone();
            assert_eq!(
                state.set_property(alias, invalid, ""),
                MutationOutcome::InvalidValue,
                "{alias} invalid capability"
            );
            assert_eq!(state, before, "{alias} atomic capability rejection");
            assert_eq!(state.get_property_value(alias), "", "{alias} getter");
            assert_eq!(
                state.get_property_value(canonical),
                "auto",
                "{alias} canonical getter"
            );
            assert_eq!(
                state.get_property_priority(alias),
                "important",
                "{alias} priority"
            );
            assert_eq!(state.remove_property(alias), "", "{alias} removal result");
            assert!(state.is_empty(), "{alias} removal");

            state.set_property(alias, "initial", "");
            assert_eq!(
                state.get_property_value(alias),
                "initial",
                "{alias} CSS-wide getter"
            );
            assert_eq!(state.remove_property(alias), "", "{alias} CSS-wide removal");

            state.set_property(alias, "var(--value)", "");
            assert_eq!(
                state.get_property_value(alias),
                "var(--value)",
                "{alias} substitution getter"
            );
            assert_eq!(
                state.get_property_value(canonical),
                "",
                "{alias} canonical substitution getter"
            );
            assert_eq!(
                state.css_text(),
                format!("{canonical}: ;"),
                "{alias} substitution cssText"
            );
            assert_eq!(
                state.remove_property(alias),
                "",
                "{alias} substitution removal"
            );

            state.set_property(canonical, "var(--value)", "");
            assert_eq!(
                state.get_property_value(alias),
                "",
                "{alias} must not expose a canonical substitution write"
            );
            assert_eq!(
                state.get_property_value(canonical),
                "var(--value)",
                "{alias} canonical substitution write"
            );
        }
    }

    #[test]
    fn browser_capability_constraints_cover_shorthand_and_alias_fallbacks() {
        for (name, valid, invalid) in [
            ("transition", "opacity 1s", "none, none"),
            ("-webkit-transition", "opacity 1s", "none, none"),
            ("mask", "content-box", "margin-box"),
            ("-webkit-mask", "content-box", "view-box"),
            ("-webkit-mask-clip", "content-box", "no-clip"),
            ("-webkit-mask-origin", "padding-box", "fill-box"),
            ("-webkit-mask-composite", "initial", "add"),
        ] {
            let mut state = DeclarationState::new();
            assert_eq!(
                state.set_property(name, valid, ""),
                MutationOutcome::Applied,
                "{name}: {valid}"
            );
            let before = state.clone();
            assert_eq!(
                state.set_property(name, invalid, ""),
                MutationOutcome::InvalidValue,
                "{name}: {invalid}"
            );
            assert_eq!(state, before, "{name} atomicity");
        }
    }

    #[test]
    fn page_break_aliases_hide_only_pending_values_from_the_canonical_property() {
        for (alias, canonical) in [
            ("page-break-after", "break-after"),
            ("page-break-before", "break-before"),
            ("page-break-inside", "break-inside"),
        ] {
            let mut state = DeclarationState::new();
            assert_eq!(
                state.set_property(alias, "auto", ""),
                MutationOutcome::Applied
            );
            assert_eq!(state.get_property_value(alias), "auto");
            assert_eq!(state.get_property_value(canonical), "auto");

            state.set_property(alias, "var(--value)", "");
            assert_eq!(state.get_property_value(alias), "var(--value)");
            assert_eq!(state.get_property_value(canonical), "");
            assert_eq!(state.css_text(), format!("{canonical}: ;"));
        }
    }

    #[test]
    fn current_keyword_and_ordered_set_branches_match_chromium_state() {
        for (name, input, canonical_name, expected) in [
            ("all", "revert-rule", "all", "revert-rule"),
            ("word-break", "auto-phrase", "word-break", "auto-phrase"),
            (
                "transform-style",
                "preserve-3d",
                "transform-style",
                "preserve-3d",
            ),
            (
                "-webkit-transform-style",
                "preserve-3d",
                "transform-style",
                "preserve-3d",
            ),
            (
                "image-rendering",
                "pixelated",
                "image-rendering",
                "pixelated",
            ),
            (
                "image-rendering",
                "crisp-edges",
                "image-rendering",
                "crisp-edges",
            ),
            ("display", "math", "display", "math"),
            ("display", "block math", "display", "block math"),
            ("grid-auto-flow", "dense", "grid-auto-flow", "dense"),
            (
                "scroll-marker-group",
                "before links",
                "scroll-marker-group",
                "before links",
            ),
            (
                "scrollbar-gutter",
                "both-edges stable",
                "scrollbar-gutter",
                "stable both-edges",
            ),
        ] {
            let mut state = DeclarationState::new();
            assert_eq!(
                state.set_property(name, input, ""),
                MutationOutcome::Applied,
                "{name}: {input}"
            );
            assert_eq!(state.item(0), canonical_name, "{name}: {input} item");
            assert_eq!(
                state.get_property_value(canonical_name),
                expected,
                "{name}: {input} value"
            );
        }
    }

    #[test]
    fn legacy_break_aliases_translate_values_without_losing_observability() {
        for (alias, canonical, canonical_value, alias_value) in [
            ("page-break-before", "break-before", "page", "always"),
            ("page-break-after", "break-after", "page", "always"),
            ("-webkit-column-break-before", "break-before", "column", ""),
            ("-webkit-column-break-after", "break-after", "column", ""),
        ] {
            let mut state = DeclarationState::new();
            assert_eq!(
                state.set_property(alias, "always", ""),
                MutationOutcome::Applied,
                "{alias}"
            );
            assert_eq!(state.item(0), canonical, "{alias} item");
            assert_eq!(
                state.get_property_value(canonical),
                canonical_value,
                "{alias} canonical"
            );
            assert_eq!(
                state.get_property_value(alias),
                alias_value,
                "{alias} alias"
            );
        }

        let mut canonical = DeclarationState::new();
        canonical.set_property("break-before", "page", "");
        assert_eq!(canonical.get_property_value("page-break-before"), "always");
        canonical.set_property("break-before", "column", "");
        assert_eq!(canonical.get_property_value("page-break-before"), "");
    }

    #[test]
    fn preserves_legacy_webkit_perspective_syntax_without_weakening_lengths() {
        let mut state = DeclarationState::new();
        assert_eq!(
            state.set_property("-webkit-perspective", "1.5", ""),
            MutationOutcome::Applied
        );
        assert_eq!(state.item(0), "perspective");
        assert_eq!(state.get_property_value("-webkit-perspective"), "1.5px");
        assert_eq!(state.css_text(), "perspective: 1.5px;");

        let before = state.clone();
        assert_eq!(
            state.set_property("-webkit-perspective", "-10px", ""),
            MutationOutcome::InvalidValue
        );
        assert_eq!(state, before);
        assert_eq!(
            state.set_property("perspective", "1", ""),
            MutationOutcome::InvalidValue
        );
        assert_eq!(state, before);
    }

    #[test]
    fn enforces_the_expanded_record_limit_atomically() {
        let limits = ResourceLimits {
            max_declarations_per_block: 1,
            ..ResourceLimits::default()
        };
        let mut state =
            DeclarationState::new_with_context_and_limits(DeclarationContext::Style, limits);
        state.set_property_checked("width", "1px", "").unwrap();

        assert!(matches!(
            state.set_property_checked("height", "2px", ""),
            Err(EngineError::DeclarationLimitExceeded {
                actual: 2,
                limit: 1
            })
        ));
        assert_eq!(state.css_text(), "width: 1px;");

        assert!(matches!(
            state.replace_css_text_checked("padding: 1px"),
            Err(EngineError::DeclarationLimitExceeded {
                actual: 4,
                limit: 1
            })
        ));
        assert_eq!(state.css_text(), "width: 1px;");
    }

    #[test]
    fn reserves_container_depth_before_declaration_mutation() {
        let limits = ResourceLimits {
            max_nesting_depth: 2,
            ..ResourceLimits::default()
        };
        let mut state =
            DeclarationState::new_with_context_and_limits(DeclarationContext::Style, limits);
        assert_eq!(
            state
                .set_property_checked_with_reserved_depth("--x", "fn(1)", "", 1)
                .unwrap(),
            MutationOutcome::Applied
        );
        assert!(matches!(
            state.set_property_checked_with_reserved_depth("--x", "fn(fn(1))", "", 1),
            Err(EngineError::NestingLimitExceeded {
                actual: 2,
                limit: 1
            })
        ));
        assert_eq!(state.get_property_value("--x"), "fn(1)");
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
    fn ordinary_and_custom_declarations_retain_semantic_values() {
        let mut state = DeclarationState::new();
        state.set_property("width", "calc(1px + 2px)", "");
        state.set_property("--theme", "var(--fallback, red)", "");

        for record in state.records() {
            assert_eq!(record.value.kind(), DeclarationValueKind::Semantic);
            assert!(record.value.semantic_value().is_some());
        }
        assert_eq!(state.get_property_value("width"), "calc(3px)");
        assert_eq!(
            state.serialize_safe().unwrap(),
            "width: 3px; --theme: var(--fallback, red);"
        );

        state.set_property("padding", "1px 2px", "");
        for record in state
            .records()
            .iter()
            .filter(|record| record.name.starts_with("padding-"))
        {
            assert_eq!(record.value.kind(), DeclarationValueKind::Semantic);
        }
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
    fn pending_shorthands_preserve_provenance_until_a_longhand_changes() {
        let mut state = DeclarationState::new();
        let value = "72px var(--space, var(--space,";
        assert_eq!(
            state.set_property("padding", value, "important"),
            MutationOutcome::Applied
        );
        assert_eq!(state.len(), 4);
        assert_eq!(state.get_property_value("padding"), value);
        assert_eq!(state.get_property_value("padding-top"), "");
        assert_eq!(state.get_property_priority("padding"), "important");
        assert_eq!(state.get_property_priority("padding-top"), "important");

        state.set_property("padding-left", "3px", "");
        assert_eq!(state.get_property_value("padding"), "");
        assert_eq!(state.get_property_value("padding-left"), "3px");

        let mut equal_priority = DeclarationState::new();
        assert_eq!(
            equal_priority.set_property("padding", value, ""),
            MutationOutcome::Applied
        );
        assert_eq!(
            equal_priority.set_property("padding-left", "3px", ""),
            MutationOutcome::Applied
        );
        assert_eq!(equal_priority.get_property_value("padding"), "");
        assert_eq!(
            equal_priority.css_text(),
            "padding-top: ; padding-right: ; padding-bottom: ; padding-left: 3px;"
        );
    }

    #[test]
    fn safe_serialization_preserves_a_pending_shorthand_before_a_longhand_override() {
        let mut state = DeclarationState::new();
        let value = "var(--zfm-font, normal 700 11px/normal 'Inter', sans-serif)";
        assert_eq!(
            state.set_property("font", value, ""),
            MutationOutcome::Applied
        );
        assert_eq!(
            state.set_property("font-size", "10px", ""),
            MutationOutcome::Applied
        );

        assert_eq!(state.get_property_value("font"), "");
        assert_eq!(state.get_property_value("font-size"), "10px");
        assert_eq!(
            state.serialize_safe().unwrap(),
            format!("font: {value}; font-size: 10px;")
        );

        state.set_property("font-style", "italic", "");
        assert_eq!(
            state.serialize_safe().unwrap(),
            format!("font: {value}; font-style: italic; font-size: 10px;")
        );

        state.set_property("font", "var(--replacement-font)", "");
        assert_eq!(
            state.serialize_safe().unwrap(),
            "font: var(--replacement-font);"
        );
    }

    #[test]
    fn every_pending_shorthand_preserves_equal_priority_longhand_overrides() {
        for shorthand in shorthand_names()
            .filter(|name| canonical_style_property_name(name).as_deref() == Some(*name))
        {
            let Some(longhands) = style_shorthand_longhands(shorthand) else {
                continue;
            };
            let Some(overridden) = longhands.first() else {
                continue;
            };
            let mut state = DeclarationState::new();
            assert_eq!(
                state.set_property(shorthand, "var(--sheetom-pending)", ""),
                MutationOutcome::Applied,
                "{shorthand} pending shorthand"
            );
            assert_eq!(
                state.set_property(overridden, "initial", ""),
                MutationOutcome::Applied,
                "{shorthand} override"
            );
            assert_eq!(
                state.serialize_safe().unwrap(),
                format!("{shorthand}: var(--sheetom-pending); {overridden}: initial;"),
                "{shorthand} serialization"
            );
        }
    }

    #[test]
    fn every_pending_shorthand_rejects_mixed_priority_longhand_overrides() {
        for shorthand in shorthand_names()
            .filter(|name| canonical_style_property_name(name).as_deref() == Some(*name))
        {
            let Some(longhands) = style_shorthand_longhands(shorthand) else {
                continue;
            };
            let Some(overridden) = longhands.first() else {
                continue;
            };
            let mut state = DeclarationState::new();
            assert_eq!(
                state.set_property(shorthand, "var(--sheetom-pending)", "important"),
                MutationOutcome::Applied,
                "{shorthand} pending shorthand"
            );
            assert_eq!(
                state.set_property(overridden, "initial", ""),
                MutationOutcome::Applied,
                "{shorthand} override"
            );
            assert_eq!(
                state.serialize_safe(),
                Err(EngineError::UnrepresentablePendingShorthand {
                    shorthand: shorthand.to_owned(),
                    conflicting_longhands: vec![(*overridden).to_owned()],
                }),
                "{shorthand} serialization"
            );
            assert_eq!(
                state.serialize_safe_resilient().unwrap(),
                (
                    format!(
                        "{shorthand}: var(--sheetom-pending) !important; {overridden}: initial !important;"
                    ),
                    vec![SerializationIssue {
                        shorthand: shorthand.to_owned(),
                        conflicting_longhands: vec![(*overridden).to_owned()],
                    }]
                ),
                "{shorthand} resilient serialization"
            );
        }
    }

    #[test]
    fn pending_normal_shorthand_and_important_longhand_are_exactly_serializable() {
        let mut state = DeclarationState::new();
        state.set_property("font", "var(--font)", "");
        state.set_property("font-size", "10px", "important");

        let expected = "font: var(--font); font-size: 10px !important;";
        assert_eq!(state.serialize_safe().unwrap(), expected);
        assert_eq!(
            state.serialize_safe_resilient().unwrap(),
            (expected.to_owned(), Vec::new())
        );
    }

    #[test]
    fn pending_shorthand_provenance_is_shared_and_released_with_its_last_member() {
        let mut state = DeclarationState::new();
        state.set_property("font", "var(--first)", "");
        let first = state.records()[0].pending_group.as_ref().unwrap();
        let second = state.records()[1].pending_group.as_ref().unwrap();
        assert!(Arc::ptr_eq(first, second));
        let previous = Arc::downgrade(first);

        state.set_property("font", "var(--second)", "");

        assert!(previous.upgrade().is_none());
    }

    #[test]
    fn safe_serialization_rejects_a_removed_pending_longhand() {
        let mut state = DeclarationState::new();
        state.set_property("padding", "var(--padding)", "");
        state.remove_property("padding-left");

        assert_eq!(
            state.serialize_safe(),
            Err(EngineError::UnrepresentablePendingShorthand {
                shorthand: "padding".to_owned(),
                conflicting_longhands: vec!["padding-left".to_owned()],
            })
        );
        assert_eq!(
            state.serialize_safe_resilient().unwrap(),
            (
                "padding: var(--padding);".to_owned(),
                vec![SerializationIssue {
                    shorthand: "padding".to_owned(),
                    conflicting_longhands: vec!["padding-left".to_owned()],
                }]
            )
        );
    }

    #[test]
    fn safe_serialization_does_not_couple_independent_substitutions() {
        let mut state = DeclarationState::new();
        assert_eq!(
            state.set_property("align-content", "var(--align, revert-layer)", ""),
            MutationOutcome::Applied
        );
        assert_eq!(
            state.set_property("justify-content", "var(--justify, revert-layer)", ""),
            MutationOutcome::Applied
        );

        assert_eq!(
            state.serialize_safe().unwrap(),
            "align-content: var(--align, revert-layer); justify-content: var(--justify, revert-layer);"
        );

        let mut shorthand = DeclarationState::new();
        assert_eq!(
            shorthand.set_property(
                "place-content",
                "var(--align, revert-layer) var(--justify, revert-layer)",
                ""
            ),
            MutationOutcome::Applied
        );
        assert_eq!(
            shorthand.serialize_safe().unwrap(),
            "place-content: var(--align, revert-layer) var(--justify, revert-layer);"
        );
    }

    #[test]
    fn custom_functions_are_pending_substitutions_with_atomic_argument_validation() {
        let mut state = DeclarationState::new();
        assert_eq!(
            state.set_property("width", "calc(--double(1px) + 1px)", ""),
            MutationOutcome::Applied
        );
        assert_eq!(
            state.get_property_value("width"),
            "calc(--double(1px) + 1px)"
        );
        for invalid in ["--()", "--double(,)", "--double(1px,)", "--double(1px;2px)"] {
            assert_eq!(
                state.set_property("width", invalid, ""),
                MutationOutcome::InvalidValue,
                "{invalid}"
            );
            assert_eq!(
                state.get_property_value("width"),
                "calc(--double(1px) + 1px)"
            );
        }
    }

    #[test]
    fn replacement_uses_chromium_winners_and_priority_partitioning() {
        let mut state = DeclarationState::new();
        state.replace_declarations(&[
            ParsedDeclaration {
                name: "width".into(),
                value: "1px".into(),
                important: true,
            },
            ParsedDeclaration {
                name: "color".into(),
                value: "red".into(),
                important: false,
            },
            ParsedDeclaration {
                name: "width".into(),
                value: "2px".into(),
                important: false,
            },
            ParsedDeclaration {
                name: "height".into(),
                value: "3px".into(),
                important: true,
            },
        ]);

        assert_eq!(state.len(), 3);
        assert_eq!(state.item(0), "color");
        assert_eq!(state.item(1), "width");
        assert_eq!(state.item(2), "height");
        assert_eq!(state.get_property_value("width"), "1px");
        assert_eq!(state.get_property_priority("width"), "important");
    }

    #[test]
    fn replacement_keeps_pending_groups_distinct_across_winners() {
        let mut state = DeclarationState::new();
        state.replace_declarations(&[
            ParsedDeclaration {
                name: "padding".into(),
                value: "var(--p)".into(),
                important: false,
            },
            ParsedDeclaration {
                name: "padding-left".into(),
                value: "var(--p)".into(),
                important: false,
            },
        ]);

        assert_eq!(state.get_property_value("padding"), "");
        assert_eq!(state.get_property_value("padding-left"), "var(--p)");
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
    fn animation_range_synthesizes_parallel_lists_like_chromium() {
        for (input, expected) in [
            ("normal normal, normal", "normal, normal"),
            ("cover calc(1px + 5%) normal", "cover calc(5% + 1px) normal"),
            ("calc(1px + 5%) normal", "calc(5% + 1px)"),
            ("normal cover", "normal cover"),
        ] {
            let mut state = DeclarationState::new();
            assert_eq!(
                state.set_property("animation-range", input, ""),
                MutationOutcome::Applied,
                "{input} should expand"
            );
            assert_eq!(
                state.get_property_value("animation-range"),
                expected,
                "{input}"
            );
            assert_eq!(state.css_text(), format!("animation-range: {expected};"));
        }

        let mut state = DeclarationState::new();
        state.set_property("animation-range", "normal cover", "");
        assert_eq!(state.get_property_value("animation-range-start"), "normal");
        assert_eq!(state.get_property_value("animation-range-end"), "cover");
    }

    #[test]
    fn transition_observable_omits_defaults_without_weakening_safe_output() {
        for (input, observable) in [
            ("normal linear", "linear"),
            ("1s 1s", "1s 1s"),
            ("none 0s linear 1s normal", "none linear 1s"),
            ("allow-discrete", "allow-discrete"),
        ] {
            let mut state = DeclarationState::new();
            assert_eq!(
                state.set_property("transition", input, ""),
                MutationOutcome::Applied,
                "{input} should expand"
            );
            assert_eq!(
                state.get_property_value("transition"),
                observable,
                "{input}"
            );
            assert_eq!(state.css_text(), format!("transition: {observable};"));

            let serialized = state.serialize_safe().unwrap();
            let mut reparsed = DeclarationState::new();
            reparsed.replace_css_text(&serialized);
            assert_eq!(reparsed.serialize_safe().unwrap(), serialized, "{input}");
        }
    }

    #[test]
    fn text_emphasis_retains_authored_defaults_and_omitted_longhands() {
        for (input, shorthand, style, color) in [
            ("dot filled red", "filled dot red", "filled dot", "red"),
            ("red none", "none red", "none", "red"),
            ("filled sesame", "filled sesame", "filled sesame", "initial"),
            ("red", "red", "initial", "red"),
        ] {
            let mut state = DeclarationState::new();
            assert_eq!(
                state.set_property("text-emphasis", input, ""),
                MutationOutcome::Applied,
                "{input} should expand"
            );
            assert_eq!(
                state.get_property_value("text-emphasis"),
                shorthand,
                "{input}"
            );
            assert_eq!(
                state.get_property_value("text-emphasis-style"),
                style,
                "{input}"
            );
            assert_eq!(
                state.get_property_value("text-emphasis-color"),
                color,
                "{input}"
            );
        }
    }

    #[test]
    fn outline_tracks_authored_components_without_semantic_defaults() {
        let mut state = DeclarationState::new();
        assert_eq!(
            state.set_property("outline", "1px auto", ""),
            MutationOutcome::Applied
        );
        assert_eq!(state.get_property_value("outline"), "auto 1px");
        assert_eq!(state.get_property_value("outline-color"), "initial");
        assert_eq!(state.get_property_value("outline-style"), "auto");
        assert_eq!(state.get_property_value("outline-width"), "1px");

        state.set_property("outline", "red", "");
        assert_eq!(state.get_property_value("outline"), "red");
        assert_eq!(state.get_property_value("outline-color"), "red");
        assert_eq!(state.get_property_value("outline-style"), "initial");
        assert_eq!(state.get_property_value("outline-width"), "initial");
    }

    #[test]
    fn list_style_tracks_omissions_and_none_ambiguity() {
        for (input, shorthand, position, image, style_type) in [
            (
                "sheetom-ident url(\"x.png\") inside",
                "inside url(\"x.png\") sheetom-ident",
                "inside",
                "url(\"x.png\")",
                "sheetom-ident",
            ),
            ("none", "none", "initial", "initial", "none"),
            ("none none", "none none", "initial", "none", "none"),
            ("none square", "none square", "initial", "none", "square"),
        ] {
            let mut state = DeclarationState::new();
            assert_eq!(
                state.set_property("list-style", input, ""),
                MutationOutcome::Applied,
                "{input} should expand"
            );
            assert_eq!(state.get_property_value("list-style"), shorthand, "{input}");
            assert_eq!(
                state.get_property_value("list-style-position"),
                position,
                "{input}"
            );
            assert_eq!(
                state.get_property_value("list-style-image"),
                image,
                "{input}"
            );
            assert_eq!(
                state.get_property_value("list-style-type"),
                style_type,
                "{input}"
            );
        }
    }

    #[test]
    fn sheetom_border_grammars_expand_to_semantic_longhands() {
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

        for (source, shorthand, width, color) in [
            ("red", "red", "initial", "red"),
            ("medium", "medium", "medium", "initial"),
            ("red 1px", "1px red", "1px", "red"),
        ] {
            let mut state = DeclarationState::new();
            assert_eq!(
                state.set_property("-webkit-text-stroke", source, ""),
                MutationOutcome::Applied,
                "{source}"
            );
            assert_eq!(
                state.get_property_value("-webkit-text-stroke"),
                shorthand,
                "{source} shorthand"
            );
            assert_eq!(
                state.get_property_value("-webkit-text-stroke-width"),
                width,
                "{source} width"
            );
            assert_eq!(
                state.get_property_value("-webkit-text-stroke-color"),
                color,
                "{source} color"
            );
            let serialized = state.serialize_safe().unwrap();
            let mut reparsed = DeclarationState::new();
            reparsed.replace_css_text(&serialized);
            assert_eq!(
                reparsed.serialize_safe().unwrap(),
                serialized,
                "{source} reparse"
            );
        }

        let mut atomic = DeclarationState::new();
        assert_eq!(
            atomic.set_property("-webkit-text-stroke", "1px red", ""),
            MutationOutcome::Applied
        );
        let before = atomic.css_text();
        assert_eq!(
            atomic.set_property("-webkit-text-stroke", "solid", ""),
            MutationOutcome::InvalidValue
        );
        assert_eq!(atomic.css_text(), before);
    }

    #[test]
    fn gap_rule_lists_expand_atomically_and_remain_mutable() {
        let source = "1px, repeat(auto, red, 2px dotted), repeat(2, color-mix(in srgb, red, blue))";
        let mut state = DeclarationState::new();
        assert_eq!(
            state.set_property("column-rule", source, ""),
            MutationOutcome::Applied
        );
        assert_eq!(state.get_property_value("column-rule"), source);
        assert_eq!(
            state.get_property_value("column-rule-width"),
            "1px, repeat(auto, medium, 2px), repeat(2, medium)"
        );
        assert_eq!(
            state.get_property_value("column-rule-style"),
            "none, repeat(auto, none, dotted), repeat(2, none)"
        );
        assert_eq!(
            state.get_property_value("column-rule-color"),
            "currentcolor, repeat(auto, red, currentcolor), repeat(2, color-mix(in srgb, red, blue))"
        );
        assert_eq!(state.css_text(), format!("column-rule: {source};"));

        assert_eq!(
            state.set_property(
                "column-rule-style",
                "solid, repeat(auto, none, dotted), repeat(2, none)",
                "",
            ),
            MutationOutcome::Applied
        );
        assert_eq!(
            state.get_property_value("column-rule"),
            "1px solid, repeat(auto, red, 2px dotted), repeat(2, color-mix(in srgb, red, blue))"
        );

        let before = state.css_text();
        assert_eq!(
            state.set_property(
                "column-rule-width",
                "1px, repeat(auto, 2px), repeat(auto, 3px)",
                "",
            ),
            MutationOutcome::InvalidValue
        );
        assert_eq!(state.css_text(), before);

        state.remove_property("column-rule-color");
        assert_eq!(state.get_property_value("column-rule"), "");
    }

    #[test]
    fn rule_component_lists_expand_to_both_axes() {
        let mut state = DeclarationState::new();
        let source = "1px, repeat(auto, thick), repeat(2, 0px)";
        assert_eq!(
            state.set_property("rule-width", source, ""),
            MutationOutcome::Applied
        );
        assert_eq!(state.get_property_value("rule-width"), source);
        assert_eq!(state.get_property_value("column-rule-width"), source);
        assert_eq!(state.get_property_value("row-rule-width"), source);

        let color = "red, repeat(auto, oklch(50% .2 120)), red";
        assert_eq!(
            state.set_property("rule-color", color, ""),
            MutationOutcome::Applied
        );
        assert_eq!(
            state.get_property_value("rule-color"),
            "red, repeat(auto, oklch(0.5 0.2 120)), red"
        );
    }

    #[test]
    fn rule_inset_family_expands_canonicalizes_and_mutates_atomically() {
        let mut state = DeclarationState::new();
        let source = "calc(10px + 5%) -2px / overlap-join 4%";
        let canonical = "calc(5% + 10px) -2px / overlap-join 4%";
        assert_eq!(
            state.set_property("rule-inset", source, "important"),
            MutationOutcome::Applied
        );
        assert_eq!(state.get_property_value("rule-inset"), canonical);
        assert_eq!(state.get_property_value("column-rule-inset"), canonical);
        assert_eq!(state.get_property_value("row-rule-inset"), canonical);
        assert_eq!(state.get_property_priority("rule-inset"), "important");
        assert_eq!(state.len(), 8);
        assert_eq!(state.item(0), "column-rule-inset-cap-start");
        assert_eq!(state.item(7), "row-rule-inset-junction-end");
        assert_eq!(
            state.get_property_value("column-rule-inset-cap-start"),
            "calc(5% + 10px)"
        );
        assert_eq!(
            state.get_property_value("row-rule-inset-junction-start"),
            "overlap-join"
        );
        assert_eq!(
            state.css_text(),
            format!("rule-inset: {canonical} !important;")
        );

        let before = state.css_text();
        for invalid in [
            "auto",
            "1px 2px 3px",
            "1px / 2px / 3px",
            "1px, 2px",
            "overlap-join 1px 2px",
        ] {
            assert_eq!(
                state.set_property("rule-inset", invalid, "important"),
                MutationOutcome::InvalidValue,
                "{invalid}"
            );
            assert_eq!(state.css_text(), before, "{invalid}");
        }

        assert_eq!(
            state.set_property("column-rule-inset-cap-start", "3px", "important"),
            MutationOutcome::Applied
        );
        assert_eq!(state.get_property_value("rule-inset"), "");
        assert_eq!(
            state.get_property_value("column-rule-inset"),
            "3px -2px / overlap-join 4%"
        );
        assert_eq!(state.remove_property("column-rule-inset-cap-start"), "3px");
        assert_eq!(state.get_property_value("column-rule-inset"), "");
        assert_eq!(state.len(), 7);
        assert_eq!(
            state.css_text(),
            "column-rule-inset-cap-end: -2px !important; rule-inset-junction: overlap-join 4% !important; row-rule-inset-cap: calc(5% + 10px) -2px !important;"
        );

        let mut components = DeclarationState::new();
        assert_eq!(
            components.set_property("rule-inset-cap", "1px 2px", ""),
            MutationOutcome::Applied
        );
        assert_eq!(components.get_property_value("rule-inset-cap"), "1px 2px");
        assert_eq!(components.len(), 4);
        assert_eq!(
            components.set_property("rule-inset-junction", "min(1px, 2px)", ""),
            MutationOutcome::Applied
        );
        assert_eq!(
            components.get_property_value("rule-inset-junction"),
            "calc(1px)"
        );
        assert_eq!(
            components.set_property("rule-inset-start", "overlap-join", ""),
            MutationOutcome::Applied
        );
        assert_eq!(
            components.get_property_value("rule-inset-start"),
            "overlap-join"
        );

        let mut pending = DeclarationState::new();
        assert_eq!(
            pending.set_property("rule-inset", "var(--inset)", "important"),
            MutationOutcome::Applied
        );
        assert_eq!(pending.get_property_value("rule-inset"), "var(--inset)");
        assert_eq!(pending.get_property_priority("rule-inset"), "important");
        assert_eq!(pending.len(), 8);
        assert_eq!(pending.get_property_value("row-rule-inset-cap-start"), "");
    }

    #[test]
    fn font_variant_routes_each_component_through_its_longhand_grammar() {
        let mut state = DeclarationState::new();
        let source =
            "text sub jis78 lining-nums stylistic(sheetom-ident) small-caps common-ligatures";
        let expected =
            "common-ligatures small-caps stylistic(sheetom-ident) lining-nums jis78 sub text";
        assert_eq!(
            state.set_property("font-variant", source, ""),
            MutationOutcome::Applied
        );
        assert_eq!(state.get_property_value("font-variant"), expected);
        assert_eq!(
            state.get_property_value("font-variant-ligatures"),
            "common-ligatures"
        );
        assert_eq!(state.get_property_value("font-variant-caps"), "small-caps");
        assert_eq!(
            state.get_property_value("font-variant-alternates"),
            "stylistic(sheetom-ident)"
        );
        assert_eq!(
            state.get_property_value("font-variant-numeric"),
            "lining-nums"
        );
        assert_eq!(state.get_property_value("font-variant-east-asian"), "jis78");
        assert_eq!(state.get_property_value("font-variant-position"), "sub");
        assert_eq!(state.get_property_value("font-variant-emoji"), "text");
        assert_eq!(
            (0..state.len())
                .map(|index| state.item(index))
                .collect::<Vec<_>>(),
            vec![
                "font-variant-ligatures",
                "font-variant-numeric",
                "font-variant-east-asian",
                "font-variant-caps",
                "font-variant-alternates",
                "font-variant-position",
                "font-variant-emoji",
            ]
        );
        assert_eq!(state.css_text(), format!("font-variant: {expected};"));

        let mut east_asian = DeclarationState::new();
        assert_eq!(
            east_asian.set_property("font-variant", "full-width jis78 common-ligatures", "",),
            MutationOutcome::Applied
        );
        assert_eq!(
            east_asian.get_property_value("font-variant-east-asian"),
            "jis78 full-width"
        );
        assert_eq!(
            east_asian.get_property_value("font-variant"),
            "common-ligatures jis78 full-width"
        );

        let mut keyword = DeclarationState::new();
        assert_eq!(
            keyword.set_property("font-variant", "normal", ""),
            MutationOutcome::Applied
        );
        assert_eq!(
            (0..keyword.len())
                .map(|index| keyword.item(index))
                .collect::<Vec<_>>(),
            vec![
                "font-variant-ligatures",
                "font-variant-caps",
                "font-variant-numeric",
                "font-variant-east-asian",
                "font-variant-alternates",
                "font-variant-position",
                "font-variant-emoji",
            ]
        );

        let before = state.css_text();
        assert_eq!(
            state.set_property("font-variant", "lining-nums oldstyle-nums", ""),
            MutationOutcome::InvalidValue
        );
        assert_eq!(state.css_text(), before);

        assert_eq!(
            state.set_property("font-variant-numeric", "slashed-zero ordinal", ""),
            MutationOutcome::Applied
        );
        assert_eq!(
            state.get_property_value("font-variant"),
            "common-ligatures small-caps stylistic(sheetom-ident) slashed-zero ordinal jis78 sub text"
        );
        state.remove_property("font-variant-emoji");
        assert_eq!(state.get_property_value("font-variant"), "");
    }

    #[test]
    fn structural_grammars_expand_by_validated_cardinality() {
        let mut state = DeclarationState::new();
        assert_eq!(
            state.set_property("overscroll-behavior", "contain none", ""),
            MutationOutcome::Applied
        );
        assert_eq!(state.get_property_value("overscroll-behavior-x"), "contain");
        assert_eq!(state.get_property_value("overscroll-behavior-y"), "none");

        assert_eq!(
            state.set_property("overscroll-behavior", "chain contain", ""),
            MutationOutcome::Applied
        );
        assert_eq!(state.get_property_value("overscroll-behavior-x"), "chain");
        assert_eq!(state.get_property_value("overscroll-behavior-y"), "contain");

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
            state.set_property("rule-color", "red", ""),
            MutationOutcome::Applied
        );
        assert_eq!(state.get_property_value("column-rule-color"), "red");
        assert_eq!(state.get_property_value("row-rule-color"), "red");

        assert_eq!(
            state.set_property("contain-intrinsic-size", "none", ""),
            MutationOutcome::Applied
        );
        assert_eq!(state.get_property_value("contain-intrinsic-width"), "none");
        assert_eq!(state.get_property_value("contain-intrinsic-height"), "none");
    }

    #[test]
    fn rule_partition_shorthands_repeat_one_keyword_and_reject_pairs() {
        for (shorthand, longhands, value, invalid, obsolete) in [
            (
                "rule-break",
                ["column-rule-break", "row-rule-break"],
                "intersection",
                "none intersection",
                "spanning-item",
            ),
            (
                "rule-visibility-items",
                ["column-rule-visibility-items", "row-rule-visibility-items"],
                "around",
                "between around",
                "none",
            ),
        ] {
            let mut state = DeclarationState::new();
            assert_eq!(
                state.set_property(shorthand, value, "important"),
                MutationOutcome::Applied,
                "{shorthand}"
            );
            assert_eq!(state.get_property_value(shorthand), value, "{shorthand}");
            for longhand in longhands {
                assert_eq!(state.get_property_value(longhand), value, "{longhand}");
                assert_eq!(
                    state.get_property_priority(longhand),
                    "important",
                    "{longhand}"
                );
            }

            let before = state.css_text();
            for rejected in [invalid, obsolete] {
                assert_eq!(
                    state.set_property(shorthand, rejected, ""),
                    MutationOutcome::InvalidValue,
                    "{shorthand}: {rejected}"
                );
                assert_eq!(state.css_text(), before, "{shorthand}: {rejected}");
            }
        }
    }

    #[test]
    fn contain_intrinsic_size_synthesizes_equal_compound_axes() {
        let mut state = DeclarationState::new();
        assert_eq!(
            state.set_property("contain-intrinsic-size", "auto none auto none", ""),
            MutationOutcome::Applied
        );
        assert_eq!(
            state.get_property_value("contain-intrinsic-width"),
            "auto none"
        );
        assert_eq!(
            state.get_property_value("contain-intrinsic-height"),
            "auto none"
        );
        assert_eq!(
            state.get_property_value("contain-intrinsic-size"),
            "auto none",
            "records: {:?}",
            state.records
        );
    }

    #[test]
    fn observable_default_shorthands_match_chromium() {
        for (name, input, expected) in [
            ("border", "none", ""),
            ("border", "currentcolor", ""),
            ("border-block-end", "currentcolor", "currentcolor"),
            ("text-decoration", "currentcolor", "none"),
            ("text-decoration", "auto", "none"),
            ("text-decoration", "solid", "none"),
        ] {
            let mut state = DeclarationState::new();
            assert_eq!(
                state.set_property(name, input, ""),
                MutationOutcome::Applied,
                "{name}: {input}"
            );
            assert_eq!(
                state.get_property_value(name),
                expected,
                "{name}: {input}; records: {:?}",
                state.records
            );
        }
    }

    #[test]
    fn observable_composite_branches_match_chromium() {
        for (name, input, expected) in [
            ("-webkit-border-radius", "1px 2px", "1px / 2px"),
            ("-webkit-border-radius", "0px", "0px"),
            ("border-radius", "0px", "0px"),
            ("border-radius", "1px / 1px 1px", "1px"),
            (
                "border-radius",
                "1px / calc(1px + 5%)",
                "1px / calc(5% + 1px)",
            ),
            ("column-rule-inset", "1px 2px", "1px 2px / 1px 2px"),
            ("font", "italic 16px/1.5 serif", "italic 16px / 1.5 serif"),
            ("font", "2px dashed red", "2px \"dashed red\""),
            ("grid-template", "\"text\"", "\"text\""),
            ("offset", "1px 2px", "1px 2px"),
            ("offset", "left 10px top 20px", "left 10px top 20px"),
            ("offset-anchor", "min(1px, 2px)", "calc(1px) center"),
            ("outline", "2px dashed red", "red dashed 2px"),
            ("scrollbar-color", "red blue", "red blue"),
        ] {
            let mut state = DeclarationState::new();
            assert_eq!(
                state.set_property(name, input, ""),
                MutationOutcome::Applied,
                "{name}: {input}"
            );
            assert_eq!(
                state.get_property_value(name),
                expected,
                "{name}: {input}; records: {:?}",
                state.records
            );
        }

        for (value, start, end, observable) in [
            ("auto", "normal", "normal", "none"),
            ("normal", "normal", "normal", "none"),
            ("10px", "10px", "normal", "10px"),
            ("scroll", "scroll", "scroll", "scroll"),
            ("1px 2px", "1px", "2px", "1px 2px"),
        ] {
            let mut state = DeclarationState::new();
            assert_eq!(
                state.set_property("timeline-trigger", value, ""),
                MutationOutcome::Applied,
                "{value}"
            );
            assert_eq!(
                state.get_property_value("timeline-trigger-activation-range-start"),
                start,
                "{value}"
            );
            assert_eq!(
                state.get_property_value("timeline-trigger-activation-range-end"),
                end,
                "{value}"
            );
            assert_eq!(
                state.get_property_value("timeline-trigger"),
                observable,
                "{value}"
            );
        }

        for (value, expected_fallbacks, expected_shorthand) in [
            ("center", "center", "center"),
            ("left top", "left top", "left top"),
            ("normal top left", "left top", "left top"),
            (
                "normal --fallback flip-y flip-x flip-start flip-inline flip-block",
                "--fallback flip-y flip-x flip-start flip-inline flip-block",
                "--fallback flip-y flip-x flip-start flip-inline flip-block",
            ),
            (
                "normal flip-block --fallback",
                "--fallback flip-block",
                "--fallback flip-block",
            ),
            ("most-width none", "none", "most-width none"),
        ] {
            let mut state = DeclarationState::new();
            assert_eq!(
                state.set_property("position-try", value, ""),
                MutationOutcome::Applied,
                "{value}"
            );
            assert_eq!(
                state.get_property_value("position-try-fallbacks"),
                expected_fallbacks,
                "{value}"
            );
            assert_eq!(
                state.get_property_value("position-try"),
                expected_shorthand,
                "{value}"
            );
        }

        for (name, input, expected) in [
            ("columns", "calc(1 + 1)", "calc(2)"),
            ("grid-area", "calc(1 + 1)", "calc(2)"),
            ("grid-column", "calc(1 + 1)", "calc(2)"),
            ("grid-row", "calc(1 + 1)", "calc(2)"),
            ("flex-flow", "wrap balance", "balance"),
            ("font-variant", "none", "none"),
            ("text-box", "none", "normal"),
        ] {
            let mut state = DeclarationState::new();
            assert_eq!(
                state.set_property(name, input, ""),
                MutationOutcome::Applied,
                "{name}: {input}"
            );
            assert_eq!(state.get_property_value(name), expected, "{name}: {input}");
        }

        let mut mask = DeclarationState::new();
        assert_eq!(
            mask.set_property("-webkit-mask-box-image", "10%", ""),
            MutationOutcome::Applied
        );
        assert_eq!(mask.get_property_value("-webkit-mask-box-image"), "");
        assert_eq!(
            mask.get_property_value("-webkit-mask-box-image-slice"),
            "10% fill"
        );

        for (input, expected) in [
            (
                "1 fill",
                ["initial", "1 fill", "initial", "initial", "initial"],
            ),
            (
                "1 fill /  / 1px",
                ["initial", "1 fill", "initial", "1px", "initial"],
            ),
            (
                "url(a.png) repeat 1 fill / auto / 2px",
                ["url(\"a.png\")", "1 fill", "auto", "2px", "repeat"],
            ),
            (
                "1 2 3 4 fill / 3px 4px 5px 6px / 5px 6px 7px 8px",
                [
                    "initial",
                    "1 2 3 4 fill",
                    "3px 4px 5px 6px",
                    "5px 6px 7px 8px",
                    "initial",
                ],
            ),
            (
                "1 fill / calc(1px + 5%) / 1px none repeat round",
                ["none", "1 fill", "calc(5% + 1px)", "1px", "repeat round"],
            ),
            (
                "none r\\65 peat 1 f\\69 ll / 1PX / 2PX",
                ["none", "1 fill", "1px", "2px", "repeat"],
            ),
            (
                "repeat repeat",
                ["initial", "initial", "initial", "initial", "repeat"],
            ),
        ] {
            let mut state = DeclarationState::new();
            assert_eq!(
                state.set_property("-webkit-mask-box-image", input, ""),
                MutationOutcome::Applied,
                "{input}"
            );
            assert_eq!(state.get_property_value("-webkit-mask-box-image"), "");
            for (index, longhand) in [
                "-webkit-mask-box-image-source",
                "-webkit-mask-box-image-slice",
                "-webkit-mask-box-image-width",
                "-webkit-mask-box-image-outset",
                "-webkit-mask-box-image-repeat",
            ]
            .iter()
            .enumerate()
            {
                assert_eq!(state.item(index), *longhand, "{input}");
                assert_eq!(
                    state.get_property_value(longhand),
                    expected[index],
                    "{input}"
                );
            }
        }

        for invalid in [
            "fill",
            "1 fill fill",
            "repeat none round",
            "1 fill none / 1px",
            "1 fill / none / 1px",
            "1 fill / 1px / repeat 1px",
            "alpha",
            "luminance",
            "none none",
            "1 2 3 4 5 fill",
            "-1 fill",
            "1 fill / -1px",
            "1 fill / 1px / 10%",
        ] {
            let mut state = DeclarationState::new();
            state.set_property("-webkit-mask-box-image", "none", "");
            let before = state.css_text();
            assert_eq!(
                state.set_property("-webkit-mask-box-image", invalid, ""),
                MutationOutcome::InvalidValue,
                "{invalid}"
            );
            assert_eq!(state.css_text(), before, "{invalid}");
        }

        for (name, value) in [
            ("animation", "100px 2"),
            ("timeline-trigger", "red"),
            ("position-try", "normal"),
        ] {
            let mut state = DeclarationState::new();
            state.set_property("color", "red", "");
            let before = state.css_text();
            assert_eq!(
                state.set_property(name, value, ""),
                MutationOutcome::InvalidValue,
                "{name}: {value}"
            );
            assert_eq!(state.css_text(), before, "{name}: {value}");
        }
    }

    #[test]
    fn timeline_trigger_owns_lists_sources_ranges_and_abbreviated_serialization() {
        for (input, observable, expected) in [
            (
                "none scroll(block root) normal normal / auto auto",
                "scroll(root)",
                ["none", "scroll(root)", "normal", "normal", "auto", "auto"],
            ),
            (
                "none auto entry-crossing 1px normal / auto auto",
                "entry-crossing 1px",
                [
                    "none",
                    "auto",
                    "entry-crossing 1px",
                    "normal",
                    "auto",
                    "auto",
                ],
            ),
            (
                "none auto normal normal / scroll 1px auto",
                " / scroll 1px",
                ["none", "auto", "normal", "normal", "scroll 1px", "auto"],
            ),
            (
                "none auto normal normal / auto auto, --x view(block 1px) normal normal / auto auto",
                "none, --x view(1px)",
                [
                    "none, --x",
                    "auto, view(1px)",
                    "normal, normal",
                    "normal, normal",
                    "auto, auto",
                    "auto, auto",
                ],
            ),
        ] {
            let mut state = DeclarationState::new();
            assert_eq!(
                state.set_property("timeline-trigger", input, ""),
                MutationOutcome::Applied,
                "{input}"
            );
            assert_eq!(state.get_property_value("timeline-trigger"), observable);
            for (index, longhand) in [
                "timeline-trigger-name",
                "timeline-trigger-source",
                "timeline-trigger-activation-range-start",
                "timeline-trigger-activation-range-end",
                "timeline-trigger-active-range-start",
                "timeline-trigger-active-range-end",
            ]
            .iter()
            .enumerate()
            {
                assert_eq!(state.item(index), *longhand, "{input}");
                assert_eq!(state.get_property_value(longhand), expected[index], "{input}");
            }
        }

        let mut ranges = DeclarationState::new();
        assert_eq!(
            ranges.set_property("timeline-trigger-active-range", "auto, cover 1px auto", ""),
            MutationOutcome::Applied
        );
        assert_eq!(
            ranges.get_property_value("timeline-trigger-active-range"),
            "auto, cover 1px"
        );
        assert_eq!(
            ranges.get_property_value("timeline-trigger-active-range-start"),
            "auto, cover 1px"
        );
        assert_eq!(
            ranges.get_property_value("timeline-trigger-active-range-end"),
            "auto, auto"
        );

        let before = ranges.css_text();
        assert_eq!(
            ranges.set_property("timeline-trigger-active-range", "auto,", ""),
            MutationOutcome::InvalidValue
        );
        assert_eq!(ranges.css_text(), before);
    }

    #[test]
    fn offset_owns_ray_coordinate_boxes_and_unordered_motion_components() {
        for (input, observable, expected_path, expected_distance, expected_rotate) in [
            (
                "normal ray(at center contain closest-corner 45deg) content-box 1px / auto",
                "ray(45deg closest-corner contain at center center) content-box 1px",
                "ray(45deg closest-corner contain at center center) content-box",
                "1px",
                "auto",
            ),
            (
                "normal none auto 45deg / auto",
                "none auto 45deg",
                "none",
                "0px",
                "auto 45deg",
            ),
            (
                "normal none auto 1px / auto",
                "none 1px",
                "none",
                "1px",
                "auto",
            ),
            (
                "normal none 1px auto 0deg / auto",
                "none 1px",
                "none",
                "1px",
                "auto 0deg",
            ),
            (
                "normal none auto 0deg / auto",
                "normal",
                "none",
                "0px",
                "auto 0deg",
            ),
        ] {
            let mut state = DeclarationState::new();
            assert_eq!(
                state.set_property("offset", input, ""),
                MutationOutcome::Applied,
                "{input}"
            );
            assert_eq!(state.get_property_value("offset"), observable, "{input}");
            assert_eq!(
                state.get_property_value("offset-path"),
                expected_path,
                "{input}"
            );
            assert_eq!(
                state.get_property_value("offset-distance"),
                expected_distance,
                "{input}"
            );
            assert_eq!(
                state.get_property_value("offset-rotate"),
                expected_rotate,
                "{input}"
            );
        }

        let mut state = DeclarationState::new();
        assert_eq!(
            state.set_property("offset", "normal none 1px / auto", ""),
            MutationOutcome::Applied
        );
        let before = state.css_text();
        assert_eq!(
            state.set_property("offset", "normal none auto auto / auto", ""),
            MutationOutcome::InvalidValue
        );
        assert_eq!(state.css_text(), before);
    }

    #[test]
    fn anchor_functions_are_typed_only_in_inset_grammars() {
        for name in [
            "top",
            "right",
            "bottom",
            "left",
            "inset-block-start",
            "inset-block-end",
            "inset-inline-start",
            "inset-inline-end",
        ] {
            let mut state = DeclarationState::new();
            assert_eq!(
                state.set_property(
                    name,
                    "anchor(inside --sheetom, calc(anchor-size(width) + 1px))",
                    "",
                ),
                MutationOutcome::Applied,
                "{name}"
            );
            assert_eq!(
                state.get_property_value(name),
                "anchor(--sheetom inside, calc(1px + anchor-size(width)))",
                "{name}"
            );
        }

        let mut recursive = DeclarationState::new();
        assert_eq!(
            recursive.set_property(
                "top",
                "anchor(inside, anchor(outside, calc(anchor-size(width) + 1px)))",
                "",
            ),
            MutationOutcome::Applied
        );
        assert_eq!(
            recursive.get_property_value("top"),
            "anchor(inside, anchor(outside, calc(1px + anchor-size(width))))"
        );
        assert_eq!(
            recursive.set_property("margin-top", "anchor(inside)", ""),
            MutationOutcome::InvalidValue
        );
        assert_eq!(
            recursive.set_property("width", "anchor(inside)", ""),
            MutationOutcome::InvalidValue
        );
        assert_eq!(
            recursive.set_property("padding-top", "anchor-size(width)", ""),
            MutationOutcome::InvalidValue
        );
    }

    #[test]
    fn anchor_inset_shorthands_expand_mutate_and_recover_atomically() {
        for (name, value) in [
            ("inset", "auto auto auto auto"),
            ("inset-block", "auto auto"),
            ("inset-inline", "auto auto"),
        ] {
            let mut defaults = DeclarationState::new();
            assert_eq!(
                defaults.set_property(name, value, ""),
                MutationOutcome::Applied
            );
            assert_eq!(defaults.get_property_value(name), "auto", "{name}");
        }

        let mut state = DeclarationState::new();
        assert_eq!(
            state.set_property(
                "inset",
                "anchor(inside) anchor(--sheetom outside, 1px) calc(anchor(start) + 1px) anchor(20%)",
                "important",
            ),
            MutationOutcome::Applied
        );
        assert_eq!(state.len(), 4);
        assert_eq!(state.item(0), "top");
        assert_eq!(state.item(1), "right");
        assert_eq!(state.item(2), "bottom");
        assert_eq!(state.item(3), "left");
        assert_eq!(state.get_property_value("top"), "anchor(inside)");
        assert_eq!(
            state.get_property_value("right"),
            "anchor(--sheetom outside, 1px)"
        );
        assert_eq!(
            state.get_property_value("bottom"),
            "calc(1px + anchor(start))"
        );
        assert_eq!(state.get_property_value("left"), "anchor(20%)");
        assert_eq!(
            state.get_property_value("inset"),
            "anchor(inside) anchor(--sheetom outside, 1px) calc(1px + anchor(start)) anchor(20%)"
        );
        assert_eq!(state.get_property_priority("inset"), "important");

        assert_eq!(
            state.set_property("top", "2px", ""),
            MutationOutcome::Applied
        );
        assert_eq!(state.get_property_value("inset"), "");
        assert_eq!(state.remove_property("top"), "2px");
        assert_eq!(state.get_property_value("top"), "");
        assert_eq!(state.len(), 3);

        let before = state.css_text();
        for invalid in [
            "anchor()",
            "anchor(--sheetom)",
            "anchor(inside,)",
            "anchor(inside outside)",
            "anchor(10px)",
            "anchor(inside, calc(anchor-size(width) + 1s))",
        ] {
            assert_eq!(
                state.set_property("right", invalid, ""),
                MutationOutcome::InvalidValue,
                "{invalid}"
            );
            assert_eq!(state.css_text(), before, "{invalid}");
        }

        let mut pending = DeclarationState::new();
        assert_eq!(
            pending.set_property("inset", "anchor(inside, var(--fallback))", "important",),
            MutationOutcome::Applied
        );
        assert_eq!(pending.len(), 4);
        assert_eq!(
            pending.get_property_value("inset"),
            "anchor(inside, var(--fallback))"
        );
        assert_eq!(pending.get_property_value("top"), "");
        assert_eq!(pending.get_property_priority("top"), "important");
    }

    #[test]
    fn structural_grammars_reject_invalid_neighbors_atomically() {
        let mut state = DeclarationState::new();
        for name in ["border-image", "-webkit-border-image"] {
            assert_eq!(
                state.set_property(name, "url(a.png) 30", ""),
                MutationOutcome::Applied,
                "{name}"
            );
            let before = state.css_text();
            assert_eq!(
                state.set_property(name, "-1", ""),
                MutationOutcome::InvalidValue,
                "{name}"
            );
            assert_eq!(state.css_text(), before, "{name}");
        }
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
    fn function_descriptors_keep_locals_and_result_only() {
        let mut state = DeclarationState::new_with_context(DeclarationContext::Function);
        state.replace_css_text(
            "--x: 1px; color: red; result: 2px; --x: 3px; unknown: value; result: ;",
        );
        assert_eq!(state.len(), 2);
        assert_eq!(state.item(0), "--x");
        assert_eq!(state.item(1), "result");
        assert_eq!(state.get_property_value("--x"), "3px");
        assert_eq!(state.get_property_value("result"), "");
        assert_eq!(state.css_text(), "--x: 3px; result: ;");

        assert_eq!(
            state.set_property("result", "red", ""),
            MutationOutcome::Applied
        );
        assert_eq!(state.get_property_value("result"), "");
        assert_eq!(state.remove_property("result"), "");
        assert_eq!(state.css_text(), "--x: 3px;");

        state.replace_css_text(
            "--x: 1px !important; --x: 2px; result: 3px !important; result: 4px;",
        );
        assert_eq!(state.css_text(), "--x: 2px; result: 4px;");
        assert_eq!(
            state.set_property("--x", "5px", "important"),
            MutationOutcome::Applied
        );
        assert_eq!(state.css_text(), "--x: 5px !important; result: 4px;");
        assert_eq!(
            state.set_property("result", "", ""),
            MutationOutcome::Applied
        );
        assert_eq!(state.css_text(), "--x: 5px !important;");
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
            for record in state.records() {
                if !matches!(
                    record.value.kind(),
                    DeclarationValueKind::Semantic | DeclarationValueKind::CssWideKeyword
                ) || record.pending_group.as_ref().is_some_and(|group| {
                    !matches!(
                        group.value.kind(),
                        DeclarationValueKind::Semantic | DeclarationValueKind::CssWideKeyword
                    )
                }) {
                    failures.push(format!(
                        "{property}: {} retained non-semantic authority",
                        record.name
                    ));
                }
            }
            let expected = case["chromium"]["longhands"]
                .as_array()
                .expect("every capability should contain Chromium longhands");
            for (actual, expected) in state.records().iter().zip(expected) {
                let expected_name = expected["name"].as_str().unwrap_or_default();
                let expected_value = expected["value"].as_str().unwrap_or_default();
                if actual.name != expected_name || actual.observable_value() != expected_value {
                    failures.push(format!(
                        "{property}: expected {expected_name}: {expected_value}, got {}: {}",
                        actual.name,
                        actual.observable_value()
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
    fn every_composite_number_result_matches_chromium_state() {
        let fixture: Value = serde_json::from_str(include_str!(
            "../../../compatibility/number-result-math-capabilities.json"
        ))
        .expect("the checked-in Chromium number-result corpus should be valid JSON");
        let cases = fixture["cases"]
            .as_array()
            .expect("the number-result corpus should contain cases");

        for candidate in cases {
            if candidate["integration"].as_str() != Some("composite-property") {
                continue;
            }
            let id = candidate["id"].as_str().unwrap_or_default();
            let property = candidate["property"].as_str().unwrap_or_default();
            let input = candidate["input"].as_str().unwrap_or_default();
            let accepted = candidate["accepted"].as_bool().unwrap_or_default();
            let mut state = DeclarationState::new();

            if !accepted {
                assert_eq!(
                    state.set_property("color", "red", ""),
                    MutationOutcome::Applied,
                    "{id} seed"
                );
                let before = state.clone();
                assert_eq!(
                    state.set_property(property, input, ""),
                    MutationOutcome::InvalidValue,
                    "{id} outcome"
                );
                assert_eq!(state, before, "{id} atomicity");
                continue;
            }

            assert_eq!(
                state.set_property(property, input, ""),
                MutationOutcome::Applied,
                "{id} outcome"
            );
            let expected_items = candidate["items"]
                .as_array()
                .expect("accepted number-result cases should contain items");
            assert_eq!(state.len(), expected_items.len(), "{id} length");
            for (index, expected) in expected_items.iter().enumerate() {
                assert_eq!(
                    state.item(index),
                    expected.as_str().unwrap_or_default(),
                    "{id}"
                );
            }
            assert_eq!(
                state.get_property_value(property),
                candidate["observable"].as_str().unwrap_or_default(),
                "{id} getter"
            );
            assert_eq!(
                state.css_text(),
                candidate["cssText"].as_str().unwrap_or_default(),
                "{id} cssText"
            );

            let mut reparsed = DeclarationState::new();
            reparsed.replace_css_text(&state.serialize_safe().unwrap());
            assert_eq!(
                reparsed.serialize_safe().unwrap(),
                state.serialize_safe().unwrap(),
                "{id} round-trip"
            );
        }
    }

    #[test]
    fn grid_shorthands_own_explicit_subgrid_area_and_auto_flow_branches() {
        let mut template = DeclarationState::new();
        assert_eq!(
            template.set_property("grid-template", "subgrid [a] / none", ""),
            MutationOutcome::Applied
        );
        assert_eq!(
            template.get_property_value("grid-template"),
            "subgrid [a] / none"
        );

        assert_eq!(
            template.set_property(
                "grid-template",
                "[top] \"main\" 1px [bottom] / [left] 1fr [right]",
                "",
            ),
            MutationOutcome::Applied
        );
        assert_eq!(
            template.get_property_value("grid-template"),
            "[top] \"main\" 1px [bottom] / [left] 1fr [right]"
        );
        assert_eq!(
            template.get_property_value("grid-template-rows"),
            "[top] 1px [bottom]"
        );
        assert_eq!(
            template.get_property_value("grid-template-areas"),
            "\"main\""
        );

        let mut auto_flow = DeclarationState::new();
        let source = "auto-flow dense 1px / [left] 1fr [right]";
        assert_eq!(
            auto_flow.set_property("grid", source, ""),
            MutationOutcome::Applied
        );
        assert_eq!(auto_flow.get_property_value("grid"), source);
        assert_eq!(auto_flow.item(0), "grid-template-columns");
        assert_eq!(auto_flow.get_property_value("grid-auto-flow"), "dense");

        assert_eq!(
            auto_flow.set_property("grid-auto-rows", "2px", ""),
            MutationOutcome::Applied
        );
        assert_eq!(
            auto_flow.get_property_value("grid"),
            "auto-flow dense 2px / [left] 1fr [right]"
        );
        let before = auto_flow.css_text();
        assert_eq!(
            auto_flow.set_property("grid", "none /", ""),
            MutationOutcome::InvalidValue
        );
        assert_eq!(auto_flow.css_text(), before);
    }

    #[test]
    fn grid_placement_shorthands_use_typed_cssom_ordering() {
        for (property, source, expected, longhand) in [
            ("grid-row", "1 span / auto", "span 1", "grid-row-start"),
            (
                "grid-column",
                "auto / 1 span",
                "auto / span 1",
                "grid-column-end",
            ),
            (
                "grid-area",
                "span sheetom-ident 1 / auto",
                "span sheetom-ident",
                "grid-row-start",
            ),
        ] {
            let mut state = DeclarationState::new();
            assert_eq!(
                state.set_property(property, source, ""),
                MutationOutcome::Applied,
                "{property}: {source}"
            );
            assert_eq!(
                state.get_property_value(property),
                expected,
                "{property}: {source} shorthand"
            );
            assert!(
                state.get_property_value(longhand).starts_with("span"),
                "{property}: {source} longhand"
            );
            let serialized = state.serialize_safe().unwrap();
            let mut reparsed = DeclarationState::new();
            reparsed.replace_css_text(&serialized);
            assert_eq!(
                reparsed.serialize_safe().unwrap(),
                serialized,
                "{property}: {source}"
            );
        }

        let mut atomic = DeclarationState::new();
        assert_eq!(
            atomic.set_property("grid-row", "span 1", ""),
            MutationOutcome::Applied
        );
        let before = atomic.css_text();
        assert_eq!(
            atomic.set_property("grid-row", "1 span sheetom-ident", ""),
            MutationOutcome::InvalidValue
        );
        assert_eq!(atomic.css_text(), before);
    }

    #[test]
    fn simplified_numeric_calculations_remain_reparsably_idempotent() {
        for source in ["calc(1 / 2)", "calc(50%)"] {
            let mut state = DeclarationState::new();
            assert_eq!(
                state.set_property("opacity", source, ""),
                MutationOutcome::Applied,
                "{source}"
            );

            let serialized = state.serialize_safe().unwrap();
            let mut reparsed = DeclarationState::new();
            reparsed.replace_css_text(&serialized);
            assert_eq!(reparsed.serialize_safe().unwrap(), serialized, "{source}");
        }
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
            for record in state.records() {
                if !matches!(
                    record.value.kind(),
                    DeclarationValueKind::Semantic | DeclarationValueKind::CssWideKeyword
                ) || record.pending_group.as_ref().is_some_and(|group| {
                    !matches!(
                        group.value.kind(),
                        DeclarationValueKind::Semantic | DeclarationValueKind::CssWideKeyword
                    )
                }) {
                    failures.push(format!(
                        "{id}: {} retained non-semantic authority",
                        record.name
                    ));
                }
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
                if actual.name != expected_name || actual.observable_value() != expected_value {
                    failures.push(format!(
                        "{id}: expected {expected_name}: {expected_value}, got {}: {}",
                        actual.name,
                        actual.observable_value()
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

    #[test]
    fn mask_shorthand_exposes_every_typed_component() {
        for shorthand in ["mask", "-webkit-mask"] {
            let mut state = DeclarationState::new();
            assert_eq!(
                state.set_property(
                    shorthand,
                    "url(\"x\") center / 1px repeat-x content-box content-box add alpha",
                    "",
                ),
                MutationOutcome::Applied,
            );
            assert_eq!(state.get_property_value("mask-mode"), "alpha");
            assert_eq!(state.get_property_value("mask-composite"), "add");
            assert_eq!(
                state.get_property_value(shorthand),
                "url(\"x\") center center / 1px repeat-x content-box alpha"
            );
        }

        let mut state = DeclarationState::new();
        state.set_property("mask", "intersect", "");
        assert_eq!(state.get_property_value("mask-composite"), "intersect");

        state.set_property("mask", "alpha", "");
        assert_eq!(state.get_property_value("mask-mode"), "alpha");
        assert_eq!(state.get_property_value("mask-image"), "initial");
        assert_eq!(state.get_property_value("mask"), "alpha");

        state.set_property("mask", "none, none", "");
        assert_eq!(
            state.get_property_value("-webkit-mask-position-x"),
            "0%, 0%"
        );
        assert_eq!(
            state.get_property_value("-webkit-mask-position-y"),
            "0%, 0%"
        );
        assert_eq!(state.get_property_value("mask"), "none, none");

        state.set_property("mask", "no-clip", "");
        assert_eq!(state.get_property_value("mask"), "no-clip");
    }

    #[test]
    fn final_indexed_shorthands_match_chromium_longhand_state() {
        let cases = [
            (
                "marker",
                "url(\"x\")",
                vec![
                    ("marker-start", "url(\"x\")"),
                    ("marker-mid", "url(\"x\")"),
                    ("marker-end", "url(\"x\")"),
                ],
            ),
            (
                "mask-position",
                "left 10px top 20px, center",
                vec![
                    ("-webkit-mask-position-x", "left 10px, center"),
                    ("-webkit-mask-position-y", "top 20px, center"),
                ],
            ),
            (
                "border-spacing",
                "1px 2px",
                vec![
                    ("-webkit-border-horizontal-spacing", "1px"),
                    ("-webkit-border-vertical-spacing", "2px"),
                ],
            ),
            (
                "animation",
                "-1s",
                vec![("animation-duration", "auto"), ("animation-delay", "-1s")],
            ),
        ];
        for (shorthand, input, expected) in cases {
            let mut state = DeclarationState::new();
            assert_eq!(
                state.set_property(shorthand, input, ""),
                MutationOutcome::Applied,
                "{shorthand}: {input}",
            );
            for (longhand, value) in expected {
                assert_eq!(
                    state.get_property_value(longhand),
                    value,
                    "{shorthand}: {input} -> {longhand}",
                );
            }
        }
    }

    #[test]
    fn legacy_background_size_duplicates_each_list_member() {
        let mut state = DeclarationState::new();
        assert_eq!(
            state.set_property("-webkit-background-size", "1px, 1px", ""),
            MutationOutcome::Applied,
        );
        assert_eq!(state.get_property_value("background-size"), "1px, 1px 1px");

        state.set_property("-webkit-background-size", "1px, 2px, 3px", "");
        assert_eq!(
            state.get_property_value("background-size"),
            "1px, 2px, 3px 3px"
        );
    }

    #[test]
    fn cursor_hotspots_truncate_toward_zero_for_cssom_observability() {
        for (input, expected) in [
            ("url(\"x\") 1.5 1.5, auto", "url(\"x\") 1 1, auto"),
            ("url(\"x\") -1.5 -2.9, auto", "url(\"x\") -1 -2, auto"),
            ("url(\"x\") .9 2.1, auto", "url(\"x\") 0 2, auto"),
        ] {
            let mut state = DeclarationState::new();
            assert_eq!(
                state.set_property("cursor", input, ""),
                MutationOutcome::Applied,
            );
            assert_eq!(state.get_property_value("cursor"), expected, "{input}");
        }
    }

    #[test]
    fn canonical_shorthand_provenance_matches_chromium_families() {
        for (property, input, expected) in [
            ("overflow", "visible visible", "visible"),
            (
                "background-position",
                "center, center",
                "center center, center center",
            ),
            ("padding", "1px 1px 1px 1px", "1px"),
            ("padding-block", "1px 1px", "1px"),
            ("border-color", "red red red red", "red"),
            ("scroll-padding", "auto auto auto auto", "auto"),
            (
                "mask-position",
                "center, center",
                "center center, center center",
            ),
            ("font-synthesis", "small-caps weight", "weight small-caps"),
            ("place-items", "normal first baseline", "normal baseline"),
            ("gap", "normal calc(1px + 5%)", "normal calc(5% + 1px)"),
        ] {
            let mut state = DeclarationState::new();
            assert_eq!(
                state.set_property(property, input, ""),
                MutationOutcome::Applied,
                "{property}: {input}",
            );
            assert_eq!(
                state.get_property_value(property),
                expected,
                "{property}: {input}",
            );
        }
    }

    #[test]
    fn border_shorthands_separate_omitted_longhands_from_semantic_initials() {
        let mut state = DeclarationState::new();
        state.set_property("border-block-start", "red none 1px", "");
        assert_eq!(
            state.get_property_value("border-block-start"),
            "1px none red"
        );
        assert_eq!(state.get_property_value("border-block-start-width"), "1px");
        assert_eq!(state.get_property_value("border-block-start-style"), "none");
        assert_eq!(state.get_property_value("border-block-start-color"), "red");

        state.set_property("border-block-start", "1px", "");
        assert_eq!(state.get_property_value("border-block-start-width"), "1px");
        assert_eq!(
            state.get_property_value("border-block-start-style"),
            "initial"
        );
        assert_eq!(
            state.get_property_value("border-block-start-color"),
            "initial"
        );

        let mut state = DeclarationState::new();
        state.set_property("border", "1px none red", "");
        assert_eq!(state.get_property_value("border"), "1px red");

        state.set_property("border", "medium", "");
        assert_eq!(state.get_property_value("border"), "");
        assert_eq!(
            state.css_text(),
            "border-width: medium; border-style: none; border-color: currentcolor; border-image: none;"
        );

        state.set_property("border", "none", "");
        assert_eq!(state.serialize_safe().unwrap(), "border: none;");
    }

    #[test]
    fn safe_projections_reparse_idempotently_for_single_value_edge_cases() {
        for (property, input, expected) in [
            ("perspective", "0px", "perspective: 0px;"),
            ("-webkit-perspective", "0px", "perspective: 0px;"),
            ("place-self", "stretch auto", "place-self: stretch auto;"),
            ("place-self", "auto auto", "place-self: auto;"),
            ("-webkit-background-size", "auto", "background-size: auto;"),
        ] {
            let mut state = DeclarationState::new();
            assert_eq!(
                state.set_property(property, input, ""),
                MutationOutcome::Applied,
                "{property}"
            );
            assert_eq!(state.serialize_safe().unwrap(), expected, "{property}");

            let mut reparsed = DeclarationState::new();
            reparsed.replace_css_text(&state.serialize_safe().unwrap());
            assert_eq!(
                reparsed.serialize_safe().unwrap(),
                expected,
                "{property} reparse"
            );
        }
    }

    #[test]
    fn background_shorthand_retains_box_and_size_grammar() {
        let mut state = DeclarationState::new();
        assert_eq!(
            state.set_property("background", "none, content-box", ""),
            MutationOutcome::Applied,
        );
        assert_eq!(
            state.get_property_value("background"),
            "none, content-box content-box"
        );

        state.set_property("background", "center / 1px 2px, none", "");
        assert_eq!(
            state.get_property_value("background-size"),
            "1px 2px, initial"
        );
        assert_eq!(
            state.get_property_value("background"),
            "center center / 1px 2px, none"
        );

        state.set_property("background", "center / calc(1px + 5%), none", "");
        assert_eq!(
            state.get_property_value("background-size"),
            "calc(5% + 1px), initial"
        );
        assert_eq!(
            state.get_property_value("background"),
            "center center / calc(5% + 1px), none"
        );

        state.set_property("background", "none, image-set(url(a.png) 1x)", "");
        assert_eq!(
            crate::observable::project_observable_value(
                "background-image",
                "image-set(url(a.png) 1x)"
            ),
            Some("image-set(url(\"a.png\") 1x)".to_owned())
        );
        assert_eq!(
            state.get_property_value("background-image"),
            "none, image-set(url(\"a.png\") 1x)"
        );
        assert_eq!(
            state.get_property_value("background"),
            "none, image-set(url(\"a.png\") 1x)"
        );

        state.set_property(
            "background",
            "image-set(url(a.png) 1x) center/cover no-repeat red",
            "",
        );
        state.set_property("background-position-x", "right", "");
        assert_eq!(
            state.get_property_value("background"),
            "image-set(url(\"a.png\") 1x) right center / cover no-repeat red"
        );
    }
}
