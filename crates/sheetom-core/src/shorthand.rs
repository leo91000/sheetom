use crate::{
    catalog::{initial_longhand_value, observed_shorthand_longhands, shorthand_longhands},
    declaration_state::{DeclarationRecord, MutationOutcome},
    extension_value::{offset_rotate_is_shorthand_default, parse_contextual_dimension_calculation},
    gap_rule::{
        canonical_gap_rule_longhand, expand_border_side_observable, expand_gap_rule,
        expand_text_stroke, gap_rule_component, synthesize_gap_rule, GapRuleComponent,
    },
    observable::{project_declaration, project_observable_value},
    parse_semantic_property_with_limits, sheetom_parser_property_name,
    syntax::{
        analyze_substitutions, split_top_level_delimiter, split_top_level_delimiter_allow_empty,
        split_top_level_whitespace,
    },
    DeclarationValue, EngineError, PropertyParseKind, ResourceLimits,
};
use cssparser::{Parser, ParserInput, Token};
use lightningcss::{
    declaration::DeclarationBlock,
    properties::{Property, PropertyId},
    stylesheet::{ParserOptions, PrinterOptions},
    traits::IntoOwned,
};

pub(crate) struct ParsedValue {
    pub(crate) value: DeclarationValue,
    pub(crate) longhands: Option<Vec<DeclarationRecord>>,
}

impl ParsedValue {
    pub(crate) fn observable_value(&self) -> &str {
        self.value.observable_css()
    }

    pub(crate) fn safe_value(&self) -> &str {
        self.value.safe_css()
    }

    pub(crate) fn pending_substitution(&self) -> bool {
        self.value.is_pending_substitution()
    }
}

pub(crate) fn synthesize_shorthand(
    records: &[DeclarationRecord],
    name: &str,
    safe: bool,
) -> Option<String> {
    synthesize_shorthand_inner(records, name, safe, false)
}

pub(crate) fn synthesize_authored_shorthand(
    records: &[DeclarationRecord],
    name: &str,
    safe: bool,
) -> Option<String> {
    synthesize_shorthand_inner(records, name, safe, true)
}

fn synthesize_shorthand_inner(
    records: &[DeclarationRecord],
    name: &str,
    safe: bool,
    authored_expansion: bool,
) -> Option<String> {
    let longhands = observed_shorthand_longhands(name)?;
    let records = if name == "grid" {
        let records = records
            .iter()
            .filter(|record| longhands.contains(&record.name.as_str()))
            .collect::<Vec<_>>();
        (records.len() == longhands.len()).then_some(records)?
    } else {
        longhands
            .iter()
            .map(|longhand| records.iter().find(|record| record.name == *longhand))
            .collect::<Option<Vec<_>>>()?
    };
    let first = records.first()?;
    if records
        .iter()
        .any(|record| record.important != first.important)
    {
        return None;
    }

    if let Some(group) = &first.pending_group {
        if group.shorthand == name
            && records.iter().all(|record| {
                record
                    .pending_group
                    .as_ref()
                    .is_some_and(|other| other.id == group.id)
            })
        {
            return Some(if safe {
                group.value.safe_css().to_owned()
            } else {
                group.value.observable_css().to_owned()
            });
        }
    }
    let has_grouped_records = records.iter().any(|record| record.pending_group.is_some());
    let can_synthesize_static_rule_inset =
        name.contains("rule-inset") && records.iter().all(|record| !record.pending_substitution());
    if has_grouped_records && !can_synthesize_static_rule_inset {
        return None;
    }

    let values = records
        .iter()
        .map(|record| {
            if safe {
                record.safe_value()
            } else {
                record.observable_value()
            }
        })
        .collect::<Vec<_>>();
    let has_css_wide = records.iter().any(|record| {
        let selected = if safe {
            record.safe_value()
        } else {
            record.observable_value()
        };
        is_css_wide_keyword(selected)
            && record.safe_value() == record.observable_value()
            && is_css_wide_keyword(record.safe_value())
    });
    if let Some(first_value) = values.first() {
        if has_css_wide {
            if values.iter().all(|value| *value == *first_value) {
                return Some((*first_value).to_owned());
            }
            if !authored_expansion {
                return None;
            }
        }
    }

    let special = synthesize_special_shorthand(name, &records, safe);
    if special.is_some() || has_authoritative_shorthand_synthesis(name) {
        return special;
    }

    synthesize_typed_shorthand(&records, name, safe)
}

fn synthesize_typed_shorthand(
    records: &[&DeclarationRecord],
    name: &str,
    safe: bool,
) -> Option<String> {
    let mut declarations = DeclarationBlock::new();
    for record in records {
        let value = if safe {
            record.safe_value()
        } else {
            record.observable_value()
        };
        let property = parse_typed_property(&record.name, value).ok()?;
        declarations.set(property, record.important);
    }
    let shorthand = PropertyId::from(name);
    let (property, _) = declarations.get(&shorthand)?;
    match property.as_ref() {
        Property::Grid(value) => value.to_cssom_string().ok(),
        Property::GridTemplate(value) => value.to_cssom_string().ok(),
        _ => property.value_to_css_string(PrinterOptions::default()).ok(),
    }
}

fn synthesize_special_shorthand(
    name: &str,
    records: &[&DeclarationRecord],
    safe: bool,
) -> Option<String> {
    match name {
        "animation-range" => synthesize_timeline_trigger_range(
            records,
            safe,
            "animation-range-start",
            "animation-range-end",
            "normal",
        ),
        "animation" | "-webkit-animation" => synthesize_animation(records, safe),
        "transition" | "-webkit-transition" => synthesize_transition(records, safe),
        "background" => synthesize_background(records, safe),
        "border-radius" | "-webkit-border-radius" => synthesize_border_radius(records, safe),
        "border-image" => synthesize_border_image(records, safe),
        "columns" => synthesize_columns(records, safe),
        "container" => synthesize_container(records, safe),
        "flex" => synthesize_flex(records, safe),
        "font" => synthesize_font(records, safe),
        "font-variant" => synthesize_font_variant(records, safe),
        "grid" => synthesize_grid(records, safe),
        "grid-area" => synthesize_grid_area(records, safe),
        "grid-column" | "grid-row" => synthesize_grid_line(name, records, safe),
        "grid-template" => synthesize_grid_template(records, safe),
        "mask" => synthesize_mask(records, safe),
        "list-style" => synthesize_list_style(records, safe),
        "offset" => synthesize_offset(records, safe),
        "outline" => synthesize_outline(records, safe),
        "position-try" => synthesize_position_try(records, safe),
        "column-rule-inset" | "row-rule-inset" | "rule-inset" => {
            synthesize_rule_inset(records, safe)
        }
        "column-rule-inset-cap"
        | "column-rule-inset-end"
        | "column-rule-inset-junction"
        | "column-rule-inset-start"
        | "row-rule-inset-cap"
        | "row-rule-inset-end"
        | "row-rule-inset-junction"
        | "row-rule-inset-start"
        | "rule-inset-cap"
        | "rule-inset-end"
        | "rule-inset-junction"
        | "rule-inset-start" => synthesize_rule_inset_component(name, records, safe),
        "scroll-timeline" => synthesize_scroll_timeline(records, safe),
        "text-emphasis" => synthesize_text_emphasis(records, safe),
        "text-decoration" => synthesize_text_decoration(records, safe),
        "-webkit-text-stroke" => synthesize_text_stroke(records, safe),
        "text-box" => synthesize_text_box(records, safe),
        "text-wrap" => synthesize_text_wrap(records, safe),
        "timeline-trigger" => synthesize_timeline_trigger(records, safe),
        "timeline-trigger-activation-range" => synthesize_timeline_trigger_range(
            records,
            safe,
            "timeline-trigger-activation-range-start",
            "timeline-trigger-activation-range-end",
            "normal",
        ),
        "timeline-trigger-active-range" => synthesize_timeline_trigger_range(
            records,
            safe,
            "timeline-trigger-active-range-start",
            "timeline-trigger-active-range-end",
            "auto",
        ),
        "white-space" => synthesize_white_space(records, safe),
        "view-timeline" => synthesize_view_timeline(records, safe),
        _ => synthesize_structural_shorthand(name, records, safe),
    }
}

fn has_authoritative_shorthand_synthesis(name: &str) -> bool {
    matches!(
        name,
        "animation"
            | "animation-range"
            | "-webkit-animation"
            | "transition"
            | "-webkit-transition"
            | "background"
            | "border-image"
            | "columns"
            | "contain-intrinsic-size"
            | "container"
            | "flex"
            | "font"
            | "font-variant"
            | "grid"
            | "grid-area"
            | "grid-column"
            | "grid-row"
            | "mask"
            | "list-style"
            | "offset"
            | "outline"
            | "position-try"
            | "column-rule-inset"
            | "row-rule-inset"
            | "rule-inset"
            | "scroll-timeline"
            | "text-emphasis"
            | "text-decoration"
            | "-webkit-text-stroke"
            | "text-box"
            | "text-wrap"
            | "timeline-trigger"
            | "timeline-trigger-activation-range"
            | "timeline-trigger-active-range"
            | "white-space"
            | "view-timeline"
    ) || is_border_like(name)
        || is_repeated_four_value(name)
        || is_repeated_two_value(name)
        || is_two_value(name)
        || matches!(
            name,
            "background-position"
                | "border-block-color"
                | "border-block-style"
                | "border-block-width"
                | "border-inline-color"
                | "border-inline-style"
                | "border-inline-width"
                | "overscroll-behavior"
        )
}

fn synthesize_timeline_trigger_range(
    records: &[&DeclarationRecord],
    safe: bool,
    start_name: &str,
    end_name: &str,
    omitted_end: &str,
) -> Option<String> {
    let starts = value_list(record_value(records, start_name, safe)?)?;
    let ends = value_list(record_value(records, end_name, safe)?)?;
    if starts.len() != ends.len() {
        return None;
    }
    starts
        .into_iter()
        .zip(ends)
        .map(|(start, end)| synthesize_timeline_range_item(start, end, omitted_end, safe))
        .collect::<Option<Vec<_>>>()
        .map(|values| values.join(", "))
}

fn synthesize_position_try(records: &[&DeclarationRecord], safe: bool) -> Option<String> {
    let order = record_value(records, "position-try-order", safe)?;
    let fallbacks = record_value(records, "position-try-fallbacks", safe)?;
    Some(match (order, fallbacks) {
        ("normal", "none") => "none".to_owned(),
        ("normal", fallbacks) => fallbacks.to_owned(),
        (order, "none") => format!("{order} none"),
        (order, fallbacks) => format!("{order} {fallbacks}"),
    })
}

fn synthesize_timeline_trigger(records: &[&DeclarationRecord], safe: bool) -> Option<String> {
    let names = value_list(record_value(records, "timeline-trigger-name", safe)?)?;
    let sources = value_list(record_value(records, "timeline-trigger-source", safe)?)?;
    let activation_starts = value_list(record_value(
        records,
        "timeline-trigger-activation-range-start",
        safe,
    )?)?;
    let activation_ends = value_list(record_value(
        records,
        "timeline-trigger-activation-range-end",
        safe,
    )?)?;
    let active_starts = value_list(record_value(
        records,
        "timeline-trigger-active-range-start",
        safe,
    )?)?;
    let active_ends = value_list(record_value(
        records,
        "timeline-trigger-active-range-end",
        safe,
    )?)?;
    let item_count = names.len();
    if item_count == 0
        || [
            sources.len(),
            activation_starts.len(),
            activation_ends.len(),
            active_starts.len(),
            active_ends.len(),
        ]
        .iter()
        .any(|count| *count != item_count)
    {
        return None;
    }

    (0..item_count)
        .map(|index| {
            if safe {
                return Some(format!(
                    "{} {} {} {} / {} {}",
                    names[index],
                    sources[index],
                    activation_starts[index],
                    activation_ends[index],
                    active_starts[index],
                    active_ends[index],
                ));
            }
            let mut components = Vec::with_capacity(3);
            if names[index] != "none" {
                components.push(names[index]);
            }
            if sources[index] != "auto" {
                components.push(sources[index]);
            }
            let activation_start = activation_starts[index];
            let activation_end = activation_ends[index];
            if activation_start == activation_end {
                if activation_start != "normal" {
                    components.push(activation_start);
                }
            } else {
                match (activation_start, activation_end) {
                    ("normal", end) => components.push(end),
                    (start, "normal") => components.push(start),
                    (start, end) => {
                        components.push(start);
                        components.push(end);
                    }
                }
            }
            if active_starts[index] == "auto" && active_ends[index] != "auto" {
                components.push(active_ends[index]);
            }
            let left = components.join(" ");
            if active_starts[index] != "auto" {
                let mut right = active_starts[index].to_owned();
                if active_ends[index] != "auto" {
                    right.push(' ');
                    right.push_str(active_ends[index]);
                }
                return Some(format!("{left} / {right}"));
            }
            Some(if left.is_empty() {
                "none".to_owned()
            } else {
                left
            })
        })
        .collect::<Option<Vec<_>>>()
        .map(|values| values.join(", "))
}

fn synthesize_timeline_range_item(
    start: &str,
    end: &str,
    omitted_end: &str,
    safe: bool,
) -> Option<String> {
    if !safe && omitted_end == "auto" && end == omitted_end {
        return Some(start.to_owned());
    }
    let implied_end = parse_timeline_range_pair_or_auto(start, omitted_end)?.1;
    if implied_end == end {
        return Some(start.to_owned());
    }
    Some(format!("{start} {end}"))
}

fn record_value<'a>(records: &'a [&DeclarationRecord], name: &str, safe: bool) -> Option<&'a str> {
    let record = records.iter().find(|record| record.name == name)?;
    Some(if safe {
        record.safe_value()
    } else {
        record.observable_value()
    })
}

fn value_list(value: &str) -> Option<Vec<&str>> {
    split_top_level_delimiter(value, b',')
}

fn synthesize_animation(records: &[&DeclarationRecord], safe: bool) -> Option<String> {
    for (longhand, expected) in [
        ("animation-timeline", "auto"),
        ("animation-range-start", "normal"),
        ("animation-range-end", "normal"),
    ] {
        if value_list(record_value(records, longhand, safe)?)?
            .iter()
            .any(|value| *value != expected)
        {
            return None;
        }
    }
    let fields = [
        "animation-duration",
        "animation-timing-function",
        "animation-delay",
        "animation-iteration-count",
        "animation-direction",
        "animation-fill-mode",
        "animation-play-state",
        "animation-name",
    ];
    let lists = fields
        .iter()
        .map(|field| value_list(record_value(records, field, safe)?))
        .collect::<Option<Vec<_>>>()?;
    let length = lists.first()?.len();
    if length == 0 || lists.iter().any(|list| list.len() != length) {
        return None;
    }
    Some(
        (0..length)
            .map(|index| {
                lists
                    .iter()
                    .filter_map(|list| list.get(index).copied())
                    .collect::<Vec<_>>()
                    .join(" ")
            })
            .collect::<Vec<_>>()
            .join(", "),
    )
}

