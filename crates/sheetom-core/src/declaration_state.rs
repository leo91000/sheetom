use crate::function_rule::{canonical_function_descriptor_name, parse_function_descriptor_value};
use crate::{
    catalog::{
        canonical_property_name as canonical_style_property_name,
        property_alias_defers_pending_value, property_alias_hides_value,
        shorthand_longhands as style_shorthand_longhands, shorthand_names,
    },
    font_face::{canonical_descriptor_name, parse_descriptor_value},
    shorthand::{
        parse_value_for_source_with_limits, synthesize_authored_shorthand, synthesize_shorthand,
    },
    syntax::{parse_declaration_list, serialize_identifier},
    validate_declaration_block_input, validate_declaration_value_input, DeclarationValue,
    EngineError, ResourceLimits,
};
use std::collections::{HashMap, HashSet};

#[derive(Clone, Debug, PartialEq)]
pub struct PendingSubstitutionGroup {
    pub(crate) id: u64,
    pub(crate) shorthand: String,
    pub(crate) value: DeclarationValue,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DeclarationRecord {
    pub name: String,
    pub value: DeclarationValue,
    pub important: bool,
    pub pending_group: Option<PendingSubstitutionGroup>,
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

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MutationOutcome {
    Applied,
    InvalidName,
    InvalidPriority,
    InvalidValue,
    UnsupportedShorthand,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
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
        let queried_name = name.to_ascii_lowercase();
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
                record.observable_value().to_owned()
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
        let mut candidate = self.clone();
        candidate.limits = parse_limits;
        let outcome = candidate.apply_property(name, value, priority);
        if outcome != MutationOutcome::Applied {
            return Ok(outcome);
        }
        candidate.validate_record_limit()?;
        candidate.limits = self.limits;
        *self = candidate;
        Ok(outcome)
    }

    fn apply_property(&mut self, name: &str, value: &str, priority: &str) -> MutationOutcome {
        let source_name = name.to_ascii_lowercase();
        let Some(name) = self.canonical_name(name) else {
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
        if self.context == DeclarationContext::Function && name == "result" {
            return MutationOutcome::Applied;
        }

        let important = priority == "important";
        let mut parsed = match self.parse_value(&name, &source_name, value, important) {
            Ok(parsed) => parsed,
            Err(outcome) => return outcome,
        };

        if parsed.pending_substitution() {
            if let Some(longhands) = self.shorthand_longhands(&name) {
                let group = self.new_pending_group(name, parsed.value.clone());
                for longhand in longhands {
                    self.commit(DeclarationRecord {
                        name: (*longhand).to_owned(),
                        value: DeclarationValue::deferred(true),
                        important,
                        pending_group: Some(group.clone()),
                        alias_value: None,
                    });
                }
                return MutationOutcome::Applied;
            }
        }

        if let Some(mut longhands) = parsed.longhands.take() {
            self.attach_static_group(&name, &parsed, &mut longhands);
            for record in longhands {
                self.commit(record);
            }
            return MutationOutcome::Applied;
        }

        let alias_value = self.alias_value_for_record(&source_name, &name, &parsed.value);
        let record_value = if alias_value.is_some() {
            DeclarationValue::deferred(true)
        } else {
            parsed.value
        };
        self.commit(DeclarationRecord {
            name,
            value: record_value,
            important,
            pending_group: None,
            alias_value,
        });
        MutationOutcome::Applied
    }

    pub fn replace_declarations(&mut self, declarations: &[ParsedDeclaration]) {
        let mut winners = HashMap::<String, (DeclarationRecord, usize, usize)>::new();

        for (source_index, declaration) in declarations.iter().enumerate() {
            if self.context == DeclarationContext::Function && declaration.important {
                continue;
            }
            let Some(name) = self.canonical_name(&declaration.name) else {
                continue;
            };
            let source_name = declaration.name.to_ascii_lowercase();
            let Ok(parsed) = self.parse_value(
                &name,
                &source_name,
                &declaration.value,
                declaration.important,
            ) else {
                continue;
            };
            let records =
                self.records_for_parsed(name, parsed, declaration.important, &source_name);
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
        records.sort_by_key(|(record, source_index, sub_index)| {
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
        let declarations = parse_declaration_list(source)
            .into_iter()
            .map(|declaration| ParsedDeclaration {
                name: declaration.name,
                value: declaration.value,
                important: declaration.important,
            })
            .collect::<Vec<_>>();
        for declaration in &declarations {
            validate_declaration_value_input(&declaration.value, parse_limits)?;
        }
        let mut candidate = self.clone();
        candidate.limits = parse_limits;
        candidate.replace_declarations(&declarations);
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
        self.serialize(false)
    }

    pub fn serialize_safe(&self) -> String {
        self.serialize(true)
    }

    pub fn serialize_formatted(&self, safe: bool, indent: &str, separator: &str) -> String {
        self.serialized_declarations(safe)
            .into_iter()
            .map(|declaration| format!("{indent}{declaration}"))
            .collect::<Vec<_>>()
            .join(separator)
    }

    fn find(&self, name: &str) -> Option<&DeclarationRecord> {
        self.records.iter().find(|record| record.name == name)
    }

    fn canonical_name(&self, name: &str) -> Option<String> {
        match self.context {
            DeclarationContext::Style => canonical_style_property_name(name),
            DeclarationContext::FontFace => canonical_descriptor_name(name),
            DeclarationContext::Function => canonical_function_descriptor_name(name),
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
        match self.context {
            DeclarationContext::Style => {
                parse_value_for_source_with_limits(name, source_name, value, important, self.limits)
            }
            DeclarationContext::FontFace => parse_descriptor_value(name, value, self.limits)
                .ok_or(MutationOutcome::InvalidValue),
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
    ) -> Vec<DeclarationRecord> {
        if parsed.pending_substitution() {
            if let Some(longhands) = self.shorthand_longhands(&name) {
                let group = self.new_pending_group(name, parsed.value.clone());
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
            return longhands;
        }
        let alias_value = self.alias_value_for_record(source_name, &name, &parsed.value);
        let value = if alias_value.is_some() {
            DeclarationValue::deferred(true)
        } else {
            parsed.value
        };
        vec![DeclarationRecord {
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
    ) -> PendingSubstitutionGroup {
        let id = self.next_pending_group_id;
        self.next_pending_group_id = self.next_pending_group_id.wrapping_add(1);
        PendingSubstitutionGroup {
            id,
            shorthand,
            value,
        }
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
        let safe_value = normalize_rgb_function_spacing(&safe_value);
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
        let group = self.new_pending_group(name.to_owned(), group_value);
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
                if !record.pending_substitution() {
                    let observable = if is_gap_rule_longhand(&record.name) {
                        record.observable_value().to_owned()
                    } else {
                        materialize_static_observable(
                            record.observable_value(),
                            record.safe_value(),
                        )
                    };
                    record.value.replace_observable(observable);
                }
                record.pending_group = None;
            }
        }
    }

    fn serialize(&self, safe: bool) -> String {
        self.serialized_declarations(safe).join(" ")
    }

    fn serialized_declarations(&self, safe: bool) -> Vec<String> {
        if matches!(
            self.context,
            DeclarationContext::FontFace | DeclarationContext::Function
        ) {
            return self
                .records
                .iter()
                .map(|record| {
                    let name = if record.name.starts_with("--") {
                        serialize_identifier(&record.name)
                    } else {
                        record.name.clone()
                    };
                    let value = if safe
                        || self.context == DeclarationContext::FontFace
                            && record.name == "font-variant"
                    {
                        record.safe_value()
                    } else {
                        record.observable_value()
                    };
                    format_declaration(&name, value, record.important)
                })
                .collect();
        }

        #[derive(Clone)]
        struct Candidate {
            name: String,
            value: String,
            important: bool,
            longhands: &'static [&'static str],
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
                Some(Candidate {
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
        let mut declarations = Vec::new();
        for record in &self.records {
            if let Some(index) = by_longhand.get(record.name.as_str()) {
                let candidate = &candidates[*index];
                if written.insert(candidate.name.as_str()) {
                    declarations.push(format_declaration(
                        &candidate.name,
                        &candidate.value,
                        candidate.important,
                    ));
                }
                continue;
            }
            let name = if record.name.starts_with("--") {
                serialize_identifier(&record.name)
            } else {
                record.name.clone()
            };
            let value = if safe {
                record.safe_value()
            } else {
                record.observable_value()
            };
            declarations.push(format_declaration(&name, value, record.important));
        }
        declarations
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
    matches!(
        name,
        "animation"
            | "background"
            | "column-rule"
            | "border-image"
            | "columns"
            | "container"
            | "flex"
            | "flex-flow"
            | "font-variant"
            | "grid-area"
            | "grid-column"
            | "grid-row"
            | "grid"
            | "grid-template"
            | "mask"
            | "offset"
            | "row-rule"
            | "rule"
            | "rule-color"
            | "rule-style"
            | "rule-width"
            | "scroll-timeline"
            | "text-box"
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
        "border-image"
            | "font-variant"
            | "flex-flow"
            | "grid-area"
            | "grid-column"
            | "grid-row"
            | "timeline-trigger"
            | "timeline-trigger-activation-range"
            | "timeline-trigger-active-range"
    ) || name.ends_with("-color")
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
    observable_parts
        .iter()
        .zip(safe_parts)
        .map(|(observable, safe)| {
            if *observable == "initial" {
                "initial"
            } else {
                safe
            }
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn normalize_rgb_function_spacing(value: &str) -> String {
    let lower = value.to_ascii_lowercase();
    let function = if lower.starts_with("rgb(") {
        "rgb"
    } else if lower.starts_with("rgba(") {
        "rgba"
    } else {
        return value.to_owned();
    };
    let Some(body) = value
        .get(function.len() + 1..)
        .and_then(|body| body.strip_suffix(')'))
    else {
        return value.to_owned();
    };
    if !body.contains(',') {
        return value.to_owned();
    }
    format!(
        "{function}({})",
        body.split(',')
            .map(str::trim)
            .collect::<Vec<_>>()
            .join(", ")
    )
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
    border_like && matches!(observable, "none" | "currentcolor")
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
    name == "outline"
        && crate::syntax::split_top_level_whitespace(observable)
            .is_some_and(|components| components.len() > 1)
}

fn format_declaration(name: &str, value: &str, important: bool) -> String {
    let priority = if important { " !important" } else { "" };
    format!("{name}: {value}{priority};")
}

#[cfg(test)]
mod tests {
    use super::{DeclarationContext, DeclarationState, MutationOutcome, ParsedDeclaration};
    use crate::{DeclarationValueKind, EngineError, ResourceLimits};
    use serde_json::Value;

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
            state.set_property("WIDTH", "10px", "important"),
            MutationOutcome::Applied
        );
        assert_eq!(state.item(0), "color");
        assert_eq!(state.item(1), "width");

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
            state.serialize_safe(),
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

        for value in ["center", "left top"] {
            let mut state = DeclarationState::new();
            assert_eq!(
                state.set_property("position-try", value, ""),
                MutationOutcome::Applied,
                "{value}"
            );
            assert_eq!(
                state.get_property_value("position-try-fallbacks"),
                value,
                "{value}"
            );
            assert_eq!(state.get_property_value("position-try"), value, "{value}");
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
            reparsed.replace_css_text(&state.serialize_safe());
            assert_eq!(
                reparsed.serialize_safe(),
                state.serialize_safe(),
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
    fn simplified_numeric_calculations_remain_reparsably_idempotent() {
        for source in ["calc(1 / 2)", "calc(50%)"] {
            let mut state = DeclarationState::new();
            assert_eq!(
                state.set_property("opacity", source, ""),
                MutationOutcome::Applied,
                "{source}"
            );

            let serialized = state.serialize_safe();
            let mut reparsed = DeclarationState::new();
            reparsed.replace_css_text(&serialized);
            assert_eq!(reparsed.serialize_safe(), serialized, "{source}");
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
}
