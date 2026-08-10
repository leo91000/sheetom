use crate::{
    catalog::{canonical_property_name, initial_longhand_value, shorthand_longhands},
    inspect_property, sheetom_parser_property_name, EngineError, PropertyParseKind,
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

    if !validate_typed_shorthand_structure(name, value) {
        return Err(MutationOutcome::InvalidValue);
    }

    if let Some(longhands) = expand_special_shorthand(name, value, important) {
        return Ok(ParsedValue {
            observable_value: value.trim().to_owned(),
            safe_value: value.trim().to_owned(),
            longhands: Some(longhands),
        });
    }

    let inspection = inspect_property(name, value);
    if !inspection.as_ref().is_ok_and(|candidate| {
        matches!(
            candidate.kind,
            PropertyParseKind::Typed | PropertyParseKind::SheetomTyped
        )
    }) {
        if let Some(longhands) = expand_structural_shorthand(name, value, important) {
            return Ok(ParsedValue {
                observable_value: value.trim().to_owned(),
                safe_value: value.trim().to_owned(),
                longhands: Some(longhands),
            });
        }
        return Err(MutationOutcome::InvalidValue);
    }
    let inspection = inspection.map_err(map_engine_error)?;

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
        let canonical_value =
            if let Some(longhand) = shorthand_longhand(&property, name, longhand_name) {
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

fn expand_special_shorthand(
    name: &str,
    value: &str,
    important: bool,
) -> Option<Vec<DeclarationRecord>> {
    let components = split_top_level_whitespace(value)?;
    let values = match name {
        "columns" | "-webkit-columns" => expand_columns(&components)?,
        "font-synthesis" => expand_font_synthesis(&components)?,
        "font-variant" => expand_font_variant(&components)?,
        "offset" => expand_offset(value)?,
        "position-try" if value == "none" => vec![
            ("position-try-order", "normal".to_owned()),
            ("position-try-fallbacks", "none".to_owned()),
        ],
        "scroll-timeline" => expand_scroll_timeline(value)?,
        "text-box" => expand_text_box(&components)?,
        "text-wrap" => expand_text_wrap(&components)?,
        "transition" | "-webkit-transition" => expand_transition(value)?,
        "timeline-trigger" if value == "none" => vec![
            ("timeline-trigger-name", "none".to_owned()),
            ("timeline-trigger-source", "auto".to_owned()),
            (
                "timeline-trigger-activation-range-start",
                "normal".to_owned(),
            ),
            ("timeline-trigger-activation-range-end", "normal".to_owned()),
            ("timeline-trigger-active-range-start", "auto".to_owned()),
            ("timeline-trigger-active-range-end", "auto".to_owned()),
        ],
        "view-timeline" => expand_view_timeline(value)?,
        "white-space" => expand_white_space(&components)?,
        _ => return None,
    };
    records_from_values(name, values, important)
}

fn records_from_values(
    shorthand: &str,
    values: Vec<(&str, String)>,
    important: bool,
) -> Option<Vec<DeclarationRecord>> {
    let longhands = shorthand_longhands(shorthand)?;
    if values.len() != longhands.len() {
        return None;
    }
    longhands
        .iter()
        .map(|longhand| {
            let value = values
                .iter()
                .find_map(|(name, value)| (*name == *longhand).then_some(value))?;
            Some(DeclarationRecord {
                name: (*longhand).to_owned(),
                observable_value: value.clone(),
                safe_value: value.clone(),
                important,
            })
        })
        .collect()
}

fn expand_columns(components: &[&str]) -> Option<Vec<(&'static str, String)>> {
    if components.is_empty() || components.len() > 2 {
        return None;
    }
    let mut width = "auto".to_owned();
    let mut count = "auto".to_owned();
    for component in components {
        if *component == "auto" {
            continue;
        }
        if let Some(canonical) = validate_column_width(component).filter(|_| width == "auto") {
            width = canonical;
            continue;
        }
        if let Some(canonical) = validate_column_count(component).filter(|_| count == "auto") {
            count = canonical;
            continue;
        }
        return None;
    }
    Some(vec![
        ("column-width", width),
        ("column-count", count),
        ("column-height", "auto".to_owned()),
        ("column-wrap", "auto".to_owned()),
    ])
}

fn validate_column_width(value: &str) -> Option<String> {
    typed_longhand_value("width", value)
}

fn validate_column_count(value: &str) -> Option<String> {
    let count = value.parse::<u32>().ok()?;
    (count > 0).then(|| count.to_string())
}

fn expand_font_synthesis(components: &[&str]) -> Option<Vec<(&'static str, String)>> {
    if components.is_empty() {
        return None;
    }
    let none = components == ["none"];
    if !none
        && components
            .iter()
            .any(|component| !matches!(*component, "weight" | "style" | "small-caps"))
    {
        return None;
    }
    let enabled = |component| {
        if !none && components.contains(&component) {
            "auto"
        } else {
            "none"
        }
        .to_owned()
    };
    Some(vec![
        ("font-synthesis-weight", enabled("weight")),
        ("font-synthesis-style", enabled("style")),
        ("font-synthesis-small-caps", enabled("small-caps")),
    ])
}

fn expand_font_variant(components: &[&str]) -> Option<Vec<(&'static str, String)>> {
    if components != ["normal"] {
        return None;
    }
    Some(
        shorthand_longhands("font-variant")?
            .iter()
            .map(|longhand| (*longhand, "normal".to_owned()))
            .collect(),
    )
}

fn expand_offset(value: &str) -> Option<Vec<(&'static str, String)>> {
    if value == "normal" {
        return Some(vec![
            ("offset-position", "normal".to_owned()),
            ("offset-path", "none".to_owned()),
            ("offset-distance", "0px".to_owned()),
            ("offset-rotate", "auto".to_owned()),
            ("offset-anchor", "auto".to_owned()),
        ]);
    }
    let slash = split_top_level_delimiter(value, b'/')?;
    if slash.is_empty() || slash.len() > 2 {
        return None;
    }
    let mut path = "none".to_owned();
    let mut distance = "0px".to_owned();
    let mut rotation = Vec::new();
    let mut position = Vec::new();
    for component in split_top_level_whitespace(slash[0])? {
        if path == "none" {
            if let Some(canonical) = offset_path_value(component) {
                path = canonical;
                continue;
            }
        }
        if distance == "0px" {
            if let Some(canonical) = offset_distance_value(component) {
                distance = canonical;
                continue;
            }
        }
        if matches!(component, "auto" | "reverse")
            || typed_longhand_value("rotate", component).is_some()
        {
            rotation.push(component);
            continue;
        }
        position.push(component);
    }
    if path == "none" && position.is_empty() {
        return None;
    }
    let offset_position = if position.is_empty() {
        "normal".to_owned()
    } else {
        typed_longhand_value("offset-position", &position.join(" "))?
    };
    let offset_rotate = if rotation.is_empty() {
        "auto".to_owned()
    } else {
        offset_rotate_value(&rotation)?
    };
    let offset_anchor = if let Some(anchor) = slash.get(1) {
        typed_longhand_value("offset-anchor", anchor)?
    } else {
        "auto".to_owned()
    };
    Some(vec![
        ("offset-position", offset_position),
        ("offset-path", path),
        ("offset-distance", distance),
        ("offset-rotate", offset_rotate),
        ("offset-anchor", offset_anchor),
    ])
}

fn offset_path_value(value: &str) -> Option<String> {
    typed_longhand_value("offset-path", value).or_else(|| {
        (value == "none"
            || [
                "path(", "ray(", "url(", "circle(", "ellipse(", "inset(", "polygon(",
            ]
            .iter()
            .any(|prefix| value.starts_with(prefix)))
        .then(|| value.to_owned())
    })
}

fn offset_distance_value(value: &str) -> Option<String> {
    typed_longhand_value("offset-distance", value).or_else(|| typed_longhand_value("width", value))
}

fn offset_rotate_value(components: &[&str]) -> Option<String> {
    if components.is_empty() || components.len() > 2 {
        return None;
    }
    let mut keyword = None;
    let mut angle = None;
    for component in components {
        if matches!(*component, "auto" | "reverse") {
            if keyword.replace(*component).is_some() {
                return None;
            }
        } else if typed_longhand_value("rotate", component).is_some() {
            if angle.replace(*component).is_some() {
                return None;
            }
        } else {
            return None;
        }
    }
    Some(components.join(" "))
}

fn typed_longhand_value(name: &str, value: &str) -> Option<String> {
    let inspection = inspect_property(name, value).ok()?;
    matches!(
        inspection.kind,
        PropertyParseKind::Typed | PropertyParseKind::SheetomTyped
    )
    .then_some(inspection.canonical_value)
}

fn expand_scroll_timeline(value: &str) -> Option<Vec<(&'static str, String)>> {
    let mut names = Vec::new();
    let mut axes = Vec::new();
    for entry in split_top_level_delimiter(value, b',')? {
        let components = split_top_level_whitespace(entry)?;
        if components.is_empty() || components.len() > 2 {
            return None;
        }
        let axis = components
            .iter()
            .find(|component| matches!(**component, "block" | "inline" | "x" | "y"))
            .copied()
            .unwrap_or("block");
        let name = components
            .iter()
            .find(|component| !matches!(**component, "block" | "inline" | "x" | "y"))
            .copied()?;
        if name != "none" && !name.starts_with("--") {
            return None;
        }
        names.push(name);
        axes.push(axis);
    }
    Some(vec![
        ("scroll-timeline-name", names.join(", ")),
        ("scroll-timeline-axis", axes.join(", ")),
    ])
}

fn expand_text_box(components: &[&str]) -> Option<Vec<(&'static str, String)>> {
    if components == ["normal"] {
        return Some(vec![
            ("text-box-trim", "none".to_owned()),
            ("text-box-edge", "auto".to_owned()),
        ]);
    }
    if components.is_empty() || components.len() > 3 {
        return None;
    }
    let trim_keywords = ["none", "trim-start", "trim-end", "trim-both"];
    let (trim, edges) = if trim_keywords.contains(&components[0]) {
        (components[0], &components[1..])
    } else {
        ("trim-both", components)
    };
    if edges.is_empty() || edges.len() > 2 {
        return None;
    }
    Some(vec![
        ("text-box-trim", trim.to_owned()),
        ("text-box-edge", edges.join(" ")),
    ])
}

fn expand_text_wrap(components: &[&str]) -> Option<Vec<(&'static str, String)>> {
    if components.is_empty() || components.len() > 2 {
        return None;
    }
    let modes = ["wrap", "nowrap"];
    let styles = ["auto", "balance", "pretty", "stable"];
    let mode_values = components
        .iter()
        .filter(|component| modes.contains(component))
        .copied()
        .collect::<Vec<_>>();
    let style_values = components
        .iter()
        .filter(|component| styles.contains(component))
        .copied()
        .collect::<Vec<_>>();
    if mode_values.len() > 1
        || style_values.len() > 1
        || mode_values.len() + style_values.len() != components.len()
    {
        return None;
    }
    let mode = mode_values.first().copied().unwrap_or("initial");
    let style = style_values.first().copied().unwrap_or("initial");
    Some(vec![
        ("text-wrap-mode", mode.to_owned()),
        ("text-wrap-style", style.to_owned()),
    ])
}

fn expand_view_timeline(value: &str) -> Option<Vec<(&'static str, String)>> {
    let mut names = Vec::new();
    let mut axes = Vec::new();
    let mut insets = Vec::new();
    for entry in split_top_level_delimiter(value, b',')? {
        let mut components = split_top_level_whitespace(entry)?;
        let name = components.first().copied()?;
        if name != "none" && !name.starts_with("--") {
            return None;
        }
        components.remove(0);
        let axis_index = components
            .iter()
            .position(|component| matches!(*component, "block" | "inline" | "x" | "y"));
        let axis = axis_index
            .map(|index| components.remove(index))
            .unwrap_or("block");
        if components.len() > 2 {
            return None;
        }
        names.push(name);
        axes.push(axis);
        insets.push(if components.is_empty() {
            "auto".to_owned()
        } else {
            components.join(" ")
        });
    }
    Some(vec![
        ("view-timeline-name", names.join(", ")),
        ("view-timeline-axis", axes.join(", ")),
        ("view-timeline-inset", insets.join(", ")),
    ])
}

fn expand_transition(value: &str) -> Option<Vec<(&'static str, String)>> {
    let mut properties = Vec::new();
    let mut durations = Vec::new();
    let mut timings = Vec::new();
    let mut delays = Vec::new();
    let mut behaviors = Vec::new();
    for entry in split_top_level_delimiter(value, b',')? {
        let mut property = None;
        let mut duration = None;
        let mut timing = None;
        let mut delay = None;
        let mut behavior = None;
        for component in split_top_level_whitespace(entry)? {
            if matches!(component, "normal" | "allow-discrete") {
                if behavior.replace(component).is_some() {
                    return None;
                }
                continue;
            }
            if let Some(canonical) = typed_longhand_value("transition-duration", component) {
                if duration.is_none() {
                    duration = Some(canonical);
                } else if delay.is_none() {
                    delay = Some(canonical);
                } else {
                    return None;
                }
                continue;
            }
            if let Some(canonical) = typed_longhand_value("transition-timing-function", component) {
                if timing.replace(canonical).is_some() {
                    return None;
                }
                continue;
            }
            if typed_longhand_value("transition-property", component).is_some() {
                if property.replace(component.to_owned()).is_some() {
                    return None;
                }
                continue;
            }
            return None;
        }
        properties.push(property.unwrap_or_else(|| "all".to_owned()));
        durations.push(duration.unwrap_or_else(|| "0s".to_owned()));
        timings.push(timing.unwrap_or_else(|| "ease".to_owned()));
        delays.push(delay.unwrap_or_else(|| "0s".to_owned()));
        behaviors.push(behavior.unwrap_or("normal"));
    }
    Some(vec![
        ("transition-property", properties.join(", ")),
        ("transition-duration", durations.join(", ")),
        ("transition-timing-function", timings.join(", ")),
        ("transition-delay", delays.join(", ")),
        ("transition-behavior", behaviors.join(", ")),
    ])
}

fn expand_white_space(components: &[&str]) -> Option<Vec<(&'static str, String)>> {
    let (collapse, mode) = match components {
        ["normal"] => ("collapse", "wrap"),
        ["pre"] => ("preserve", "nowrap"),
        ["pre-wrap"] => ("preserve", "wrap"),
        ["pre-line"] => ("preserve-breaks", "wrap"),
        ["nowrap"] => ("collapse", "nowrap"),
        ["break-spaces"] => ("break-spaces", "wrap"),
        [first, second]
            if matches!(
                *first,
                "collapse" | "preserve" | "preserve-breaks" | "break-spaces"
            ) && matches!(*second, "wrap" | "nowrap") =>
        {
            (*first, *second)
        }
        _ => return None,
    };
    Some(vec![
        ("white-space-collapse", collapse.to_owned()),
        ("text-wrap-mode", mode.to_owned()),
    ])
}

fn validate_typed_shorthand_structure(name: &str, value: &str) -> bool {
    if matches!(name, "mask" | "-webkit-mask") {
        return split_top_level_delimiter(value, b',').is_some();
    }
    if name != "background" {
        return true;
    }
    let Some(layers) = split_top_level_delimiter(value, b',') else {
        return false;
    };
    for layer in layers.iter().take(layers.len().saturating_sub(1)) {
        let Some(components) = split_top_level_whitespace(layer) else {
            return false;
        };
        if components
            .iter()
            .any(|component| typed_longhand_value("color", component).is_some())
        {
            return false;
        }
    }
    true
}

fn shorthand_longhand<'i>(
    property: &Property<'i>,
    shorthand_name: &str,
    longhand_name: &str,
) -> Option<Property<'i>> {
    let direct = PropertyId::from(longhand_name);
    if let Some(longhand) = property.longhand(&direct) {
        return Some(longhand);
    }
    if sheetom_parser_property_name(shorthand_name) != Some("border") {
        return None;
    }

    let source_name = if longhand_name.ends_with("-width") {
        "border-top-width"
    } else if longhand_name.ends_with("-style") {
        "border-top-style"
    } else if longhand_name.ends_with("-color") {
        "border-top-color"
    } else {
        return None;
    };
    property.longhand(&PropertyId::from(source_name))
}