fn repeat_list(values: Vec<&str>, length: usize) -> Option<Vec<&str>> {
    if values.len() == length {
        return Some(values);
    }
    if values.len() == 1 {
        return Some(vec![values[0]; length]);
    }
    None
}

fn synthesize_transition(records: &[&DeclarationRecord], safe: bool) -> Option<String> {
    let properties = [
        "transition-behavior",
        "transition-duration",
        "transition-timing-function",
        "transition-delay",
        "transition-property",
    ];
    let raw_lists = properties
        .iter()
        .map(|property| value_list(record_value(records, property, safe)?))
        .collect::<Option<Vec<_>>>()?;
    let length = raw_lists.iter().map(Vec::len).max()?;
    let lists = raw_lists
        .into_iter()
        .map(|list| repeat_list(list, length))
        .collect::<Option<Vec<_>>>()?;
    let mut transitions = Vec::with_capacity(length);
    for index in 0..length {
        let behavior = *lists[0].get(index)?;
        let duration = *lists[1].get(index)?;
        let timing = *lists[2].get(index)?;
        let delay = *lists[3].get(index)?;
        let property = *lists[4].get(index)?;
        let mut components = Vec::new();
        if property != "all" {
            components.push(property);
        }
        if !safe {
            if duration != "0s" {
                components.push(duration);
            }
            if timing != "ease" {
                components.push(timing);
            }
            if delay != "0s" {
                components.push(delay);
            }
            if behavior != "normal" {
                components.push(behavior);
            }
            if components.is_empty() {
                components.push("all");
            }
            transitions.push(components.join(" "));
            continue;
        }
        if duration == "0s"
            && timing == "ease"
            && behavior == "normal"
            && crate::property_constraints::has_direct_negative_component(delay)
        {
            components.push(delay);
            transitions.push(components.join(" "));
            continue;
        }
        if duration != "0s" || timing != "ease" || delay != "0s" || behavior != "normal" {
            components.push(duration);
        }
        if timing != "ease" || delay != "0s" || behavior != "normal" {
            components.push(timing);
        }
        if delay != "0s" || behavior != "normal" {
            components.push(delay);
        }
        if behavior != "normal" {
            components.push(behavior);
        }
        if components.is_empty() {
            components.push("all");
        }
        transitions.push(components.join(" "));
    }
    Some(transitions.join(", "))
}

fn parallel_lists<'a>(
    records: &'a [&DeclarationRecord],
    properties: &[&str],
    safe: bool,
) -> Option<Vec<Vec<&'a str>>> {
    let lists = properties
        .iter()
        .map(|property| value_list(record_value(records, property, safe)?))
        .collect::<Option<Vec<_>>>()?;
    let length = lists.first()?.len();
    (length > 0 && lists.iter().all(|list| list.len() == length)).then_some(lists)
}

fn synthesize_background(records: &[&DeclarationRecord], safe: bool) -> Option<String> {
    let properties = [
        "background-image",
        "background-position-x",
        "background-position-y",
        "background-size",
        "background-repeat",
        "background-attachment",
        "background-origin",
        "background-clip",
    ];
    let lists = parallel_lists(records, &properties, safe)?;
    let color = record_value(records, "background-color", safe)?;
    let mut layers = Vec::with_capacity(lists[0].len());
    for index in 0..lists[0].len() {
        let image = *lists[0].get(index)?;
        let x = *lists[1].get(index)?;
        let y = *lists[2].get(index)?;
        let size = *lists[3].get(index)?;
        let repeat = *lists[4].get(index)?;
        let attachment = *lists[5].get(index)?;
        let origin = *lists[6].get(index)?;
        let clip = *lists[7].get(index)?;
        let mut components = Vec::new();
        if image != "initial" {
            components.push(image.to_owned());
        }
        if x != "initial" || y != "initial" || size != "initial" {
            if x == "initial" || y == "initial" {
                return None;
            }
            components.push(format!("{x} {y}"));
            if size != "initial" {
                components.push(format!("/ {size}"));
            }
        }
        for value in [repeat, attachment] {
            if value != "initial" {
                components.push(value.to_owned());
            }
        }
        if origin == "initial" && clip == "text" {
            components.push(clip.to_owned());
        } else if origin != "initial" || clip != "initial" {
            if origin == "initial" || clip == "initial" {
                return None;
            }
            components.push(origin.to_owned());
            components.push(clip.to_owned());
        }
        if index + 1 == lists[0].len() && color != "initial" {
            components.push(color.to_owned());
        }
        if components.is_empty() {
            return None;
        }
        layers.push(components.join(" "));
    }
    Some(layers.join(", "))
}

fn synthesize_border_image(records: &[&DeclarationRecord], safe: bool) -> Option<String> {
    let raw_source = record_value(records, "border-image-source", safe)?;
    let source = if safe {
        canonicalize_reparsable_url(raw_source)
    } else {
        raw_source.to_owned()
    };
    let slice = record_value(records, "border-image-slice", safe)?;
    let width = record_value(records, "border-image-width", safe)?;
    let outset = record_value(records, "border-image-outset", safe)?;
    let repeat = record_value(records, "border-image-repeat", safe)?;
    if slice == "100%" && width == "1" && outset == "0" && repeat == "stretch" {
        return Some(source);
    }
    Some(format!("{source} {slice} / {width} / {outset} {repeat}"))
}

fn canonicalize_reparsable_url(value: &str) -> String {
    let Some(body) = value
        .strip_prefix("url(\"")
        .and_then(|body| body.strip_suffix("\")"))
    else {
        return value.to_owned();
    };
    if body.is_empty()
        || body.chars().any(|character| {
            character.is_whitespace() || matches!(character, '"' | '\'' | '(' | ')' | '\\')
        })
    {
        return value.to_owned();
    }
    format!("url({body})")
}

fn synthesize_view_timeline(records: &[&DeclarationRecord], safe: bool) -> Option<String> {
    let names = value_list(record_value(records, "view-timeline-name", safe)?)?;
    let axes = repeat_list(
        value_list(record_value(records, "view-timeline-axis", safe)?)?,
        names.len(),
    )?;
    let insets = repeat_list(
        value_list(record_value(records, "view-timeline-inset", safe)?)?,
        names.len(),
    )?;
    Some(
        names
            .iter()
            .enumerate()
            .map(|(index, name)| {
                let mut components = vec![*name];
                if axes[index] != "block" {
                    components.push(axes[index]);
                }
                if insets[index] != "auto" {
                    components.push(insets[index]);
                }
                components.join(" ")
            })
            .collect::<Vec<_>>()
            .join(", "),
    )
}

fn synthesize_columns(records: &[&DeclarationRecord], safe: bool) -> Option<String> {
    let width = record_value(records, "column-width", safe)?;
    let count = record_value(records, "column-count", safe)?;
    let height = record_value(records, "column-height", safe)?;
    record_value(records, "column-wrap", safe)?;
    let mut shorthand = match (width, count) {
        ("auto", "auto") => "auto".to_owned(),
        ("auto", value) | (value, "auto") => value.to_owned(),
        _ => format!("{width} {count}"),
    };
    if height != "auto" {
        shorthand.push_str(" / ");
        shorthand.push_str(height);
    }
    Some(shorthand)
}

fn synthesize_container(records: &[&DeclarationRecord], safe: bool) -> Option<String> {
    let name = record_value(records, "container-name", safe)?;
    let kind = record_value(records, "container-type", safe)?;
    Some(if kind == "normal" {
        name.to_owned()
    } else {
        format!("{name} / {kind}")
    })
}

fn synthesize_flex(records: &[&DeclarationRecord], safe: bool) -> Option<String> {
    let grow = record_value(records, "flex-grow", safe)?;
    let shrink = record_value(records, "flex-shrink", safe)?;
    let basis = record_value(records, "flex-basis", safe)?;
    Some(format!("{grow} {shrink} {basis}"))
}

fn synthesize_grid_line(name: &str, records: &[&DeclarationRecord], safe: bool) -> Option<String> {
    if records.len() != 2 {
        return None;
    }
    let start = if safe {
        records[0].safe_value()
    } else {
        records[0].observable_value()
    };
    let end = if safe {
        records[1].safe_value()
    } else {
        records[1].observable_value()
    };
    if !records.iter().any(|record| {
        parse_semantic_property_with_limits(
            &record.name,
            if safe {
                record.safe_value()
            } else {
                record.observable_value()
            },
            ResourceLimits::default(),
        )
        .is_ok_and(|value| value.recovered().contains_context_dependent_sign())
    }) {
        return synthesize_typed_shorthand(records, name, safe);
    }
    Some(if end == "auto" {
        start.to_owned()
    } else {
        format!("{start} / {end}")
    })
}

fn synthesize_grid_area(records: &[&DeclarationRecord], safe: bool) -> Option<String> {
    if records.len() != 4 {
        return None;
    }
    let values = records
        .iter()
        .map(|record| {
            if safe {
                record.safe_value()
            } else {
                record.observable_value()
            }
        })
        .collect::<Vec<_>>();
    if !records.iter().any(|record| {
        parse_semantic_property_with_limits(
            &record.name,
            if safe {
                record.safe_value()
            } else {
                record.observable_value()
            },
            ResourceLimits::default(),
        )
        .is_ok_and(|value| value.recovered().contains_context_dependent_sign())
    }) {
        return synthesize_typed_shorthand(records, "grid-area", safe);
    }
    let retained = values
        .iter()
        .rposition(|value| *value != "auto")
        .map_or(1, |index| index + 1);
    Some(values[..retained].join(" / "))
}

fn synthesize_grid(records: &[&DeclarationRecord], safe: bool) -> Option<String> {
    let authored_auto_flow = records.first()?.name == "grid-template-columns";
    if !authored_auto_flow {
        return synthesize_typed_shorthand(records, "grid", safe);
    }

    let rows = record_value(records, "grid-template-rows", safe)?;
    let columns = record_value(records, "grid-template-columns", safe)?;
    let areas = record_value(records, "grid-template-areas", safe)?;
    let flow = record_value(records, "grid-auto-flow", safe)?;
    let auto_columns = record_value(records, "grid-auto-columns", safe)?;
    let auto_rows = record_value(records, "grid-auto-rows", safe)?;
    if areas != "none" {
        return None;
    }

    if matches!(flow, "row" | "dense" | "row dense") && rows == "none" && auto_columns == "auto" {
        let mut left = "auto-flow".to_owned();
        if matches!(flow, "dense" | "row dense") {
            left.push_str(" dense");
        }
        if auto_rows != "auto" {
            left.push(' ');
            left.push_str(auto_rows);
        }
        return Some(format!("{left} / {columns}"));
    }

    if matches!(flow, "column" | "column dense") && columns == "none" && auto_rows == "auto" {
        let mut right = "auto-flow".to_owned();
        if flow == "column dense" {
            right.push_str(" dense");
        }
        if auto_columns != "auto" {
            right.push(' ');
            right.push_str(auto_columns);
        }
        return Some(format!("{rows} / {right}"));
    }

    None
}

fn synthesize_grid_template(records: &[&DeclarationRecord], safe: bool) -> Option<String> {
    let rows = record_value(records, "grid-template-rows", safe)?;
    let columns = record_value(records, "grid-template-columns", safe)?;
    let areas = record_value(records, "grid-template-areas", safe)?;
    if rows == "none" && columns == "none" {
        return Some("none".to_owned());
    }
    if areas == "none" {
        return Some(format!("{rows} / {columns}"));
    }
    if rows == "auto" && columns == "none" {
        return Some(areas.to_owned());
    }
    let area_rows = split_top_level_whitespace(areas)?;
    let row_sizes = split_top_level_whitespace(rows)?;
    if area_rows.len() != row_sizes.len() {
        return None;
    }
    Some(format!(
        "{} / {columns}",
        area_rows
            .iter()
            .zip(row_sizes)
            .map(|(area, size)| format!("{area} {size}"))
            .collect::<Vec<_>>()
            .join(" ")
    ))
}

fn synthesize_font(records: &[&DeclarationRecord], safe: bool) -> Option<String> {
    for (longhand, expected) in [
        ("font-variant-ligatures", "normal"),
        ("font-variant-numeric", "normal"),
        ("font-variant-east-asian", "normal"),
        ("font-variant-alternates", "normal"),
        ("font-size-adjust", "none"),
        ("font-language-override", "normal"),
        ("font-kerning", "auto"),
        ("font-optical-sizing", "auto"),
        ("font-feature-settings", "normal"),
        ("font-variation-settings", "normal"),
        ("font-variant-position", "normal"),
        ("font-variant-emoji", "normal"),
    ] {
        if record_value(records, longhand, safe)? != expected {
            return None;
        }
    }

    let mut components = Vec::new();
    for (longhand, initial) in [
        ("font-style", "normal"),
        ("font-variant-caps", "normal"),
        ("font-weight", "normal"),
        ("font-stretch", "normal"),
    ] {
        let value = record_value(records, longhand, safe)?;
        if value != initial {
            components.push(value.to_owned());
        }
    }
    let size = record_value(records, "font-size", safe)?;
    let line_height = record_value(records, "line-height", safe)?;
    components.push(if line_height == "normal" {
        size.to_owned()
    } else {
        format!("{size} / {line_height}")
    });
    components.push(record_value(records, "font-family", safe)?.to_owned());
    Some(components.join(" "))
}

fn synthesize_font_variant(records: &[&DeclarationRecord], safe: bool) -> Option<String> {
    let values = [
        record_value(records, "font-variant-ligatures", safe)?,
        record_value(records, "font-variant-caps", safe)?,
        record_value(records, "font-variant-alternates", safe)?,
        record_value(records, "font-variant-numeric", safe)?,
        record_value(records, "font-variant-east-asian", safe)?,
        record_value(records, "font-variant-position", safe)?,
        record_value(records, "font-variant-emoji", safe)?,
    ];
    if values.iter().all(|value| *value == "normal") {
        return Some("normal".to_owned());
    }
    if values[0] == "none" && values[1..].iter().all(|value| *value == "normal") {
        return Some("none".to_owned());
    }
    Some(
        values
            .into_iter()
            .filter(|value| *value != "normal")
            .collect::<Vec<_>>()
            .join(" "),
    )
}

fn synthesize_mask(records: &[&DeclarationRecord], safe: bool) -> Option<String> {
    let properties = [
        "mask-image",
        "-webkit-mask-position-x",
        "-webkit-mask-position-y",
        "mask-size",
        "mask-repeat",
        "mask-origin",
        "mask-clip",
        "mask-composite",
        "mask-mode",
    ];
    let lists = parallel_lists(records, &properties, safe)?;
    let mut layers = Vec::with_capacity(lists[0].len());
    for index in 0..lists[0].len() {
        // CSSOM exposes an omitted mask image as `initial` on the expanded
        // longhand. The shorthand grammar still has to treat that observable
        // spelling as its semantic initial value (`none`) when deciding which
        // components to omit from the synthesized shorthand.
        let image = match *lists[0].get(index)? {
            "initial" => "none",
            image => image,
        };
        let x = *lists[1].get(index)?;
        let y = *lists[2].get(index)?;
        let size = *lists[3].get(index)?;
        let repeat = *lists[4].get(index)?;
        let origin = *lists[5].get(index)?;
        let clip = *lists[6].get(index)?;
        let composite = *lists[7].get(index)?;
        let mode = *lists[8].get(index)?;
        let mut components = Vec::new();
        if image != "none" {
            components.push(image.to_owned());
        }
        if x != "0%" || y != "0%" || size != "auto" {
            components.push(format!("{x} {y}"));
            if size != "auto" {
                components.push(format!("/ {size}"));
            }
        }
        if repeat != "repeat" {
            components.push(repeat.to_owned());
        }
        if clip == "no-clip" {
            if origin != "border-box" {
                components.push(origin.to_owned());
            }
            components.push(clip.to_owned());
        } else if origin != "border-box" || clip != "border-box" {
            components.push(origin.to_owned());
            if clip != origin {
                components.push(clip.to_owned());
            }
        }
        if composite != "add" {
            components.push(composite.to_owned());
        }
        if mode != "match-source" {
            components.push(mode.to_owned());
        }
        layers.push(if components.is_empty() {
            "none".to_owned()
        } else {
            components.join(" ")
        });
    }
    Some(layers.join(", "))
}

fn synthesize_offset(records: &[&DeclarationRecord], safe: bool) -> Option<String> {
    let position = record_value(records, "offset-position", safe)?;
    let path = record_value(records, "offset-path", safe)?;
    let distance = record_value(records, "offset-distance", safe)?;
    let rotate = record_value(records, "offset-rotate", safe)?;
    let anchor = record_value(records, "offset-anchor", safe)?;
    let rotate_is_default = offset_rotate_is_shorthand_default(rotate);
    if position == "normal"
        && path == "none"
        && distance == "0px"
        && rotate_is_default
        && anchor == "auto"
    {
        return Some("normal".to_owned());
    }
    let mut components = Vec::new();
    if position != "normal" {
        components.push(position.to_owned());
    }
    let motion_is_non_default = distance != "0px" || !rotate_is_default || anchor != "auto";
    if path != "none" || motion_is_non_default {
        components.push(path.to_owned());
    }
    if distance != "0px" {
        components.push(distance.to_owned());
    }
    if !rotate_is_default {
        components.push(rotate.to_owned());
    }
    if anchor != "auto" {
        components.push(format!("/ {anchor}"));
    }
    (!components.is_empty()).then(|| components.join(" "))
}

fn synthesize_outline(records: &[&DeclarationRecord], safe: bool) -> Option<String> {
    let color = record_value(records, "outline-color", safe)?;
    let style = record_value(records, "outline-style", safe)?;
    let width = record_value(records, "outline-width", safe)?;
    if !safe {
        let components = [color, style, width]
            .into_iter()
            .filter(|value| *value != "initial")
            .collect::<Vec<_>>();
        return Some(if components.is_empty() {
            "initial".to_owned()
        } else {
            components.join(" ")
        });
    }
    Some(format!("{color} {style} {width}"))
}

fn synthesize_border_radius(records: &[&DeclarationRecord], safe: bool) -> Option<String> {
    let mut horizontal = Vec::with_capacity(4);
    let mut vertical = Vec::with_capacity(4);
    for name in [
        "border-top-left-radius",
        "border-top-right-radius",
        "border-bottom-right-radius",
        "border-bottom-left-radius",
    ] {
        let value = record_value(records, name, safe)?;
        let components = split_top_level_whitespace(value)?;
        let [x, y] = match components.as_slice() {
            [value] => [*value, *value],
            [x, y] => [*x, *y],
            _ => return None,
        };
        horizontal.push(x.to_owned());
        vertical.push(y.to_owned());
    }
    let horizontal = compress_four_values(horizontal)?;
    let vertical = compress_four_values(vertical)?;
    Some(if horizontal == vertical {
        horizontal
    } else {
        format!("{horizontal} / {vertical}")
    })
}

fn synthesize_list_style(records: &[&DeclarationRecord], safe: bool) -> Option<String> {
    let position = record_value(records, "list-style-position", safe)?;
    let image = record_value(records, "list-style-image", safe)?;
    let style_type = record_value(records, "list-style-type", safe)?;
    let defaults = if safe {
        ["outside", "none", "disc"]
    } else {
        ["initial", "initial", "initial"]
    };
    let components = [position, image, style_type]
        .into_iter()
        .zip(defaults)
        .filter_map(|(value, default)| (value != default).then_some(value))
        .collect::<Vec<_>>();
    Some(if components.is_empty() {
        defaults[0].to_owned()
    } else {
        components.join(" ")
    })
}

fn synthesize_text_emphasis(records: &[&DeclarationRecord], safe: bool) -> Option<String> {
    let style = record_value(records, "text-emphasis-style", safe)?;
    let color = record_value(records, "text-emphasis-color", safe)?;
    let style_default = if safe { "none" } else { "initial" };
    let color_default = if safe { "currentcolor" } else { "initial" };
    match (style == style_default, color == color_default) {
        (true, true) => Some(style_default.to_owned()),
        (true, false) => Some(color.to_owned()),
        (false, true) => Some(style.to_owned()),
        (false, false) => Some(format!("{style} {color}")),
    }
}

fn synthesize_rule_inset(records: &[&DeclarationRecord], safe: bool) -> Option<String> {
    let values = record_values(records, safe)?;
    let values = match values.as_slice() {
        [cap_start, cap_end, junction_start, junction_end] => {
            [cap_start, cap_end, junction_start, junction_end]
        }
        [column_cap_start, column_cap_end, column_junction_start, column_junction_end, row_cap_start, row_cap_end, row_junction_start, row_junction_end]
            if column_cap_start == row_cap_start
                && column_cap_end == row_cap_end
                && column_junction_start == row_junction_start
                && column_junction_end == row_junction_end =>
        {
            [
                column_cap_start,
                column_cap_end,
                column_junction_start,
                column_junction_end,
            ]
        }
        _ => return None,
    };
    if values.iter().all(|value| *value == values[0]) {
        return Some(values[0].clone());
    }
    let cap = format!("{} {}", values[0], values[1]);
    let junction = format!("{} {}", values[2], values[3]);
    Some(format!("{cap} / {junction}"))
}

fn synthesize_rule_inset_component(
    name: &str,
    records: &[&DeclarationRecord],
    safe: bool,
) -> Option<String> {
    let values = record_values(records, safe)?;
    if name.ends_with("-start") || name.ends_with("-end") {
        return values
            .iter()
            .all(|value| *value == values[0])
            .then(|| values[0].clone());
    }
    let pair = match values.as_slice() {
        [start, end] => [start, end],
        [first_start, first_end, second_start, second_end]
            if first_start == second_start && first_end == second_end =>
        {
            [first_start, first_end]
        }
        _ => return None,
    };
    Some(if pair[0] == pair[1] {
        pair[0].clone()
    } else {
        format!("{} {}", pair[0], pair[1])
    })
}

fn synthesize_text_wrap(records: &[&DeclarationRecord], safe: bool) -> Option<String> {
    let mode = record_value(records, "text-wrap-mode", safe)?;
    let style = record_value(records, "text-wrap-style", safe)?;
    Some(if style == "initial" {
        mode.to_owned()
    } else if style == "auto" {
        if mode == "initial" {
            "wrap".to_owned()
        } else {
            mode.to_owned()
        }
    } else if matches!(mode, "wrap" | "initial") {
        style.to_owned()
    } else {
        format!("{mode} {style}")
    })
}

fn synthesize_scroll_timeline(records: &[&DeclarationRecord], safe: bool) -> Option<String> {
    let names = value_list(record_value(records, "scroll-timeline-name", safe)?)?;
    let axes = repeat_list(
        value_list(record_value(records, "scroll-timeline-axis", safe)?)?,
        names.len(),
    )?;
    (!names.is_empty()).then(|| {
        names
            .iter()
            .enumerate()
            .map(|(index, name)| {
                if axes[index] == "block" {
                    (*name).to_owned()
                } else {
                    format!("{name} {}", axes[index])
                }
            })
            .collect::<Vec<_>>()
            .join(", ")
    })
}

fn synthesize_text_box(records: &[&DeclarationRecord], safe: bool) -> Option<String> {
    let trim = record_value(records, "text-box-trim", safe)?;
    let edge = record_value(records, "text-box-edge", safe)?;
    if trim == "none" && edge == "auto" {
        return Some("normal".to_owned());
    }
    if edge == "auto" {
        return Some(trim.to_owned());
    }
    Some(if trim == "trim-both" {
        edge.to_owned()
    } else {
        format!("{trim} {edge}")
    })
}

fn synthesize_white_space(records: &[&DeclarationRecord], safe: bool) -> Option<String> {
    let collapse = match record_value(records, "white-space-collapse", safe)? {
        "initial" => "collapse",
        value => value,
    };
    let mode = match record_value(records, "text-wrap-mode", safe)? {
        "initial" => "wrap",
        value => value,
    };
    Some(match (collapse, mode) {
        ("collapse", "wrap") => "normal".to_owned(),
        ("preserve", "nowrap") => "pre".to_owned(),
        ("preserve", "wrap") => "pre-wrap".to_owned(),
        ("preserve-breaks", "wrap") => "pre-line".to_owned(),
        ("collapse", "nowrap") => "nowrap".to_owned(),
        ("break-spaces", "wrap") => "break-spaces".to_owned(),
        _ => format!("{collapse} {mode}"),
    })
}

fn synthesize_structural_shorthand(
    name: &str,
    records: &[&DeclarationRecord],
    safe: bool,
) -> Option<String> {
    if name == "background-position" {
        return synthesize_background_position(records, safe);
    }
    if name == "place-self" {
        let align = record_value(records, "align-self", safe)?;
        let justify = record_value(records, "justify-self", safe)?;
        return Some(if align == justify {
            align.to_owned()
        } else {
            format!("{align} {justify}")
        });
    }
    if name == "contain-intrinsic-size" && records.len() == 2 {
        let values = record_values(records, safe)?;
        return Some(if values[0] == values[1] {
            values[0].clone()
        } else {
            values.join(" ")
        });
    }
    if matches!(name, "rule-width" | "rule-style" | "rule-color") && records.len() == 2 {
        let values = record_values(records, safe)?;
        return (values[0] == values[1]).then(|| values[0].clone());
    }
    if is_border_like(name) {
        return synthesize_border_like(name, records, safe);
    }
    if is_repeated_four_value(name) && records.len() == 4 {
        return compress_four_values(record_values(records, safe)?);
    }
    if (is_repeated_two_value(name) || is_two_value(name)) && records.len() == 2 {
        let values = record_values(records, safe)?;
        return Some(if values[0] == values[1] {
            values[0].clone()
        } else {
            values.join(" ")
        });
    }
    synthesize_repeated_pair(name, records, safe)
}

fn synthesize_background_position(records: &[&DeclarationRecord], safe: bool) -> Option<String> {
    let x = value_list(record_value(records, "background-position-x", safe)?)?;
    let y = value_list(record_value(records, "background-position-y", safe)?)?;
    if x.len() != y.len() || x.is_empty() {
        return None;
    }
    if x.len() == 1 && x[0] == "initial" && y[0] == "initial" {
        return Some("initial".to_owned());
    }
    x.into_iter()
        .zip(y)
        .map(|(x, y)| {
            (!matches!(x, "initial") && !matches!(y, "initial")).then(|| format!("{x} {y}"))
        })
        .collect::<Option<Vec<_>>>()
        .map(|layers| layers.join(", "))
}

fn record_values(records: &[&DeclarationRecord], safe: bool) -> Option<Vec<String>> {
    Some(
        records
            .iter()
            .map(|record| {
                if safe {
                    record.safe_value().to_owned()
                } else {
                    record.observable_value().to_owned()
                }
            })
            .collect(),
    )
}

fn compress_four_values(values: Vec<String>) -> Option<String> {
    let [top, right, bottom, left] = values.as_slice() else {
        return None;
    };
    Some(if top == right && top == bottom && top == left {
        top.clone()
    } else if top == bottom && right == left {
        format!("{top} {right}")
    } else if right == left {
        format!("{top} {right} {bottom}")
    } else {
        values.join(" ")
    })
}

fn synthesize_border_like(
    name: &str,
    records: &[&DeclarationRecord],
    safe: bool,
) -> Option<String> {
    let collect_component = |component: BorderComponent| {
        records
            .iter()
            .filter(|record| border_component(&record.name) == Some(component))
            .map(|record| {
                if safe {
                    record.safe_value()
                } else {
                    record.observable_value()
                }
            })
            .collect::<Vec<_>>()
    };
    let widths = collect_component(BorderComponent::Width);
    let styles = collect_component(BorderComponent::Style);
    let colors = collect_component(BorderComponent::Color);
    let width = uniform_value(&widths)?;
    let style = uniform_value(&styles)?;
    let color = uniform_value(&colors)?;
    if matches!(name, "column-rule" | "row-rule" | "rule") {
        return synthesize_gap_rule(width, style, color).ok();
    }
    if !safe {
        let logical_side = matches!(
            name,
            "border-block-end" | "border-block-start" | "border-inline-end" | "border-inline-start"
        );
        let mut components = Vec::new();
        if !width.eq_ignore_ascii_case("initial")
            && (logical_side || !width.eq_ignore_ascii_case("medium"))
        {
            components.push(width);
        }
        if !style.eq_ignore_ascii_case("initial")
            && (logical_side || !style.eq_ignore_ascii_case("none"))
        {
            components.push(style);
        }
        if !color.eq_ignore_ascii_case("initial")
            && (logical_side || !color.eq_ignore_ascii_case("currentcolor"))
        {
            components.push(color);
        }
        return (!components.is_empty()).then(|| components.join(" "));
    }
    if width.eq_ignore_ascii_case("medium")
        && style.eq_ignore_ascii_case("none")
        && color.eq_ignore_ascii_case("currentcolor")
    {
        if matches!(
            name,
            "border-block-end" | "border-block-start" | "border-inline-end" | "border-inline-start"
        ) {
            return Some("medium none currentcolor".to_owned());
        }
        if is_border_like(name) {
            return safe.then(|| "none".to_owned());
        }
        return Some("medium".to_owned());
    }
    let mut components = Vec::new();
    if !width.eq_ignore_ascii_case("medium") {
        components.push(width);
    }
    components.push(style);
    if !color.eq_ignore_ascii_case("currentcolor") {
        components.push(color);
    }
    Some(components.join(" "))
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum BorderComponent {
    Width,
    Style,
    Color,
}

fn border_component(name: &str) -> Option<BorderComponent> {
    match name {
        "border-top-width"
        | "border-right-width"
        | "border-bottom-width"
        | "border-left-width"
        | "border-block-start-width"
        | "border-block-end-width"
        | "border-inline-start-width"
        | "border-inline-end-width"
        | "column-rule-width"
        | "row-rule-width" => Some(BorderComponent::Width),
        "border-top-style"
        | "border-right-style"
        | "border-bottom-style"
        | "border-left-style"
        | "border-block-start-style"
        | "border-block-end-style"
        | "border-inline-start-style"
        | "border-inline-end-style"
        | "column-rule-style"
        | "row-rule-style" => Some(BorderComponent::Style),
        "border-top-color"
        | "border-right-color"
        | "border-bottom-color"
        | "border-left-color"
        | "border-block-start-color"
        | "border-block-end-color"
        | "border-inline-start-color"
        | "border-inline-end-color"
        | "column-rule-color"
        | "row-rule-color" => Some(BorderComponent::Color),
        _ => None,
    }
}

fn synthesize_text_decoration(records: &[&DeclarationRecord], safe: bool) -> Option<String> {
    let line = record_value(records, "text-decoration-line", safe)?;
    let thickness = record_value(records, "text-decoration-thickness", safe)?;
    let style = record_value(records, "text-decoration-style", safe)?;
    let color = record_value(records, "text-decoration-color", safe)?;
    let mut components = Vec::new();
    let has_non_default_component = thickness != "initial" && thickness != "auto"
        || style != "initial" && style != "solid"
        || color != "initial" && color != "currentcolor";
    if line != "initial" && (line != "none" || !has_non_default_component) {
        components.push(line);
    }
    if thickness != "initial" && thickness != "auto" {
        components.push(thickness);
    }
    if style != "initial" && style != "solid" {
        components.push(style);
    }
    if color != "initial" && color != "currentcolor" {
        components.push(color);
    }
    Some(if components.is_empty() {
        "none".to_owned()
    } else {
        components.join(" ")
    })
}

fn synthesize_text_stroke(records: &[&DeclarationRecord], safe: bool) -> Option<String> {
    let width = record_value(records, "-webkit-text-stroke-width", safe)?;
    let color = record_value(records, "-webkit-text-stroke-color", safe)?;
    let mut components = Vec::with_capacity(2);
    if width != "initial" {
        components.push(width);
    }
    if color != "initial" {
        components.push(color);
    }
    Some(if components.is_empty() {
        "initial".to_owned()
    } else {
        components.join(" ")
    })
}

fn uniform_value<'a>(values: &[&'a str]) -> Option<&'a str> {
    let first = values.first().copied()?;
    values.iter().all(|value| *value == first).then_some(first)
}