fn parse_typed_property<'i>(name: &'i str, value: &'i str) -> Result<Property<'i>, EngineError> {
    let parser_name = sheetom_parser_property_name(name).unwrap_or(name);
    Property::parse_string(
        PropertyId::from(parser_name),
        value,
        ParserOptions::default(),
    )
    .map_err(|error| EngineError::Parse(error.to_string()))
}

fn map_engine_error(_: EngineError) -> MutationOutcome {
    MutationOutcome::InvalidValue
}

fn expand_structural_shorthand(
    name: &str,
    value: &str,
    important: bool,
) -> Option<Vec<DeclarationRecord>> {
    let longhands = shorthand_longhands(name)?;
    let maximum = structural_cardinality(name, longhands.len())?;
    let components = split_top_level_whitespace(value)?;
    if components.is_empty() || components.len() > maximum {
        return None;
    }
    let expanded = expand_repeated_components(&components, longhands.len())?;
    let mut records = Vec::with_capacity(longhands.len());
    for (longhand, component) in longhands.iter().zip(expanded) {
        let canonical = validate_structural_longhand(longhand, component)?;
        records.push(DeclarationRecord {
            name: (*longhand).to_owned(),
            observable_value: canonical.clone(),
            safe_value: canonical,
            important,
        });
    }
    Some(records)
}