fn is_border_like(name: &str) -> bool {
    matches!(
        name,
        "border"
            | "border-block"
            | "border-block-end"
            | "border-block-start"
            | "border-bottom"
            | "border-inline"
            | "border-inline-end"
            | "border-inline-start"
            | "border-left"
            | "border-right"
            | "border-top"
            | "column-rule"
            | "row-rule"
            | "rule"
    )
}

fn is_two_value(name: &str) -> bool {
    matches!(
        name,
        "gap"
            | "grid-gap"
            | "inset-block"
            | "inset-inline"
            | "margin-block"
            | "margin-inline"
            | "overscroll-behavior"
            | "padding-block"
            | "padding-inline"
            | "scroll-margin-block"
            | "scroll-margin-inline"
            | "scroll-padding-block"
            | "scroll-padding-inline"
    )
}

fn is_repeated_two_value(name: &str) -> bool {
    matches!(
        name,
        "border-block-color"
            | "border-block-style"
            | "border-block-width"
            | "border-inline-color"
            | "border-inline-style"
            | "border-inline-width"
            | "corner-block-end-shape"
            | "corner-block-start-shape"
            | "corner-bottom-shape"
            | "corner-inline-end-shape"
            | "corner-inline-start-shape"
            | "corner-left-shape"
            | "corner-right-shape"
            | "corner-top-shape"
    )
}

fn is_repeated_four_value(name: &str) -> bool {
    matches!(
        name,
        "border-color" | "border-style" | "border-width" | "corner-shape"
    )
}

fn is_css_wide_keyword(value: &str) -> bool {
    matches!(
        value,
        "initial" | "inherit" | "unset" | "revert" | "revert-layer" | "revert-rule"
    )
}

fn synthesize_repeated_pair(
    name: &str,
    records: &[&DeclarationRecord],
    safe: bool,
) -> Option<String> {
    let pair_names = [
        "background-position",
        "border-block-color",
        "border-block-style",
        "border-block-width",
        "border-inline-color",
        "border-inline-style",
        "border-inline-width",
        "overscroll-behavior",
    ];
    if !pair_names.contains(&name) || records.len() != 2 {
        return None;
    }
    let first = if safe {
        records[0].safe_value()
    } else {
        records[0].observable_value()
    };
    let second = if safe {
        records[1].safe_value()
    } else {
        records[1].observable_value()
    };
    Some(if first == second {
        first.to_owned()
    } else {
        format!("{first} {second}")
    })
}

pub(crate) fn parse_value(
    name: &str,
    value: &str,
    important: bool,
) -> Result<ParsedValue, MutationOutcome> {
    parse_value_with_limits(name, value, important, ResourceLimits::default())
}

pub(crate) fn parse_value_with_limits(
    name: &str,
    value: &str,
    important: bool,
    limits: ResourceLimits,
) -> Result<ParsedValue, MutationOutcome> {
    parse_value_for_source_with_limits(name, name, value, important, limits)
}

pub(crate) fn parse_value_for_source_with_limits(
    name: &str,
    source_name: &str,
    value: &str,
    important: bool,
    limits: ResourceLimits,
) -> Result<ParsedValue, MutationOutcome> {
    let semantic_name = if source_name == "-webkit-perspective" {
        source_name
    } else {
        name
    };
    let substitutions = analyze_substitutions(value);
    if !substitutions.valid {
        return Err(MutationOutcome::InvalidValue);
    }
    if name.starts_with("--") {
        let semantic = parse_semantic_property_with_limits(semantic_name, value, limits)
            .map_err(map_engine_error)?;
        return Ok(ParsedValue {
            value: DeclarationValue::semantic(semantic).map_err(map_engine_error)?,
            longhands: None,
        });
    }

    if let Some(keyword) = css_wide_keyword(value) {
        let longhands = shorthand_longhands(name).map(|longhands| {
            longhands
                .iter()
                .map(|longhand| DeclarationRecord {
                    name: (*longhand).to_owned(),
                    value: DeclarationValue::css_wide(keyword.clone()),
                    important,
                    pending_group: None,
                    alias_value: None,
                })
                .collect()
        });
        return Ok(ParsedValue {
            value: DeclarationValue::css_wide(keyword),
            longhands,
        });
    }

    if substitutions.found {
        let semantic = parse_semantic_property_with_limits(semantic_name, value, limits)
            .map_err(map_engine_error)?;
        return Ok(ParsedValue {
            value: DeclarationValue::semantic(semantic).map_err(map_engine_error)?,
            longhands: None,
        });
    }

    crate::property_constraints::validate_authored_property_capability(source_name, value)
        .map_err(map_engine_error)?;

    let legacy_break_value = if value.eq_ignore_ascii_case("always") {
        match source_name {
            "page-break-after" | "page-break-before" => Some("page"),
            "-webkit-column-break-after" | "-webkit-column-break-before" => Some("column"),
            _ => None,
        }
    } else {
        None
    };
    let value = legacy_break_value.unwrap_or(value);

    let legacy_webkit_radius;
    let value = if source_name == "-webkit-border-radius"
        && !value.contains('/')
        && split_top_level_whitespace(value).is_some_and(|components| components.len() == 2)
    {
        let components = split_top_level_whitespace(value).unwrap_or_default();
        legacy_webkit_radius = format!("{} / {}", components[0], components[1]);
        legacy_webkit_radius.as_str()
    } else {
        value
    };

    if !validate_typed_shorthand_structure(name, value) {
        return Err(MutationOutcome::InvalidValue);
    }

    if let Some(longhands) = expand_special_shorthand(name, value, important, limits) {
        let value = validated_expanded_shorthand_value(name, value, limits)?;
        return Ok(ParsedValue {
            value,
            longhands: Some(longhands),
        });
    }
    if let Some(longhands) = expand_structural_shorthand(name, value, important, limits) {
        let record_refs = longhands.iter().collect::<Vec<_>>();
        let canonical = synthesize_structural_shorthand(name, &record_refs, false)
            .unwrap_or_else(|| value.to_owned());
        let value = validated_expanded_shorthand_value(name, &canonical, limits)?;
        return Ok(ParsedValue {
            value,
            longhands: Some(longhands),
        });
    }

    let semantic = parse_semantic_property_with_limits(semantic_name, value, limits);
    if !semantic.as_ref().is_ok_and(|candidate| {
        matches!(
            candidate.parse_kind(),
            PropertyParseKind::Typed | PropertyParseKind::SheetomTyped
        )
    }) {
        if name == "border-image" {
            if let Some(longhands) = expand_border_image(value)
                .and_then(|values| records_from_values(name, value, values, important, limits))
            {
                return Ok(ParsedValue {
                    value: validated_expanded_shorthand_value(name, value, limits)?,
                    longhands: Some(longhands),
                });
            }
        }
        return Err(MutationOutcome::InvalidValue);
    }
    let semantic = semantic.map_err(map_engine_error)?;
    let Some(longhand_names) = observed_shorthand_longhands(name) else {
        let projection = project_declaration(&semantic).map_err(map_engine_error)?;
        let (canonical, observable) = if source_name == "-webkit-background-size" {
            (
                projection.canonical,
                duplicate_single_component(&projection.observable),
            )
        } else {
            (projection.canonical, projection.observable)
        };
        let value = DeclarationValue::semantic_with_canonical(semantic, canonical, observable);
        return Ok(ParsedValue {
            value,
            longhands: None,
        });
    };

    let property = parse_typed_property(name, value).map_err(|_| MutationOutcome::InvalidValue)?;
    let mut longhands = Vec::with_capacity(longhand_names.len());
    for longhand_name in longhand_names {
        let declaration_value =
            if let Some(longhand) = shorthand_longhand(&property, name, longhand_name) {
                let mut safe_value = longhand
                    .value_to_css_string(PrinterOptions::default())
                    .map_err(|_| MutationOutcome::InvalidValue)?;
                let source = authored_longhand_source(longhand_name, &safe_value, value);
                let owned_longhand = longhand.into_owned();
                let semantic = crate::SemanticDeclaration::from_standard_property(
                    longhand_name,
                    owned_longhand,
                    source,
                );
                let mut observable_value = project_declaration(&semantic)
                    .map_err(map_engine_error)?
                    .observable;
                if let Some((observable, safe)) =
                    observable_shorthand_override(name, longhand_name, value, &safe_value)
                {
                    observable_value = observable;
                    if let Some(safe) = safe {
                        safe_value = safe;
                    }
                }
                DeclarationValue::semantic_with_canonical(semantic, safe_value, observable_value)
            } else if let Some(initial_value) = initial_longhand_value(longhand_name) {
                let semantic =
                    parse_semantic_property_with_limits(longhand_name, initial_value, limits)
                        .map_err(map_engine_error)?;
                let projection = project_declaration(&semantic).map_err(map_engine_error)?;
                let mut observable_value = projection.observable;
                let mut safe_value = initial_value.to_owned();
                if let Some((observable, safe)) =
                    observable_shorthand_override(name, longhand_name, value, &safe_value)
                {
                    observable_value = observable;
                    if let Some(safe) = safe {
                        safe_value = safe;
                    }
                }
                DeclarationValue::semantic_with_canonical(semantic, safe_value, observable_value)
            } else {
                return Err(MutationOutcome::UnsupportedShorthand);
            };
        longhands.push(DeclarationRecord {
            name: (*longhand_name).to_owned(),
            value: declaration_value,
            important,
            pending_group: None,
            alias_value: None,
        });
    }
    if name == "grid" && grid_uses_auto_flow(value) {
        const GRID_AUTO_FLOW_ORDER: &[&str] = &[
            "grid-template-columns",
            "grid-template-rows",
            "grid-template-areas",
            "grid-auto-flow",
            "grid-auto-columns",
            "grid-auto-rows",
        ];
        longhands.sort_by_key(|record| {
            GRID_AUTO_FLOW_ORDER
                .iter()
                .position(|name| *name == record.name)
                .unwrap_or(GRID_AUTO_FLOW_ORDER.len())
        });
    }

    Ok(ParsedValue {
        value: DeclarationValue::semantic(semantic).map_err(map_engine_error)?,
        longhands: Some(longhands),
    })
}

fn duplicate_single_component(value: &str) -> String {
    if matches!(value, "contain" | "cover") {
        return value.to_owned();
    }
    if split_top_level_whitespace(value).is_some_and(|components| components.len() == 1) {
        return format!("{value} {value}");
    }
    value.to_owned()
}

fn css_wide_keyword(value: &str) -> Option<String> {
    let keyword = value.trim().to_ascii_lowercase();
    matches!(
        keyword.as_str(),
        "initial" | "inherit" | "unset" | "revert" | "revert-layer" | "revert-rule"
    )
    .then_some(keyword)
}

fn authored_longhand_source(name: &str, safe_value: &str, shorthand_input: &str) -> String {
    if semantic_value_matches(name, shorthand_input, safe_value) {
        return shorthand_input.to_owned();
    }

    let safe_items = split_top_level_delimiter(safe_value, b',').unwrap_or_default();
    let shorthand_items = split_top_level_delimiter(shorthand_input, b',').unwrap_or_default();
    if safe_items.len() == shorthand_items.len() && !safe_items.is_empty() {
        let authored = safe_items
            .iter()
            .zip(shorthand_items)
            .map(|(safe_item, shorthand_item)| {
                authored_item_source(name, safe_item.trim(), shorthand_item)
            })
            .collect::<Option<Vec<_>>>();
        if let Some(authored) = authored {
            return authored.join(", ");
        }
    }

    authored_item_source(name, safe_value, shorthand_input).unwrap_or_else(|| safe_value.to_owned())
}

fn authored_item_source(name: &str, safe_value: &str, shorthand_input: &str) -> Option<String> {
    if semantic_value_matches(name, shorthand_input, safe_value) {
        return Some(shorthand_input.trim().to_owned());
    }
    split_top_level_whitespace(shorthand_input)?
        .into_iter()
        .find(|candidate| semantic_value_matches(name, candidate, safe_value))
        .map(str::to_owned)
}

fn semantic_value_matches(name: &str, source: &str, canonical: &str) -> bool {
    parse_semantic_property_with_limits(name, source.trim(), ResourceLimits::default())
        .ok()
        .and_then(|declaration| declaration.canonical_value().ok())
        .is_some_and(|candidate| {
            candidate == canonical
                || candidate
                    .strip_prefix("calc(")
                    .and_then(|value| value.strip_suffix(')'))
                    .is_some_and(|value| value == canonical)
                || canonical == "0" && is_explicit_zero_dimension(source)
        })
}

fn is_explicit_zero_dimension(source: &str) -> bool {
    let mut input = ParserInput::new(source.trim());
    let mut parser = Parser::new(&mut input);
    let value = match parser.next() {
        Ok(Token::Dimension { value, .. }) => *value,
        _ => return false,
    };
    value == 0.0 && parser.is_exhausted()
}

fn observable_shorthand_override(
    shorthand: &str,
    longhand: &str,
    input: &str,
    safe_value: &str,
) -> Option<(String, Option<String>)> {
    if shorthand == "grid" && longhand == "grid-auto-flow" && safe_value == "row dense" {
        return Some(("dense".to_owned(), None));
    }
    if matches!(shorthand, "grid-area" | "grid-column" | "grid-row")
        && matches!(
            longhand,
            "grid-row-start" | "grid-column-start" | "grid-row-end" | "grid-column-end"
        )
    {
        return project_observable_value(longhand, safe_value).map(|value| (value, None));
    }
    if shorthand == "background" {
        return observable_background_longhand(longhand, input).map(|value| (value, None));
    }
    if matches!(
        shorthand,
        "border-block-end" | "border-block-start" | "border-inline-end" | "border-inline-start"
    ) {
        let expansion = expand_border_side_observable(input).ok()?;
        let observable = match border_component(longhand)? {
            BorderComponent::Width => expansion.width,
            BorderComponent::Style => expansion.style,
            BorderComponent::Color => expansion.color,
        };
        return Some((observable, None));
    }
    if matches!(shorthand, "mask" | "-webkit-mask") {
        if longhand == "mask-image" {
            return observable_mask_images(input).map(|value| (value, None));
        }
        if matches!(
            longhand,
            "-webkit-mask-position-x" | "-webkit-mask-position-y"
        ) {
            return observable_mask_positions(longhand, input).map(|value| (value, None));
        }
    }
    if shorthand == "text-emphasis" {
        return observable_text_emphasis_longhand(longhand, input).map(|value| (value, None));
    }
    if shorthand == "outline" {
        return observable_outline_longhand(longhand, input).map(|value| (value, None));
    }
    if shorthand == "list-style" {
        return observable_list_style_longhand(longhand, input).map(|value| (value, None));
    }
    None
}

fn observable_list_style_longhand(longhand: &str, input: &str) -> Option<String> {
    let components = split_top_level_whitespace(input)?;
    let position = components
        .iter()
        .find(|component| typed_longhand_value("list-style-position", component).is_some())
        .copied();
    let image = components
        .iter()
        .find(|component| {
            **component != "none" && typed_longhand_value("list-style-image", component).is_some()
        })
        .copied();
    let style_type = components
        .iter()
        .find(|component| {
            **component != "none"
                && Some(**component) != position
                && Some(**component) != image
                && typed_longhand_value("list-style-type", component).is_some()
        })
        .copied();
    let none_count = components
        .iter()
        .filter(|component| **component == "none")
        .count();
    let source = match longhand {
        "list-style-position" => position,
        "list-style-image" => image.or_else(|| {
            (none_count >= 2 || none_count == 1 && style_type.is_some()).then_some("none")
        }),
        "list-style-type" => style_type.or_else(|| (none_count >= 1).then_some("none")),
        _ => return None,
    };
    source.map_or_else(
        || Some("initial".to_owned()),
        |source| project_observable_value(longhand, source).or_else(|| Some(source.to_owned())),
    )
}

fn observable_outline_longhand(longhand: &str, input: &str) -> Option<String> {
    split_top_level_whitespace(input)?
        .into_iter()
        .find(|component| typed_longhand_value(longhand, component).is_some())
        .map_or_else(
            || Some("initial".to_owned()),
            |component| {
                project_observable_value(longhand, component).or_else(|| Some(component.to_owned()))
            },
        )
}

fn observable_text_emphasis_longhand(longhand: &str, input: &str) -> Option<String> {
    let components = split_top_level_whitespace(input)?;
    let color_index = components
        .iter()
        .position(|component| typed_longhand_value("text-emphasis-color", component).is_some());
    match longhand {
        "text-emphasis-color" => color_index.map_or_else(
            || Some("initial".to_owned()),
            |index| {
                let source = components[index];
                project_observable_value(longhand, source).or_else(|| Some(source.to_owned()))
            },
        ),
        "text-emphasis-style" => {
            let source = components
                .iter()
                .enumerate()
                .filter_map(|(index, component)| (Some(index) != color_index).then_some(*component))
                .collect::<Vec<_>>()
                .join(" ");
            if source.is_empty() {
                return Some("initial".to_owned());
            }
            project_observable_value(longhand, &source).or(Some(source))
        }
        _ => None,
    }
}

fn observable_mask_images(input: &str) -> Option<String> {
    split_top_level_delimiter(input, b',')?
        .iter()
        .map(|layer| {
            let image = split_top_level_whitespace(layer)?
                .into_iter()
                .find(|component| is_image_component(component));
            Some(match image {
                Some(image) => project_observable_value("mask-image", image)
                    .unwrap_or_else(|| image.to_owned()),
                None => "initial".to_owned(),
            })
        })
        .collect::<Option<Vec<_>>>()
        .map(|layers| layers.join(", "))
}

fn observable_mask_positions(longhand: &str, input: &str) -> Option<String> {
    split_top_level_delimiter(input, b',')?
        .iter()
        .map(|layer| {
            let (x, y) = position_axis_components(layer)?;
            let value = match longhand {
                "-webkit-mask-position-x" => x,
                "-webkit-mask-position-y" => y,
                _ => return None,
            };
            if value == "initial" {
                return Some("0%".to_owned());
            }
            Some(project_observable_value(longhand, &value).unwrap_or(value))
        })
        .collect::<Option<Vec<_>>>()
        .map(|layers| layers.join(", "))
}

fn grid_uses_auto_flow(input: &str) -> bool {
    split_top_level_delimiter(input, b'/').is_some_and(|sections| {
        sections.len() == 2
            && sections.iter().any(|section| {
                split_top_level_whitespace(section)
                    .is_some_and(|components| components.contains(&"auto-flow"))
            })
    })
}

fn observable_background_longhand(longhand: &str, input: &str) -> Option<String> {
    let layers = split_top_level_delimiter(input, b',')?;
    if longhand == "background-color" {
        let value = observable_background_layer_value(longhand, layers.last()?)?;
        return Some(project_observable_value(longhand, &value).unwrap_or(value));
    }
    Some(
        layers
            .iter()
            .map(|layer| {
                let value = observable_background_layer_value(longhand, layer)?;
                Some(project_observable_value(longhand, &value).unwrap_or(value))
            })
            .collect::<Option<Vec<_>>>()?
            .join(", "),
    )
}

fn observable_background_layer_value(longhand: &str, input: &str) -> Option<String> {
    let components = split_top_level_whitespace(input)?;
    let color = components
        .iter()
        .find(|component| typed_longhand_value("color", component).is_some())
        .copied();
    let image = components
        .iter()
        .find(|component| is_image_component(component))
        .copied();
    let repeats = [
        "repeat",
        "repeat-x",
        "repeat-y",
        "no-repeat",
        "space",
        "round",
    ];
    let attachments = ["scroll", "fixed", "local"];
    let visual_boxes = ["border-box", "padding-box", "content-box"];
    let visual_box_values = components
        .iter()
        .filter(|component| visual_boxes.contains(component))
        .copied()
        .collect::<Vec<_>>();
    let has_border_area = components.contains(&"border-area");
    let has_text = components.contains(&"text");
    let special_clip = match (has_border_area, has_text) {
        (true, true) => Some("border-area text"),
        (true, false) => Some("border-area"),
        (false, true) => Some("text"),
        (false, false) => None,
    };
    let slash = components.iter().position(|component| *component == "/");
    let (position_x, position_y) = position_axis_components(input)?;
    let value = match longhand {
        "background-color" => color.unwrap_or("initial").to_owned(),
        "background-image" => image.unwrap_or("initial").to_owned(),
        "background-position-x" => position_x,
        "background-position-y" => position_y,
        "background-size" => {
            background_layer_size(&components, slash).unwrap_or("initial".to_owned())
        }
        "background-repeat" => components
            .iter()
            .find(|component| repeats.contains(component))
            .copied()
            .unwrap_or("initial")
            .to_owned(),
        "background-attachment" => components
            .iter()
            .find(|component| attachments.contains(component))
            .copied()
            .unwrap_or("initial")
            .to_owned(),
        "background-origin" => visual_box_values
            .first()
            .copied()
            .or_else(|| has_border_area.then_some("border-box"))
            .unwrap_or("initial")
            .to_owned(),
        "background-clip" => special_clip
            .or_else(|| visual_box_values.get(1).copied())
            .or_else(|| visual_box_values.first().copied())
            .unwrap_or("initial")
            .to_owned(),
        _ => return None,
    };
    Some(value)
}

fn background_layer_size(components: &[&str], slash: Option<usize>) -> Option<String> {
    let start = slash?.checked_add(1)?;
    let available = components.len().saturating_sub(start).min(2);
    for count in (1..=available).rev() {
        let candidate = components.get(start..start + count)?.join(" ");
        if typed_longhand_value("background-size", &candidate).is_some() {
            return Some(candidate);
        }
    }
    None
}

fn position_axis_components(input: &str) -> Option<(String, String)> {
    let components = split_top_level_whitespace(input).unwrap_or_default();
    let slash = components
        .iter()
        .position(|component| *component == "/")
        .unwrap_or(components.len());
    let positions = components[..slash]
        .iter()
        .filter(|component| {
            matches!(**component, "left" | "right" | "top" | "bottom" | "center")
                || typed_longhand_value("background-position-x", component).is_some()
                || typed_longhand_value("background-position-y", component).is_some()
        })
        .copied()
        .collect::<Vec<_>>();
    match positions.as_slice() {
        [] => Some(("initial".to_owned(), "initial".to_owned())),
        [value] if matches!(*value, "top" | "bottom") => {
            Some(("center".to_owned(), (*value).to_owned()))
        }
        [value] => Some(((*value).to_owned(), "center".to_owned())),
        [horizontal @ ("left" | "right"), x, vertical @ ("top" | "bottom"), y] => {
            Some((format!("{horizontal} {x}"), format!("{vertical} {y}")))
        }
        [first, second, ..] => Some(((*first).to_owned(), (*second).to_owned())),
    }
}

fn is_image_component(component: &str) -> bool {
    component == "none"
        || [
            "url(",
            "image(",
            "image-set(",
            "cross-fade(",
            "linear-gradient(",
            "radial-gradient(",
            "conic-gradient(",
            "repeating-linear-gradient(",
            "repeating-radial-gradient(",
            "repeating-conic-gradient(",
        ]
        .iter()
        .any(|prefix| component.starts_with(prefix))
}

fn expand_special_shorthand(
    name: &str,
    value: &str,
    important: bool,
    limits: ResourceLimits,
) -> Option<Vec<DeclarationRecord>> {
    if name == "font" && is_system_font(value) {
        return Some(
            SYSTEM_FONT_LONGHANDS
                .iter()
                .map(|longhand| DeclarationRecord {
                    name: (*longhand).to_owned(),
                    value: DeclarationValue::deferred(false),
                    important,
                    pending_group: None,
                    alias_value: None,
                })
                .collect(),
        );
    }

    if matches!(name, "column-rule" | "row-rule" | "rule") {
        return expand_gap_rule_records(name, value, important, limits);
    }
    if name == "-webkit-text-stroke" {
        return expand_text_stroke_records(value, important, limits);
    }
    if matches!(name, "rule-width" | "rule-style" | "rule-color") {
        return expand_gap_rule_component_records(name, value, important, limits);
    }
    if matches!(name, "column-rule-inset" | "row-rule-inset" | "rule-inset") {
        let values = expand_rule_inset(name, value)?;
        return records_from_values(name, value, values, important, limits);
    }
    if name == "contain-intrinsic-size" {
        let values = expand_contain_intrinsic_size(value)?;
        return records_from_values(name, value, values, important, limits);
    }

    let components = split_top_level_whitespace(value)?;
    let values = match name {
        "animation" | "-webkit-animation" => expand_contextual_animation(value)?,
        "columns" | "-webkit-columns" => expand_columns(value)?,
        "flex" | "-webkit-flex" => expand_contextual_flex(&components)?,
        "grid-area" => expand_contextual_grid_area(value)?,
        "grid-column" => {
            expand_contextual_grid_line(value, "grid-column-start", "grid-column-end")?
        }
        "grid-row" => expand_contextual_grid_line(value, "grid-row-start", "grid-row-end")?,
        "-webkit-mask-box-image" => expand_webkit_mask_box_image(value)?,
        "font-synthesis" => expand_font_synthesis(&components)?,
        "font-variant" => expand_font_variant(&components)?,
        "offset" => expand_offset(value)?,
        "position-try" => expand_position_try(value)?,
        "scroll-timeline" => expand_scroll_timeline(value)?,
        "text-box" => expand_text_box(&components)?,
        "text-decoration" => expand_text_decoration(&components)?,
        "text-wrap" => expand_text_wrap(&components)?,
        "transition" | "-webkit-transition" => expand_transition(value)?,
        "timeline-trigger" => expand_timeline_trigger(value)?,
        "timeline-trigger-activation-range" => expand_timeline_trigger_range(
            value,
            "timeline-trigger-activation-range-start",
            "timeline-trigger-activation-range-end",
            "normal",
        )?,
        "timeline-trigger-active-range" => expand_timeline_trigger_range(
            value,
            "timeline-trigger-active-range-start",
            "timeline-trigger-active-range-end",
            "auto",
        )?,
        "view-timeline" => expand_view_timeline(value)?,
        "white-space" => expand_white_space(&components)?,
        _ => return None,
    };
    records_from_values(name, value, values, important, limits)
}

fn expand_contain_intrinsic_size(value: &str) -> Option<Vec<(&'static str, String)>> {
    let components = split_top_level_whitespace(value)?;
    if components.is_empty() || components.len() > 4 {
        return None;
    }

    if let Some(value) = typed_longhand_value("contain-intrinsic-width", value) {
        return Some(vec![
            ("contain-intrinsic-width", value.clone()),
            ("contain-intrinsic-height", value),
        ]);
    }

    for split in 1..components.len() {
        let width = components[..split].join(" ");
        let height = components[split..].join(" ");
        let Some(width) = typed_longhand_value("contain-intrinsic-width", &width) else {
            continue;
        };
        let Some(height) = typed_longhand_value("contain-intrinsic-height", &height) else {
            continue;
        };
        return Some(vec![
            ("contain-intrinsic-width", width),
            ("contain-intrinsic-height", height),
        ]);
    }
    None
}

fn expand_gap_rule_records(
    shorthand: &str,
    source: &str,
    important: bool,
    limits: ResourceLimits,
) -> Option<Vec<DeclarationRecord>> {
    let expansion = expand_gap_rule(source).ok()?;
    observed_shorthand_longhands(shorthand)?
        .iter()
        .map(|longhand| {
            let (canonical, observable) = match gap_rule_component(longhand) {
                Some(GapRuleComponent::Width) => (&expansion.width, &expansion.width_observable),
                Some(GapRuleComponent::Style) => (&expansion.style, &expansion.style_observable),
                Some(GapRuleComponent::Color) => (&expansion.color, &expansion.color_observable),
                None => return None,
            };
            Some(DeclarationRecord {
                name: (*longhand).to_owned(),
                value: semantic_longhand_value(longhand, canonical, observable, limits)?,
                important,
                pending_group: None,
                alias_value: None,
            })
        })
        .collect()
}