fn structural_cardinality(name: &str, longhand_count: usize) -> Option<usize> {
    const TWO_VALUE: &[&str] = &[
        "contain-intrinsic-size",
        "interest-delay",
        "overscroll-behavior",
        "rule-break",
        "rule-color",
        "rule-style",
        "rule-visibility-items",
        "timeline-trigger-activation-range",
        "timeline-trigger-active-range",
    ];
    const FOUR_VALUE: &[&str] = &[
        "column-rule-inset",
        "column-rule-inset-cap",
        "column-rule-inset-end",
        "column-rule-inset-junction",
        "column-rule-inset-start",
        "corner-block-end-shape",
        "corner-block-start-shape",
        "corner-bottom-shape",
        "corner-inline-end-shape",
        "corner-inline-start-shape",
        "corner-left-shape",
        "corner-right-shape",
        "corner-shape",
        "corner-top-shape",
        "row-rule-inset",
        "row-rule-inset-cap",
        "row-rule-inset-end",
        "row-rule-inset-junction",
        "row-rule-inset-start",
        "rule-inset",
        "rule-inset-cap",
        "rule-inset-end",
        "rule-inset-junction",
        "rule-inset-start",
    ];

    if TWO_VALUE.contains(&name) {
        return Some(2.min(longhand_count));
    }
    if FOUR_VALUE.contains(&name) {
        return Some(4.min(longhand_count));
    }
    None
}