fn expand_text_stroke_records(
    source: &str,
    important: bool,
    limits: ResourceLimits,
) -> Option<Vec<DeclarationRecord>> {
    let expansion = expand_text_stroke(source).ok()?;
    [
        (
            "-webkit-text-stroke-width",
            expansion.width,
            expansion.width_observable,
        ),
        (
            "-webkit-text-stroke-color",
            expansion.color,
            expansion.color_observable,
        ),
    ]
    .into_iter()
    .map(|(name, canonical, observable)| {
        Some(DeclarationRecord {
            name: name.to_owned(),
            value: semantic_longhand_value(name, &canonical, &observable, limits)?,
            important,
            pending_group: None,
            alias_value: None,
        })
    })
    .collect()
}

fn expand_gap_rule_component_records(
    shorthand: &str,
    source: &str,
    important: bool,
    limits: ResourceLimits,
) -> Option<Vec<DeclarationRecord>> {
    let representative = match shorthand {
        "rule-width" => "column-rule-width",
        "rule-style" => "column-rule-style",
        "rule-color" => "column-rule-color",
        _ => return None,
    };
    let canonical = canonical_gap_rule_longhand(representative, source).ok()?;
    observed_shorthand_longhands(shorthand)?
        .iter()
        .map(|longhand| {
            Some(DeclarationRecord {
                name: (*longhand).to_owned(),
                value: semantic_longhand_value(longhand, &canonical, source, limits)?,
                important,
                pending_group: None,
                alias_value: None,
            })
        })
        .collect()
}

fn expand_rule_inset<'a>(shorthand: &'a str, source: &'a str) -> Option<Vec<(&'a str, String)>> {
    let sections = split_top_level_delimiter(source, b'/')?;
    if sections.is_empty() || sections.len() > 2 {
        return None;
    }
    let cap = rule_inset_pair(sections[0])?;
    let junction = match sections.get(1) {
        Some(section) => rule_inset_pair(section)?,
        None => cap.clone(),
    };

    observed_shorthand_longhands(shorthand)?
        .iter()
        .map(|longhand| {
            let component = if longhand.ends_with("-cap-start") {
                cap[0]
            } else if longhand.ends_with("-cap-end") {
                cap[1]
            } else if longhand.ends_with("-junction-start") {
                junction[0]
            } else if longhand.ends_with("-junction-end") {
                junction[1]
            } else {
                return None;
            };
            typed_longhand_value(longhand, component).map(|value| (*longhand, value))
        })
        .collect()
}

fn rule_inset_pair(source: &str) -> Option<Vec<&str>> {
    let components = split_top_level_whitespace(source.trim())?;
    match components.as_slice() {
        [value] => Some(vec![*value, *value]),
        [start, end] => Some(vec![*start, *end]),
        _ => None,
    }
}

fn expand_timeline_trigger_range(
    value: &str,
    start_name: &'static str,
    end_name: &'static str,
    omitted_end: &str,
) -> Option<Vec<(&'static str, String)>> {
    let pairs = value_list(value)?
        .into_iter()
        .map(|value| parse_timeline_range_pair_or_auto(value, omitted_end))
        .collect::<Option<Vec<_>>>()?;
    let mut starts = Vec::with_capacity(pairs.len());
    let mut ends = Vec::with_capacity(pairs.len());
    for (start, end) in pairs {
        starts.push(start);
        ends.push(end);
    }
    Some(vec![
        (start_name, starts.join(", ")),
        (end_name, ends.join(", ")),
    ])
}

fn parse_timeline_range_pair_or_auto(value: &str, omitted_end: &str) -> Option<(String, String)> {
    let (start_name, end_name) = if omitted_end == "auto" {
        (
            "timeline-trigger-active-range-start",
            "timeline-trigger-active-range-end",
        )
    } else {
        (
            "timeline-trigger-activation-range-start",
            "timeline-trigger-activation-range-end",
        )
    };
    if let Some(start) = typed_longhand_value(start_name, value) {
        if start == "auto" {
            return Some((start, omitted_end.to_owned()));
        }
        let (_, end) =
            crate::browser_longhand::parse_timeline_range_pair(&start, omitted_end).ok()?;
        return Some((start, end));
    }

    let components = split_top_level_whitespace(value)?;
    for split in (1..components.len()).rev() {
        let Some(start) = typed_longhand_value(start_name, &components[..split].join(" ")) else {
            continue;
        };
        let Some(end) = typed_longhand_value(end_name, &components[split..].join(" ")) else {
            continue;
        };
        return Some((start, end));
    }
    None
}

fn expand_position_try(value: &str) -> Option<Vec<(&'static str, String)>> {
    if value.eq_ignore_ascii_case("none") {
        return Some(vec![
            ("position-try-order", "normal".to_owned()),
            ("position-try-fallbacks", "none".to_owned()),
        ]);
    }
    if let Some(fallbacks) = typed_longhand_value("position-try-fallbacks", value) {
        return Some(vec![
            ("position-try-order", "normal".to_owned()),
            ("position-try-fallbacks", fallbacks),
        ]);
    }

    let mut components = split_top_level_whitespace(value)?;
    let order_index = components
        .iter()
        .position(|component| typed_longhand_value("position-try-order", component).is_some())?;
    let order = typed_longhand_value("position-try-order", components.remove(order_index))?;
    if components.is_empty() {
        return None;
    }
    let fallbacks = typed_longhand_value("position-try-fallbacks", &components.join(" "))?;
    Some(vec![
        ("position-try-order", order),
        ("position-try-fallbacks", fallbacks),
    ])
}

fn expand_timeline_trigger(value: &str) -> Option<Vec<(&'static str, String)>> {
    let items = value_list(value)?
        .into_iter()
        .map(expand_timeline_trigger_item)
        .collect::<Option<Vec<_>>>()?;
    const LONGHANDS: [&str; 6] = [
        "timeline-trigger-name",
        "timeline-trigger-source",
        "timeline-trigger-activation-range-start",
        "timeline-trigger-activation-range-end",
        "timeline-trigger-active-range-start",
        "timeline-trigger-active-range-end",
    ];
    Some(
        LONGHANDS
            .into_iter()
            .enumerate()
            .map(|(index, longhand)| {
                (
                    longhand,
                    items
                        .iter()
                        .map(|item| item[index].as_str())
                        .collect::<Vec<_>>()
                        .join(", "),
                )
            })
            .collect(),
    )
}

fn expand_timeline_trigger_item(value: &str) -> Option<[String; 6]> {
    let trimmed = value.trim();
    let sections = if let Some(active) = trimmed.strip_prefix('/') {
        let active_sections = split_top_level_delimiter(active.trim(), b'/')?;
        if active_sections.len() != 1 {
            return None;
        }
        vec!["", active_sections[0]]
    } else {
        split_top_level_delimiter(trimmed, b'/')?
    };
    if sections.len() > 2 {
        return None;
    }

    let mut components = split_top_level_whitespace(sections.first()?.trim())?;
    let name = if let Some(value) = components
        .first()
        .and_then(|component| typed_longhand_value("timeline-trigger-name", component))
    {
        components.remove(0);
        value
    } else {
        "none".to_owned()
    };
    let source = if let Some(value) = components
        .first()
        .and_then(|component| typed_longhand_value("timeline-trigger-source", component))
    {
        components.remove(0);
        value
    } else {
        "auto".to_owned()
    };
    let activation = if components.is_empty() {
        ("normal".to_owned(), "normal".to_owned())
    } else {
        parse_timeline_range_pair_or_auto(&components.join(" "), "normal")?
    };
    let active = match sections.get(1) {
        Some(value) => parse_timeline_range_pair_or_auto(value.trim(), "auto")?,
        None => ("auto".to_owned(), "auto".to_owned()),
    };
    Some([name, source, activation.0, activation.1, active.0, active.1])
}

fn contextual_longhand_value(name: &str, value: &str) -> Option<String> {
    let declaration =
        parse_semantic_property_with_limits(name, value, ResourceLimits::default()).ok()?;
    if !matches!(
        declaration.parse_kind(),
        PropertyParseKind::Typed | PropertyParseKind::SheetomTyped
    ) {
        return None;
    }
    if declaration.parse_kind() != PropertyParseKind::SheetomTyped
        && !declaration.recovered().contains_context_dependent_sign()
    {
        return None;
    }
    declaration.canonical_value().ok()
}

fn expand_contextual_animation(value: &str) -> Option<Vec<(&'static str, String)>> {
    let layers = split_top_level_delimiter(value, b',')?;
    let mut replacements = Vec::with_capacity(layers.len());
    let mut iterations = Vec::with_capacity(layers.len());
    let mut bare_iterations = Vec::with_capacity(layers.len());
    for layer in layers {
        let components = split_top_level_whitespace(layer)?;
        let contextual = components
            .iter()
            .enumerate()
            .filter_map(|(index, component)| {
                contextual_longhand_value("animation-iteration-count", component)
                    .map(|value| (index, value))
            })
            .collect::<Vec<_>>();
        let mut replaced = components
            .iter()
            .map(|component| (*component).to_owned())
            .collect::<Vec<_>>();
        let iteration = match contextual.as_slice() {
            [] => None,
            [(index, iteration)] => {
                replaced[*index] = "1".to_owned();
                Some(iteration.clone())
            }
            _ => return None,
        };
        replacements.push(replaced.join(" "));
        bare_iterations.push(iteration.is_some() && components.len() == 1);
        iterations.push(iteration);
    }
    if iterations.iter().all(Option::is_none) {
        return None;
    }
    let mut values = expand_typed_shorthand_values("animation", &replacements.join(", "))?;
    let iteration = values
        .iter_mut()
        .find(|(name, _)| *name == "animation-iteration-count")?;
    let mut iteration_values = value_list(&iteration.1)?
        .into_iter()
        .map(str::to_owned)
        .collect::<Vec<_>>();
    if iteration_values.len() != iterations.len() {
        return None;
    }
    for (value, replacement) in iteration_values.iter_mut().zip(iterations) {
        if let Some(replacement) = replacement {
            *value = replacement;
        }
    }
    iteration.1 = iteration_values.join(", ");
    let duration = values
        .iter_mut()
        .find(|(name, _)| *name == "animation-duration")?;
    let mut durations = value_list(&duration.1)?
        .into_iter()
        .map(str::to_owned)
        .collect::<Vec<_>>();
    if durations.len() != bare_iterations.len() {
        return None;
    }
    for (duration, bare) in durations.iter_mut().zip(bare_iterations) {
        if bare {
            *duration = "auto".to_owned();
        }
    }
    duration.1 = durations.join(", ");
    Some(values)
}

fn expand_contextual_flex(components: &[&str]) -> Option<Vec<(&'static str, String)>> {
    if components.is_empty() || components.len() > 3 {
        return None;
    }
    let contextual = components.iter().any(|component| {
        contextual_longhand_value("flex-grow", component).is_some()
            || parse_contextual_dimension_calculation(component).is_some()
    });
    if !contextual {
        return None;
    }
    let (grow, shrink, basis) = match components {
        [single] => {
            if let Some(grow) = typed_longhand_value("flex-grow", single) {
                (grow, "1".to_owned(), "0%".to_owned())
            } else {
                ("1".to_owned(), "1".to_owned(), typed_flex_basis(single)?)
            }
        }
        [first, second] => {
            let grow = typed_longhand_value("flex-grow", first)?;
            if let Some(shrink) = typed_longhand_value("flex-shrink", second) {
                (grow, shrink, "0%".to_owned())
            } else {
                (grow, "1".to_owned(), typed_flex_basis(second)?)
            }
        }
        [first, second, third] => (
            typed_longhand_value("flex-grow", first)?,
            typed_longhand_value("flex-shrink", second)?,
            typed_flex_basis(third)?,
        ),
        _ => return None,
    };
    Some(vec![
        ("flex-grow", grow),
        ("flex-shrink", shrink),
        ("flex-basis", basis),
    ])
}

fn typed_flex_basis(value: &str) -> Option<String> {
    if leading_function_is(value, "calc-size") {
        return None;
    }
    parse_contextual_dimension_calculation(value)
        .or_else(|| typed_longhand_value("flex-basis", value))
}

fn leading_function_is(value: &str, expected: &str) -> bool {
    let mut input = ParserInput::new(value);
    let mut parser = Parser::new(&mut input);
    matches!(
        parser.next(),
        Ok(Token::Function(name)) if name.eq_ignore_ascii_case(expected)
    )
}

fn expand_contextual_grid_line(
    value: &str,
    start_name: &'static str,
    end_name: &'static str,
) -> Option<Vec<(&'static str, String)>> {
    let sections = split_top_level_delimiter(value, b'/')?;
    if sections.is_empty() || sections.len() > 2 {
        return None;
    }
    let has_contextual = sections.iter().enumerate().any(|(index, value)| {
        let name = if index == 0 { start_name } else { end_name };
        contextual_longhand_value(name, value.trim()).is_some()
    });
    if !has_contextual {
        return None;
    }
    let start = typed_longhand_value(start_name, sections[0].trim())?;
    let end = match sections.get(1) {
        Some(value) => typed_longhand_value(end_name, value.trim())?,
        None => "auto".to_owned(),
    };
    Some(vec![(start_name, start), (end_name, end)])
}

fn expand_contextual_grid_area(value: &str) -> Option<Vec<(&'static str, String)>> {
    let names = [
        "grid-row-start",
        "grid-column-start",
        "grid-row-end",
        "grid-column-end",
    ];
    let sections = split_top_level_delimiter(value, b'/')?;
    if sections.is_empty() || sections.len() > names.len() {
        return None;
    }
    let has_contextual = sections
        .iter()
        .enumerate()
        .any(|(index, value)| contextual_longhand_value(names[index], value.trim()).is_some());
    if !has_contextual {
        return None;
    }
    let first = typed_longhand_value(names[0], sections[0].trim())?;
    let mut values = Vec::with_capacity(names.len());
    values.push((names[0], first));
    for (index, name) in names.iter().enumerate().skip(1) {
        let value = match sections.get(index) {
            Some(value) => typed_longhand_value(name, value.trim())?,
            None => "auto".to_owned(),
        };
        values.push((*name, value));
    }
    Some(values)
}

fn expand_typed_shorthand_values(name: &str, value: &str) -> Option<Vec<(&'static str, String)>> {
    let property = parse_typed_property(name, value).ok()?;
    observed_shorthand_longhands(name)?
        .iter()
        .map(|longhand_name| {
            let value = shorthand_longhand(&property, name, longhand_name)
                .and_then(|longhand| longhand.value_to_css_string(PrinterOptions::default()).ok())
                .or_else(|| initial_longhand_value(longhand_name).map(str::to_owned))?;
            Some((*longhand_name, value))
        })
        .collect()
}

const SYSTEM_FONT_LONGHANDS: &[&str] = &[
    "font-style",
    "font-variant-ligatures",
    "font-variant-caps",
    "font-variant-numeric",
    "font-variant-east-asian",
    "font-variant-alternates",
    "font-variant-position",
    "font-variant-emoji",
    "font-weight",
    "font-stretch",
    "font-size",
    "line-height",
    "font-family",
    "font-optical-sizing",
    "font-size-adjust",
    "font-kerning",
    "font-feature-settings",
    "font-variation-settings",
    "font-language-override",
];