fn expand_repeated_components<'a>(
    components: &[&'a str],
    longhand_count: usize,
) -> Option<Vec<&'a str>> {
    let expanded = match (components, longhand_count) {
        ([first], count) => vec![*first; count],
        ([first, second], 2) => vec![*first, *second],
        ([first, second], 4) => vec![*first, *second, *first, *second],
        ([first, second, third], 4) => vec![*first, *second, *third, *second],
        ([first, second, third, fourth], 4) => {
            vec![*first, *second, *third, *fourth]
        }
        ([first, second], 8) => {
            vec![
                *first, *second, *first, *second, *first, *second, *first, *second,
            ]
        }
        ([first, second, third], 8) => {
            vec![
                *first, *second, *third, *second, *first, *second, *third, *second,
            ]
        }
        ([first, second, third, fourth], 8) => vec![
            *first, *second, *third, *fourth, *first, *second, *third, *fourth,
        ],
        _ => return None,
    };
    Some(expanded)
}

fn validate_structural_longhand(name: &str, value: &str) -> Option<String> {
    if let Ok(inspection) = inspect_property(name, value) {
        if matches!(
            inspection.kind,
            PropertyParseKind::Typed | PropertyParseKind::SheetomTyped
        ) {
            return Some(inspection.canonical_value);
        }
    }

    let validation_name = if name.starts_with("overscroll-behavior-") {
        return matches!(value, "auto" | "contain" | "none").then(|| value.to_owned());
    } else if name.contains("rule-inset") {
        Some("padding-top")
    } else if name.ends_with("rule-break") {
        return matches!(value, "normal" | "none" | "spanning-item").then(|| value.to_owned());
    } else if name.ends_with("rule-visibility-items") {
        return matches!(value, "normal" | "none" | "all").then(|| value.to_owned());
    } else if name.starts_with("corner-") && name.ends_with("-shape") {
        return matches!(
            value,
            "round" | "scoop" | "bevel" | "notch" | "square" | "squircle"
        )
        .then(|| value.to_owned());
    } else if name.starts_with("contain-intrinsic-") {
        return (value == "none").then(|| value.to_owned());
    } else if name.starts_with("interest-delay-") {
        if value == "normal" {
            return Some(value.to_owned());
        }
        Some("animation-duration")
    } else if name.starts_with("timeline-trigger-") && name.contains("-range-") {
        return matches!(value, "normal" | "auto").then(|| value.to_owned());
    } else {
        None
    };

    let inspection = inspect_property(validation_name?, value).ok()?;
    matches!(
        inspection.kind,
        PropertyParseKind::Typed | PropertyParseKind::SheetomTyped
    )
    .then_some(inspection.canonical_value)
}

fn split_top_level_whitespace(value: &str) -> Option<Vec<&str>> {
    split_top_level(value, |byte| byte.is_ascii_whitespace(), false)
}

fn split_top_level_delimiter(value: &str, delimiter: u8) -> Option<Vec<&str>> {
    split_top_level(value, |byte| byte == delimiter, true)
}

fn split_top_level(
    value: &str,
    is_separator: impl Fn(u8) -> bool,
    require_nonempty_parts: bool,
) -> Option<Vec<&str>> {
    let bytes = value.as_bytes();
    let mut components = Vec::new();
    let mut start = None;
    let mut depth = 0usize;
    let mut index = 0usize;
    let mut quote = None;
    let mut in_comment = false;

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
            if byte == delimiter {
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
            start.get_or_insert(index);
            index += 1;
            continue;
        }
        if matches!(byte, b'(' | b'[' | b'{') {
            depth += 1;
            start.get_or_insert(index);
            index += 1;
            continue;
        }
        if matches!(byte, b')' | b']' | b'}') {
            if depth == 0 {
                return None;
            }
            depth -= 1;
            index += 1;
            continue;
        }
        if depth == 0 && is_separator(byte) {
            if let Some(component_start) = start.take() {
                let component = value[component_start..index].trim();
                if !component.is_empty() {
                    components.push(component);
                }
            } else if require_nonempty_parts {
                return None;
            }
            index += 1;
            continue;
        }
        start.get_or_insert(index);
        index += 1;
    }
    if depth != 0 || quote.is_some() || in_comment {
        return None;
    }
    if let Some(component_start) = start {
        let component = value[component_start..].trim();
        if !component.is_empty() {
            components.push(component);
        }
    } else if require_nonempty_parts {
        return None;
    }
    Some(components)
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
            "#00f"
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
        assert_eq!(state.get_property_value("row-rule-color"), "#00f");

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
        let failures = cases
            .iter()
            .filter_map(|case| {
                let property = case["property"].as_str()?;
                let input = case["input"].as_str()?;
                let mut state = DeclarationState::new();
                let outcome = state.set_property(property, input, "");
                (outcome != MutationOutcome::Applied)
                    .then(|| format!("{property}: {input} ({outcome:?})"))
            })
            .collect::<Vec<_>>();
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
        let mut failures = Vec::new();
        for case in profiles
            .iter()
            .flat_map(|profile| profile["cases"].as_array().into_iter().flatten())
        {
            let id = case["id"].as_str().unwrap_or("missing-id");
            let property = case["property"].as_str().unwrap_or_default();
            let input = case["input"].as_str().unwrap_or_default();
            let expected = case["accepted"].as_bool().unwrap_or(false);
            let mut state = DeclarationState::new();
            let outcome = state.set_property(property, input, "");
            let accepted = outcome == MutationOutcome::Applied;
            if accepted != expected {
                failures.push(format!(
                    "{id}: expected accepted={expected}, got {outcome:?}"
                ));
            }
        }
        assert!(
            failures.is_empty(),
            "Reviewed Chromium grammar branches that diverged:\n{}",
            failures.join("\n")
        );
    }
}