fn is_system_font(value: &str) -> bool {
    matches!(
        value,
        "caption" | "icon" | "menu" | "message-box" | "small-caption" | "status-bar"
    )
}

fn records_from_values(
    shorthand: &str,
    shorthand_input: &str,
    values: Vec<(&str, String)>,
    important: bool,
    limits: ResourceLimits,
) -> Option<Vec<DeclarationRecord>> {
    let longhands = observed_shorthand_longhands(shorthand)?;
    if values.len() != longhands.len() {
        return None;
    }
    let expansion_order =
        if shorthand == "font-variant" && !matches!(shorthand_input, "normal" | "none") {
            values.iter().map(|(name, _)| *name).collect::<Vec<_>>()
        } else {
            longhands.to_vec()
        };
    expansion_order
        .iter()
        .map(|longhand| {
            let value = values
                .iter()
                .find_map(|(name, value)| (*name == *longhand).then_some(value))?;
            let source = if longhand.contains("rule-inset") {
                value.clone()
            } else {
                authored_longhand_source(longhand, value, shorthand_input)
            };
            Some(DeclarationRecord {
                name: (*longhand).to_owned(),
                value: semantic_longhand_value(longhand, value, &source, limits)?,
                important,
                pending_group: None,
                alias_value: None,
            })
        })
        .collect()
}

fn expand_columns(value: &str) -> Option<Vec<(&'static str, String)>> {
    let sections = split_top_level_delimiter(value, b'/')?;
    if sections.is_empty() || sections.len() > 2 {
        return None;
    }
    let components = split_top_level_whitespace(sections[0])?;
    if components.is_empty() || components.len() > 2 {
        return None;
    }
    let mut width = "auto".to_owned();
    let mut count = "auto".to_owned();
    for component in components {
        if component == "auto" {
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
    let height = match sections.as_slice() {
        [_] => "auto".to_owned(),
        [_, height] => typed_longhand_value("column-height", height.trim())?,
        _ => return None,
    };
    Some(vec![
        ("column-width", width),
        ("column-count", count),
        ("column-height", height),
        ("column-wrap", "auto".to_owned()),
    ])
}

fn expand_border_image(value: &str) -> Option<Vec<(&'static str, String)>> {
    if value == "none" {
        return Some(vec![
            ("border-image-source", "none".to_owned()),
            ("border-image-slice", "100%".to_owned()),
            ("border-image-width", "1".to_owned()),
            ("border-image-outset", "0".to_owned()),
            ("border-image-repeat", "stretch".to_owned()),
        ]);
    }
    let sections = split_top_level_delimiter(value, b'/')?;
    if sections.is_empty() || sections.len() > 3 {
        return None;
    }
    let mut first = split_top_level_whitespace(sections[0])?;
    let source_index = first.iter().position(|component| {
        *component == "none"
            || component.starts_with("url(")
            || component.contains("gradient(")
            || component.starts_with("image(")
            || component.starts_with("image-set(")
    });
    let source = source_index
        .map(|index| first.remove(index).to_owned())
        .unwrap_or_else(|| "none".to_owned());
    let source = typed_longhand_value("border-image-source", &source)?;
    let repeat_values = first
        .iter()
        .enumerate()
        .filter(|(_, component)| matches!(**component, "stretch" | "repeat" | "round" | "space"))
        .map(|(index, component)| (index, *component))
        .collect::<Vec<_>>();
    if repeat_values.len() > 2 {
        return None;
    }
    let mut repeat = if repeat_values.is_empty() {
        "stretch".to_owned()
    } else {
        repeat_values
            .iter()
            .map(|(_, value)| *value)
            .collect::<Vec<_>>()
            .join(" ")
    };
    for (index, _) in repeat_values.into_iter().rev() {
        first.remove(index);
    }
    let fill = first
        .iter()
        .position(|component| *component == "fill")
        .map(|index| first.remove(index));
    if first.len() > 4 {
        return None;
    }
    let slice = if first.is_empty() {
        "100%".to_owned()
    } else {
        typed_longhand_value("border-image-slice", &first.join(" "))?
    };
    let slice = if fill.is_some() {
        format!("{slice} fill")
    } else {
        slice
    };
    let width = sections.get(1).copied().unwrap_or("1").trim();
    let width = validate_repeated_longhand_components("border-image-width", width, 4)?;
    let mut outset = "0".to_owned();
    if let Some(last) = sections.get(2) {
        let mut components = split_top_level_whitespace(last)?;
        if components
            .last()
            .is_some_and(|component| matches!(*component, "stretch" | "repeat" | "round" | "space"))
        {
            if repeat != "stretch" {
                return None;
            }
            let trailing_repeat = components.pop()?.to_owned();
            if components.last().is_some_and(|component| {
                matches!(*component, "stretch" | "repeat" | "round" | "space")
            }) {
                let first_repeat = components.pop()?;
                repeat = format!("{first_repeat} {trailing_repeat}");
            } else {
                repeat = trailing_repeat;
            }
        }
        if !components.is_empty() {
            outset = validate_repeated_longhand_components(
                "border-image-outset",
                &components.join(" "),
                4,
            )?;
        }
    }
    Some(vec![
        ("border-image-source", source),
        ("border-image-slice", slice),
        ("border-image-width", width),
        ("border-image-outset", outset),
        ("border-image-repeat", repeat),
    ])
}

#[derive(Default)]
struct WebkitMaskBoxImageGlobals {
    source: Option<String>,
    repeat: Option<String>,
}

fn expand_webkit_mask_box_image(value: &str) -> Option<Vec<(&'static str, String)>> {
    let sections = split_top_level_delimiter_allow_empty(value, b'/')?;
    if sections.is_empty() || sections.len() > 3 {
        return None;
    }

    let first_components = split_top_level_whitespace(sections[0])?;
    let require_slice = sections.len() > 1;
    let (leading_globals, slice) =
        parse_webkit_mask_box_image_first_section(&first_components, require_slice)?;

    let mut width = "initial".to_owned();
    let mut outset = "initial".to_owned();
    let mut trailing_globals = WebkitMaskBoxImageGlobals::default();

    if sections.len() == 2 {
        let components = split_top_level_whitespace(sections[1])?;
        let (parsed_width, globals) =
            parse_webkit_mask_box_image_tail(&components, "-webkit-mask-box-image-width")?;
        width = parsed_width;
        trailing_globals = globals;
    } else if sections.len() == 3 {
        let width_components = split_top_level_whitespace(sections[1])?;
        if !width_components.is_empty() {
            width = canonical_repeated_longhand_components(
                "-webkit-mask-box-image-width",
                &width_components,
            )?;
        }

        let outset_components = split_top_level_whitespace(sections[2])?;
        let (parsed_outset, globals) =
            parse_webkit_mask_box_image_tail(&outset_components, "-webkit-mask-box-image-outset")?;
        outset = parsed_outset;
        trailing_globals = globals;
    }

    let globals = merge_webkit_mask_box_image_globals(leading_globals, trailing_globals)?;
    Some(vec![
        (
            "-webkit-mask-box-image-source",
            globals.source.unwrap_or_else(|| "initial".to_owned()),
        ),
        (
            "-webkit-mask-box-image-slice",
            slice.unwrap_or_else(|| "initial".to_owned()),
        ),
        ("-webkit-mask-box-image-width", width),
        ("-webkit-mask-box-image-outset", outset),
        (
            "-webkit-mask-box-image-repeat",
            globals.repeat.unwrap_or_else(|| "initial".to_owned()),
        ),
    ])
}

fn parse_webkit_mask_box_image_first_section(
    components: &[&str],
    require_slice: bool,
) -> Option<(WebkitMaskBoxImageGlobals, Option<String>)> {
    if components.len() > 8 {
        return None;
    }
    for split in 0..=components.len() {
        let Some(globals) = parse_webkit_mask_box_image_globals(&components[..split]) else {
            continue;
        };
        let Some(slice) = parse_webkit_mask_box_image_slice(&components[split..]) else {
            continue;
        };
        if require_slice && slice.is_none() {
            continue;
        }
        if components.is_empty()
            && slice.is_none()
            && globals.source.is_none()
            && globals.repeat.is_none()
        {
            continue;
        }
        return Some((globals, slice));
    }
    None
}

fn parse_webkit_mask_box_image_tail(
    components: &[&str],
    longhand: &str,
) -> Option<(String, WebkitMaskBoxImageGlobals)> {
    if components.len() > 7 {
        return None;
    }
    for split in 1..=components.len() {
        let Some(value) = canonical_repeated_longhand_components(longhand, &components[..split])
        else {
            continue;
        };
        let Some(globals) = parse_webkit_mask_box_image_globals(&components[split..]) else {
            continue;
        };
        return Some((value, globals));
    }
    None
}

fn parse_webkit_mask_box_image_slice(components: &[&str]) -> Option<Option<String>> {
    if components.is_empty() {
        return Some(None);
    }
    let mut values = Vec::with_capacity(components.len());
    let mut fill = false;
    for component in components {
        if css_identifier_is(component, "fill") {
            if fill {
                return None;
            }
            fill = true;
        } else {
            values.push(*component);
        }
    }
    if values.is_empty() {
        return None;
    }
    let value = canonical_repeated_longhand_components("-webkit-mask-box-image-slice", &values)?;
    Some(Some(format!("{value} fill")))
}

fn parse_webkit_mask_box_image_globals(components: &[&str]) -> Option<WebkitMaskBoxImageGlobals> {
    if components.len() > 3 {
        return None;
    }
    if components.is_empty() {
        return Some(WebkitMaskBoxImageGlobals::default());
    }

    if let Some(source) = canonical_webkit_mask_box_image_source(components[0]) {
        let repeat = if components.len() == 1 {
            None
        } else {
            Some(canonical_webkit_mask_box_image_repeat(&components[1..])?)
        };
        return Some(WebkitMaskBoxImageGlobals {
            source: Some(source),
            repeat,
        });
    }

    for repeat_length in [2, 1] {
        if components.len() < repeat_length {
            continue;
        }
        let Some(repeat) = canonical_webkit_mask_box_image_repeat(&components[..repeat_length])
        else {
            continue;
        };
        let source = match components.get(repeat_length..) {
            Some([]) => None,
            Some([source]) => Some(canonical_webkit_mask_box_image_source(source)?),
            _ => continue,
        };
        return Some(WebkitMaskBoxImageGlobals {
            source,
            repeat: Some(repeat),
        });
    }
    None
}

fn merge_webkit_mask_box_image_globals(
    leading: WebkitMaskBoxImageGlobals,
    trailing: WebkitMaskBoxImageGlobals,
) -> Option<WebkitMaskBoxImageGlobals> {
    if leading.source.is_some() && trailing.source.is_some()
        || leading.repeat.is_some() && trailing.repeat.is_some()
    {
        return None;
    }
    Some(WebkitMaskBoxImageGlobals {
        source: leading.source.or(trailing.source),
        repeat: leading.repeat.or(trailing.repeat),
    })
}

fn canonical_webkit_mask_box_image_source(component: &str) -> Option<String> {
    let value = typed_longhand_value("-webkit-mask-box-image-source", component)?;
    (!is_css_wide_keyword(&value)).then_some(value)
}

fn canonical_webkit_mask_box_image_repeat(components: &[&str]) -> Option<String> {
    if components.is_empty() || components.len() > 2 {
        return None;
    }
    let values = components
        .iter()
        .map(|component| {
            ["stretch", "repeat", "round", "space"]
                .iter()
                .find(|keyword| css_identifier_is(component, keyword))
                .map(|keyword| (*keyword).to_owned())
        })
        .collect::<Option<Vec<_>>>()?;
    Some(if values.len() == 2 && values[0] == values[1] {
        values[0].clone()
    } else {
        values.join(" ")
    })
}

fn canonical_repeated_longhand_components(name: &str, components: &[&str]) -> Option<String> {
    if components.is_empty() || components.len() > 4 {
        return None;
    }
    let canonical = components
        .iter()
        .map(|component| typed_longhand_value(name, component))
        .collect::<Option<Vec<_>>>()?;
    let expanded = match canonical.as_slice() {
        [first] => vec![first.clone(), first.clone(), first.clone(), first.clone()],
        [first, second] => vec![first.clone(), second.clone(), first.clone(), second.clone()],
        [first, second, third] => {
            vec![first.clone(), second.clone(), third.clone(), second.clone()]
        }
        [first, second, third, fourth] => {
            vec![first.clone(), second.clone(), third.clone(), fourth.clone()]
        }
        _ => return None,
    };
    compress_four_values(expanded)
}

fn css_identifier_is(value: &str, expected: &str) -> bool {
    let mut input = ParserInput::new(value);
    let mut parser = Parser::new(&mut input);
    matches!(
        parser.next(),
        Ok(Token::Ident(identifier)) if identifier.eq_ignore_ascii_case(expected)
    ) && parser.is_exhausted()
}

fn validate_repeated_longhand_components(
    name: &str,
    value: &str,
    maximum: usize,
) -> Option<String> {
    let components = split_top_level_whitespace(value)?;
    if components.is_empty() || components.len() > maximum {
        return None;
    }
    components
        .iter()
        .map(|component| typed_longhand_value(name, component))
        .collect::<Option<Vec<_>>>()
        .map(|components| components.join(" "))
}

pub(crate) fn canonicalize_webkit_border_image(value: &str) -> Option<String> {
    let values = expand_border_image(value)?;
    let get = |name: &str| {
        values
            .iter()
            .find_map(|(candidate, value)| (*candidate == name).then_some(value.as_str()))
    };
    let source = get("border-image-source")?;
    let slice = get("border-image-slice")?;
    let width = get("border-image-width")?;
    let outset = get("border-image-outset")?;
    let repeat = get("border-image-repeat")?;
    let slice = if split_top_level_whitespace(slice)?.contains(&"fill") {
        slice.to_owned()
    } else {
        format!("{slice} fill")
    };
    Some(format!("{source} {slice} / {width} / {outset} {repeat}"))
}

fn validate_column_width(value: &str) -> Option<String> {
    if let Some(value) = parse_contextual_dimension_calculation(value) {
        return Some(value);
    }
    if value.parse::<f64>().is_ok() && value != "0" {
        return None;
    }
    typed_longhand_value("column-width", value)
}

fn validate_column_count(value: &str) -> Option<String> {
    if let Some(value) = contextual_longhand_value("column-count", value) {
        return Some(value);
    }
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
    if components == ["none"] {
        return Some(
            shorthand_longhands("font-variant")?
                .iter()
                .map(|longhand| {
                    let value = if *longhand == "font-variant-ligatures" {
                        "none"
                    } else {
                        "normal"
                    };
                    (*longhand, value.to_owned())
                })
                .collect(),
        );
    }
    if components == ["normal"] {
        return Some(
            shorthand_longhands("font-variant")?
                .iter()
                .map(|longhand| (*longhand, "normal".to_owned()))
                .collect(),
        );
    }
    const COMPONENT_LONGHANDS: &[&str] = &[
        "font-variant-ligatures",
        "font-variant-numeric",
        "font-variant-east-asian",
        "font-variant-caps",
        "font-variant-alternates",
        "font-variant-position",
        "font-variant-emoji",
    ];
    let mut grouped = COMPONENT_LONGHANDS
        .iter()
        .map(|longhand| (*longhand, Vec::new()))
        .collect::<Vec<_>>();
    for component in components {
        let candidates = COMPONENT_LONGHANDS
            .iter()
            .filter(|longhand| typed_longhand_value(longhand, component).is_some())
            .copied()
            .collect::<Vec<_>>();
        let [longhand] = candidates.as_slice() else {
            return None;
        };
        grouped
            .iter_mut()
            .find(|(candidate, _)| candidate == longhand)?
            .1
            .push(*component);
    }
    grouped
        .into_iter()
        .map(|(longhand, components)| {
            let source = if components.is_empty() {
                "normal".to_owned()
            } else {
                components.join(" ")
            };
            typed_longhand_value(longhand, &source).map(|value| (longhand, value))
        })
        .collect()
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
    let components = split_top_level_whitespace(slash[0])?;
    let (offset_position, path, distance, offset_rotate) = parse_offset_main(&components)?;
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

fn parse_offset_main(components: &[&str]) -> Option<(String, String, String, String)> {
    for position_length in (1..=components.len()).rev() {
        let position =
            typed_longhand_value("offset-position", &components[..position_length].join(" "));
        let Some(position) = position else {
            continue;
        };
        if position_length == components.len() {
            return Some((
                position,
                "none".to_owned(),
                "0px".to_owned(),
                "auto".to_owned(),
            ));
        }
        if let Some((path, distance, rotation)) =
            parse_offset_path_tail(&components[position_length..])
        {
            return Some((position, path, distance, rotation));
        }
    }

    let (path, distance, rotation) = parse_offset_path_tail(components)?;
    Some(("normal".to_owned(), path, distance, rotation))
}

fn parse_offset_path_tail(components: &[&str]) -> Option<(String, String, String)> {
    for path_length in (1..=components.len()).rev() {
        let Some(path) = offset_path_value(&components[..path_length].join(" ")) else {
            continue;
        };
        let Some((distance, rotation)) = parse_offset_motion_components(&components[path_length..])
        else {
            continue;
        };
        return Some((path, distance, rotation));
    }
    None
}

fn parse_offset_motion_components(components: &[&str]) -> Option<(String, String)> {
    let mut distance = None;
    let mut rotation = Vec::new();
    for component in components {
        if distance.is_none() {
            if let Some(value) = offset_distance_value(component) {
                distance = Some(value);
                continue;
            }
        }
        rotation.push(*component);
    }
    let rotation = if rotation.is_empty() {
        "auto".to_owned()
    } else {
        offset_rotate_value(&rotation)?
    };
    Some((distance.unwrap_or_else(|| "0px".to_owned()), rotation))
}

fn offset_path_value(value: &str) -> Option<String> {
    typed_longhand_value("offset-path", value)
}

fn offset_distance_value(value: &str) -> Option<String> {
    typed_longhand_value("offset-distance", value)
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
    let declaration =
        parse_semantic_property_with_limits(name, value, ResourceLimits::default()).ok()?;
    if !matches!(
        declaration.parse_kind(),
        PropertyParseKind::Typed | PropertyParseKind::SheetomTyped
    ) {
        return None;
    }
    declaration.canonical_value().ok()
}

fn expand_scroll_timeline(value: &str) -> Option<Vec<(&'static str, String)>> {
    let mut names = Vec::new();
    let mut axes = Vec::new();
    for entry in split_top_level_delimiter(value, b',')? {
        let components = split_top_level_whitespace(entry)?;
        if components.is_empty() || components.len() > 2 {
            return None;
        }
        let mut name = None;
        let mut axis = None;
        for component in components {
            if let Some(value) = typed_longhand_value("scroll-timeline-axis", component) {
                if axis.replace(value).is_some() {
                    return None;
                }
                continue;
            }
            let value = typed_longhand_value("scroll-timeline-name", component)?;
            if name.replace(value).is_some() {
                return None;
            }
        }
        names.push(name?);
        axes.push(axis.unwrap_or_else(|| "block".to_owned()));
    }
    Some(vec![
        ("scroll-timeline-name", names.join(", ")),
        ("scroll-timeline-axis", axes.join(", ")),
    ])
}

fn expand_text_box(components: &[&str]) -> Option<Vec<(&'static str, String)>> {
    if matches!(components, ["normal"]) {
        return Some(vec![
            ("text-box-trim", "none".to_owned()),
            ("text-box-edge", "auto".to_owned()),
        ]);
    }
    if components.is_empty() || components.len() > 3 {
        return None;
    }

    let source = components.join(" ");
    if let Some(trim) = typed_longhand_value("text-box-trim", &source) {
        return Some(vec![
            ("text-box-trim", trim),
            ("text-box-edge", "auto".to_owned()),
        ]);
    }
    if let Some(edge) = typed_longhand_value("text-box-edge", &source) {
        return Some(vec![
            ("text-box-trim", "trim-both".to_owned()),
            ("text-box-edge", edge),
        ]);
    }

    for trim_index in [0, components.len() - 1] {
        let trim = typed_longhand_value("text-box-trim", components[trim_index]);
        let edge = if trim_index == 0 {
            components[1..].join(" ")
        } else {
            components[..trim_index].join(" ")
        };
        if let (Some(trim), Some(edge)) = (trim, typed_longhand_value("text-box-edge", &edge)) {
            return Some(vec![("text-box-trim", trim), ("text-box-edge", edge)]);
        }
    }
    None
}

fn expand_text_decoration(components: &[&str]) -> Option<Vec<(&'static str, String)>> {
    if components.is_empty() {
        return None;
    }
    let line_keywords = [
        "none",
        "underline",
        "overline",
        "line-through",
        "blink",
        "spelling-error",
        "grammar-error",
    ];
    let style_keywords = ["solid", "double", "dotted", "dashed", "wavy"];
    let mut lines = Vec::new();
    let mut thickness = None;
    let mut style = None;
    let mut color = None;
    for component in components {
        if line_keywords.contains(component) {
            lines.push(*component);
        } else if style_keywords.contains(component) {
            if style.replace(*component).is_some() {
                return None;
            }
        } else if typed_longhand_value("text-decoration-thickness", component).is_some() {
            if thickness.replace(*component).is_some() {
                return None;
            }
        } else if typed_longhand_value("color", component).is_some() {
            if color.replace(*component).is_some() {
                return None;
            }
        } else {
            return None;
        }
    }
    let line = if lines.is_empty() {
        "initial".to_owned()
    } else {
        typed_longhand_value("text-decoration-line", &lines.join(" "))?
    };
    Some(vec![
        ("text-decoration-line", line),
        (
            "text-decoration-thickness",
            thickness.unwrap_or("initial").to_owned(),
        ),
        (
            "text-decoration-style",
            style.unwrap_or("initial").to_owned(),
        ),
        (
            "text-decoration-color",
            color.unwrap_or("initial").to_owned(),
        ),
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
        if components.iter().any(|component| {
            *component != "auto" && typed_longhand_value("padding-top", component).is_none()
        }) {
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
            if duration.is_none() {
                if let Some(canonical) = typed_longhand_value("transition-duration", component) {
                    duration = Some(canonical);
                    continue;
                }
            }
            if delay.is_none() {
                if let Some(canonical) = typed_longhand_value("transition-delay", component) {
                    delay = Some(canonical);
                    continue;
                }
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
        [collapse @ ("collapse" | "preserve" | "preserve-breaks")] => (*collapse, "initial"),
        ["wrap"] => ("initial", "wrap"),
        [collapse @ ("collapse" | "preserve" | "preserve-breaks" | "break-spaces"), mode @ ("wrap" | "nowrap")]
        | [mode @ ("wrap" | "nowrap"), collapse @ ("collapse" | "preserve" | "preserve-breaks" | "break-spaces")] => {
            (*collapse, *mode)
        }
        _ => return None,
    };
    Some(vec![
        ("white-space-collapse", collapse.to_owned()),
        ("text-wrap-mode", mode.to_owned()),
    ])
}

fn validate_typed_shorthand_structure(name: &str, value: &str) -> bool {
    if matches!(name, "border-image" | "-webkit-border-image")
        && crate::property_constraints::has_direct_negative_component(value)
    {
        return false;
    }
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

    let source_name = match border_component(longhand_name)? {
        BorderComponent::Width => "border-top-width",
        BorderComponent::Style => "border-top-style",
        BorderComponent::Color => "border-top-color",
    };
    property.longhand(&PropertyId::from(source_name))
}

fn parse_typed_property<'i>(name: &'i str, value: &'i str) -> Result<Property<'i>, EngineError> {
    let parser_name = sheetom_parser_property_name(name).unwrap_or(name);
    let property = Property::parse_string(
        PropertyId::from(parser_name),
        value,
        ParserOptions::default(),
    )
    .map_err(|error| EngineError::Parse(error.to_string()))?;
    if matches!(property, Property::Unparsed(_) | Property::Custom(_)) {
        return Err(EngineError::Parse(format!(
            "shorthand requires a typed grammar: {name}: {value}"
        )));
    }
    crate::property_constraints::validate_standard_property(name, value, &property)?;
    Ok(property)
}

fn map_engine_error(_: EngineError) -> MutationOutcome {
    MutationOutcome::InvalidValue
}

fn expand_structural_shorthand(
    name: &str,
    value: &str,
    important: bool,
    limits: ResourceLimits,
) -> Option<Vec<DeclarationRecord>> {
    let longhands = observed_shorthand_longhands(name)?;
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
            value: semantic_longhand_value(
                longhand,
                &canonical,
                if longhand.contains("rule-inset") {
                    &canonical
                } else {
                    component.trim()
                },
                limits,
            )?,
            important,
            pending_group: None,
            alias_value: None,
        });
    }
    Some(records)
}

fn semantic_longhand_value(
    name: &str,
    canonical: &str,
    observable: &str,
    limits: ResourceLimits,
) -> Option<DeclarationValue> {
    if let Some(keyword) = css_wide_keyword(canonical) {
        return Some(DeclarationValue::css_wide(keyword));
    }
    let semantic = parse_semantic_property_with_limits(name, observable, limits)
        .or_else(|_| parse_semantic_property_with_limits(name, canonical, limits))
        .ok()?;
    let projection = project_declaration(&semantic).ok()?;
    let observable_projection =
        if name.contains("rule-inset") || semantic.recovered().contains_context_dependent_sign() {
            canonical.to_owned()
        } else {
            projection.observable
        };
    Some(DeclarationValue::semantic_with_canonical(
        semantic,
        canonical.to_owned(),
        observable_projection,
    ))
}

fn validated_expanded_shorthand_value(
    name: &str,
    source: &str,
    limits: ResourceLimits,
) -> Result<DeclarationValue, MutationOutcome> {
    let source = source.trim();
    let semantic =
        crate::SemanticDeclaration::from_validated_expanded_shorthand(name, source, limits)
            .map_err(map_engine_error)?;
    let projection = project_declaration(&semantic).map_err(map_engine_error)?;
    Ok(DeclarationValue::semantic_with_canonical(
        semantic,
        source.to_owned(),
        projection.observable,
    ))
}

fn structural_cardinality(name: &str, longhand_count: usize) -> Option<usize> {
    const ONE_VALUE: &[&str] = &[
        "column-rule-inset-end",
        "column-rule-inset-start",
        "row-rule-inset-end",
        "row-rule-inset-start",
        "rule-break",
        "rule-color",
        "rule-inset-end",
        "rule-inset-start",
        "rule-visibility-items",
    ];
    const TWO_VALUE: &[&str] = &[
        "border-block-color",
        "border-block-style",
        "border-block-width",
        "border-inline-color",
        "border-inline-style",
        "border-inline-width",
        "contain-intrinsic-size",
        "column-rule-inset-cap",
        "column-rule-inset-junction",
        "interest-delay",
        "overscroll-behavior",
        "row-rule-inset-cap",
        "row-rule-inset-junction",
        "rule-inset-cap",
        "rule-inset-junction",
        "rule-style",
        "timeline-trigger-activation-range",
        "timeline-trigger-active-range",
    ];
    const FOUR_VALUE: &[&str] = &[
        "corner-block-end-shape",
        "corner-block-start-shape",
        "corner-bottom-shape",
        "corner-inline-end-shape",
        "corner-inline-start-shape",
        "corner-left-shape",
        "corner-right-shape",
        "corner-shape",
        "corner-top-shape",
    ];

    if ONE_VALUE.contains(&name) {
        return Some(1);
    }
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
    if let Some(value) = typed_longhand_value(name, value) {
        return Some(value);
    }

    let validation_name = if name.starts_with("overscroll-behavior-") {
        return matches!(value, "auto" | "contain" | "none" | "chain").then(|| value.to_owned());
    } else if name.contains("rule-inset") {
        Some("padding-top")
    } else if name.ends_with("rule-break") {
        return matches!(value, "normal" | "none" | "intersection").then(|| value.to_owned());
    } else if name.ends_with("rule-visibility-items") {
        return matches!(value, "normal" | "all" | "between" | "around").then(|| value.to_owned());
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

    typed_longhand_value(validation_name?, value)
}

#[cfg(test)]
mod tests {
    use super::{border_component, parse_typed_property, shorthand_longhand, BorderComponent};
    use lightningcss::stylesheet::PrinterOptions;

    #[test]
    fn border_component_registry_excludes_similarly_suffixed_properties() {
        assert_eq!(
            border_component("border-inline-start-width"),
            Some(BorderComponent::Width)
        );
        assert_eq!(
            border_component("column-rule-style"),
            Some(BorderComponent::Style)
        );
        assert_eq!(
            border_component("row-rule-color"),
            Some(BorderComponent::Color)
        );
        assert_eq!(border_component("border-image-width"), None);
        assert_eq!(border_component("text-decoration-color"), None);
    }

    #[test]
    fn typed_border_image_owns_slash_sections_and_trailing_repeat() {
        let property = parse_typed_property("border-image", "none 1 fill / 1px repeat")
            .expect("Lightning should parse the complete shorthand");
        for (name, expected) in [
            ("border-image-source", "none"),
            ("border-image-slice", "1 fill"),
            ("border-image-width", "1px"),
            ("border-image-outset", "0"),
            ("border-image-repeat", "repeat"),
        ] {
            let actual = shorthand_longhand(&property, "border-image", name)
                .unwrap_or_else(|| panic!("typed shorthand should expose {name}"))
                .value_to_css_string(PrinterOptions::default())
                .expect("typed longhand should serialize");
            assert_eq!(actual, expected, "{name}");
        }

        let property = parse_typed_property("border-image", "none 1 / 1px / 0px")
            .expect("typed border image should retain an explicit zero dimension");
        let outset = shorthand_longhand(&property, "border-image", "border-image-outset")
            .expect("typed shorthand should expose its outset");
        assert_eq!(
            super::authored_longhand_source(
                "border-image-outset",
                &outset
                    .value_to_css_string(PrinterOptions::default())
                    .expect("outset should serialize"),
                "none 1 / 1px / 0px",
            ),
            "0px"
        );
    }
}
