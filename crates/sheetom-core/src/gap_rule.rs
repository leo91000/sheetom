use cssparser::{Parser, ParserInput};
use lightningcss::{
    error::ParserError,
    properties::border::{BorderSideWidth, LineStyle},
    stylesheet::PrinterOptions,
    traits::{Parse, ToCss},
    values::color::CssColor,
};

use crate::{observable::project_observable_value, EngineError};

type ParseError<'i> = cssparser::ParseError<'i, ParserError<'i>>;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum RepeatCount {
    Auto,
    Integer(i32),
}

#[derive(Clone, Debug, PartialEq)]
enum RepeatedItem<T> {
    Value(T),
    Repeat { count: RepeatCount, values: Vec<T> },
}

#[derive(Clone, Debug, PartialEq)]
pub struct RepeatedList<T> {
    items: Vec<RepeatedItem<T>>,
}

impl<'i, T> Parse<'i> for RepeatedList<T>
where
    T: Parse<'i>,
{
    fn parse<'t>(input: &mut Parser<'i, 't>) -> Result<Self, ParseError<'i>> {
        let leading_empty = input.try_parse(Parser::expect_comma).is_ok();
        let mut trailing_empty = false;
        let mut items = Vec::new();
        while !input.is_exhausted() {
            items.push(RepeatedItem::<T>::parse(input)?);
            if input.is_exhausted() {
                break;
            }
            input.expect_comma()?;
            if input.is_exhausted() {
                trailing_empty = true;
            }
        }
        let auto_indexes = items
            .iter()
            .enumerate()
            .filter_map(|(index, item)| {
                matches!(
                    item,
                    RepeatedItem::Repeat {
                        count: RepeatCount::Auto,
                        ..
                    }
                )
                .then_some(index)
            })
            .collect::<Vec<_>>();
        if auto_indexes.len() > 1 || items.is_empty() {
            return Err(input.new_custom_error(ParserError::InvalidValue));
        }
        if leading_empty {
            return Err(input.new_custom_error(ParserError::InvalidValue));
        }
        let Some(auto_index) = auto_indexes.first().copied() else {
            if leading_empty || trailing_empty {
                return Err(input.new_custom_error(ParserError::InvalidValue));
            }
            return Ok(Self { items });
        };
        if trailing_empty && auto_index + 1 != items.len() {
            return Err(input.new_custom_error(ParserError::InvalidValue));
        }
        Ok(Self { items })
    }
}

impl<'i, T> Parse<'i> for RepeatedItem<T>
where
    T: Parse<'i>,
{
    fn parse<'t>(input: &mut Parser<'i, 't>) -> Result<Self, ParseError<'i>> {
        if input
            .try_parse(|input| input.expect_function_matching("repeat"))
            .is_err()
        {
            return T::parse(input).map(Self::Value);
        }

        input.parse_nested_block(|input| {
            let count = if input
                .try_parse(|input| input.expect_ident_matching("auto"))
                .is_ok()
            {
                RepeatCount::Auto
            } else {
                let count = input.expect_integer()?;
                if count < 1 {
                    return Err(input.new_custom_error(ParserError::InvalidValue));
                }
                RepeatCount::Integer(count)
            };
            input.expect_comma()?;
            let values = input.parse_comma_separated(T::parse)?;
            Ok(Self::Repeat { count, values })
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Authored<T> {
    value: T,
    source: String,
    authored: bool,
}

impl<T: Default> Default for Authored<T> {
    fn default() -> Self {
        Self {
            value: T::default(),
            source: String::new(),
            authored: false,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
struct GapRule {
    width: Authored<BorderSideWidth>,
    style: Authored<LineStyle>,
    color: Authored<CssColor>,
}

impl<'i> Parse<'i> for GapRule {
    fn parse<'t>(input: &mut Parser<'i, 't>) -> Result<Self, ParseError<'i>> {
        let mut width = None;
        let mut style = None;
        let mut color = None;
        let mut consumed = false;

        loop {
            if width.is_none() {
                if let Ok(value) = input.try_parse(parse_authored::<BorderSideWidth>) {
                    width = Some(value);
                    consumed = true;
                    continue;
                }
            }
            if style.is_none() {
                if let Ok(value) = input.try_parse(parse_authored::<LineStyle>) {
                    style = Some(value);
                    consumed = true;
                    continue;
                }
            }
            if color.is_none() {
                if let Ok(value) = input.try_parse(parse_authored::<CssColor>) {
                    color = Some(value);
                    consumed = true;
                    continue;
                }
            }
            break;
        }

        if !consumed {
            return Err(input.new_custom_error(ParserError::InvalidValue));
        }
        Ok(Self {
            width: width.unwrap_or_else(|| Authored {
                value: BorderSideWidth::default(),
                source: "medium".to_owned(),
                authored: false,
            }),
            style: style.unwrap_or_else(|| Authored {
                value: LineStyle::default(),
                source: "none".to_owned(),
                authored: false,
            }),
            color: color.unwrap_or_else(|| Authored {
                value: CssColor::current_color(),
                source: "currentcolor".to_owned(),
                authored: false,
            }),
        })
    }
}

fn parse_authored<'i, 't, T>(input: &mut Parser<'i, 't>) -> Result<Authored<T>, ParseError<'i>>
where
    T: Parse<'i>,
{
    let start = input.position();
    let value = T::parse(input)?;
    Ok(Authored {
        value,
        source: input.slice_from(start).trim().to_owned(),
        authored: true,
    })
}

#[derive(Clone, Debug, PartialEq)]
pub enum GapRuleLonghandValue {
    Width(RepeatedList<Authored<BorderSideWidth>>),
    Style(RepeatedList<Authored<LineStyle>>),
    Color(RepeatedList<Authored<CssColor>>),
}

impl GapRuleLonghandValue {
    pub fn canonical_value(&self) -> Result<String, EngineError> {
        match self {
            Self::Width(value) => {
                serialize_repeated_list(value, |value| serialize_typed(&value.value))
            }
            Self::Style(value) => {
                serialize_repeated_list(value, |value| serialize_typed(&value.value))
            }
            Self::Color(value) => {
                serialize_repeated_list(value, |value| serialize_typed(&value.value))
            }
        }
    }

    pub(crate) fn observable_value(&self) -> Result<String, EngineError> {
        match self {
            Self::Width(value) => serialize_repeated_list(value, |value| {
                observable_component("border-top-width", value)
            }),
            Self::Style(value) => serialize_repeated_list(value, |value| {
                observable_component("border-top-style", value)
            }),
            Self::Color(value) => serialize_repeated_list(value, |value| {
                observable_component("border-top-color", value)
            }),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct GapRuleExpansion {
    pub(crate) width: String,
    pub(crate) width_observable: String,
    pub(crate) style: String,
    pub(crate) style_observable: String,
    pub(crate) color: String,
    pub(crate) color_observable: String,
}

pub(crate) struct BorderSideObservableExpansion {
    pub(crate) width: String,
    pub(crate) style: String,
    pub(crate) color: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct TextStrokeExpansion {
    pub(crate) width: String,
    pub(crate) width_observable: String,
    pub(crate) color: String,
    pub(crate) color_observable: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum GapRuleComponent {
    Width,
    Style,
    Color,
}

pub(crate) fn gap_rule_component(property_name: &str) -> Option<GapRuleComponent> {
    match property_name {
        "-webkit-column-rule-width" | "column-rule-width" | "row-rule-width" | "rule-width" => {
            Some(GapRuleComponent::Width)
        }
        "-webkit-column-rule-style" | "column-rule-style" | "row-rule-style" | "rule-style" => {
            Some(GapRuleComponent::Style)
        }
        "-webkit-column-rule-color" | "column-rule-color" | "row-rule-color" | "rule-color" => {
            Some(GapRuleComponent::Color)
        }
        _ => None,
    }
}

#[derive(Clone, Debug, PartialEq)]
struct TextStroke {
    width: Authored<BorderSideWidth>,
    color: Authored<CssColor>,
}

impl<'i> Parse<'i> for TextStroke {
    fn parse<'t>(input: &mut Parser<'i, 't>) -> Result<Self, ParseError<'i>> {
        let mut width = None;
        let mut color = None;
        let mut consumed = false;

        loop {
            if width.is_none() {
                if let Ok(value) = input.try_parse(parse_authored::<BorderSideWidth>) {
                    width = Some(value);
                    consumed = true;
                    continue;
                }
            }
            if color.is_none() {
                if let Ok(value) = input.try_parse(parse_authored::<CssColor>) {
                    color = Some(value);
                    consumed = true;
                    continue;
                }
            }
            break;
        }

        if !consumed {
            return Err(input.new_custom_error(ParserError::InvalidValue));
        }
        Ok(Self {
            width: width.unwrap_or_default(),
            color: color.unwrap_or_default(),
        })
    }
}

pub(crate) fn expand_border_side_observable(
    source: &str,
) -> Result<BorderSideObservableExpansion, EngineError> {
    let rule = parse_entire::<GapRule>(source)?;
    Ok(BorderSideObservableExpansion {
        width: observable_authored_component("border-top-width", &rule.width)?,
        style: observable_authored_component("border-top-style", &rule.style)?,
        color: observable_authored_component("border-top-color", &rule.color)?,
    })
}

pub(crate) fn expand_text_stroke(source: &str) -> Result<TextStrokeExpansion, EngineError> {
    let stroke = parse_entire::<TextStroke>(source)?;
    Ok(TextStrokeExpansion {
        width: if stroke.width.authored {
            serialize_typed(&stroke.width.value)?
        } else {
            "initial".to_owned()
        },
        width_observable: observable_authored_component(
            "-webkit-text-stroke-width",
            &stroke.width,
        )?,
        color: if stroke.color.authored {
            serialize_typed(&stroke.color.value)?
        } else {
            "initial".to_owned()
        },
        color_observable: observable_authored_component(
            "-webkit-text-stroke-color",
            &stroke.color,
        )?,
    })
}

pub(crate) fn parse_gap_rule_longhand(
    property_name: &str,
    source: &str,
) -> Result<GapRuleLonghandValue, EngineError> {
    match gap_rule_component(property_name) {
        Some(GapRuleComponent::Width) => {
            return parse_entire(source).map(GapRuleLonghandValue::Width);
        }
        Some(GapRuleComponent::Style) => {
            return parse_entire(source).map(GapRuleLonghandValue::Style);
        }
        Some(GapRuleComponent::Color) => {
            return parse_entire(source).map(GapRuleLonghandValue::Color);
        }
        None => {}
    }
    Err(EngineError::Parse(format!(
        "unsupported gap-rule longhand: {property_name}"
    )))
}

pub(crate) fn expand_gap_rule(source: &str) -> Result<GapRuleExpansion, EngineError> {
    let rules = parse_entire::<RepeatedList<GapRule>>(source)?;
    Ok(GapRuleExpansion {
        width: serialize_repeated_list(&rules, |rule| serialize_typed(&rule.width.value))?,
        width_observable: serialize_repeated_list(&rules, |rule| {
            observable_component("border-top-width", &rule.width)
        })?,
        style: serialize_repeated_list(&rules, |rule| serialize_typed(&rule.style.value))?,
        style_observable: serialize_repeated_list(&rules, |rule| {
            observable_component("border-top-style", &rule.style)
        })?,
        color: serialize_repeated_list(&rules, |rule| serialize_typed(&rule.color.value))?,
        color_observable: serialize_repeated_list(&rules, |rule| {
            observable_component("border-top-color", &rule.color)
        })?,
    })
}

pub(crate) fn canonical_gap_rule_longhand(
    property_name: &str,
    source: &str,
) -> Result<String, EngineError> {
    parse_gap_rule_longhand(property_name, source)?.canonical_value()
}

pub(crate) fn synthesize_gap_rule(
    width: &str,
    style: &str,
    color: &str,
) -> Result<String, EngineError> {
    let widths = parse_entire::<RepeatedList<Authored<BorderSideWidth>>>(width)?;
    let styles = parse_entire::<RepeatedList<Authored<LineStyle>>>(style)?;
    let colors = parse_entire::<RepeatedList<Authored<CssColor>>>(color)?;
    synthesize_parallel_lists(&widths, &styles, &colors)
}

impl<'i, T> Parse<'i> for Authored<T>
where
    T: Parse<'i>,
{
    fn parse<'t>(input: &mut Parser<'i, 't>) -> Result<Self, ParseError<'i>> {
        parse_authored(input)
    }
}

fn synthesize_parallel_lists(
    widths: &RepeatedList<Authored<BorderSideWidth>>,
    styles: &RepeatedList<Authored<LineStyle>>,
    colors: &RepeatedList<Authored<CssColor>>,
) -> Result<String, EngineError> {
    if widths.items.len() != styles.items.len() || widths.items.len() != colors.items.len() {
        return Err(EngineError::Serialize(
            "gap-rule longhand list structures differ".to_owned(),
        ));
    }
    let mut items = Vec::with_capacity(widths.items.len());
    for ((width, style), color) in widths.items.iter().zip(&styles.items).zip(&colors.items) {
        match (width, style, color) {
            (
                RepeatedItem::Value(width),
                RepeatedItem::Value(style),
                RepeatedItem::Value(color),
            ) => items.push(synthesize_gap_rule_item(width, style, color)),
            (
                RepeatedItem::Repeat {
                    count: width_count,
                    values: widths,
                },
                RepeatedItem::Repeat {
                    count: style_count,
                    values: styles,
                },
                RepeatedItem::Repeat {
                    count: color_count,
                    values: colors,
                },
            ) if width_count == style_count
                && width_count == color_count
                && widths.len() == styles.len()
                && widths.len() == colors.len() =>
            {
                let count = match width_count {
                    RepeatCount::Auto => "auto".to_owned(),
                    RepeatCount::Integer(value) => value.to_string(),
                };
                let values = widths
                    .iter()
                    .zip(styles)
                    .zip(colors)
                    .map(|((width, style), color)| synthesize_gap_rule_item(width, style, color))
                    .collect::<Vec<_>>()
                    .join(", ");
                items.push(format!("repeat({count}, {values})"));
            }
            _ => {
                return Err(EngineError::Serialize(
                    "gap-rule longhand repeat structures differ".to_owned(),
                ));
            }
        }
    }
    Ok(items.join(", "))
}

fn synthesize_gap_rule_item(
    width: &Authored<BorderSideWidth>,
    style: &Authored<LineStyle>,
    color: &Authored<CssColor>,
) -> String {
    let mut components = Vec::with_capacity(3);
    if width.value != BorderSideWidth::default() {
        components.push(width.source.as_str());
    }
    if style.value != LineStyle::default() {
        components.push(style.source.as_str());
    }
    if color.value != CssColor::current_color() {
        components.push(color.source.as_str());
    }
    if components.is_empty() {
        return "medium".to_owned();
    }
    components.join(" ")
}

fn parse_entire<'i, T>(source: &'i str) -> Result<T, EngineError>
where
    T: Parse<'i>,
{
    let mut input = ParserInput::new(source);
    let mut parser = Parser::new(&mut input);
    parser
        .parse_entirely(T::parse)
        .map_err(|_| EngineError::Parse("invalid CSS gap-rule value".to_owned()))
}

fn serialize_repeated_list<T, F>(
    list: &RepeatedList<T>,
    serialize_value: F,
) -> Result<String, EngineError>
where
    F: Fn(&T) -> Result<String, EngineError>,
{
    let mut serialized = Vec::with_capacity(list.items.len());
    for item in &list.items {
        match item {
            RepeatedItem::Value(value) => serialized.push(serialize_value(value)?),
            RepeatedItem::Repeat { count, values } => {
                let count = match count {
                    RepeatCount::Auto => "auto".to_owned(),
                    RepeatCount::Integer(value) => value.to_string(),
                };
                let values = values
                    .iter()
                    .map(&serialize_value)
                    .collect::<Result<Vec<_>, _>>()?
                    .join(", ");
                serialized.push(format!("repeat({count}, {values})"));
            }
        }
    }
    Ok(serialized.join(", "))
}

fn serialize_typed<T: ToCss>(value: &T) -> Result<String, EngineError> {
    value
        .to_css_string(PrinterOptions::default())
        .map_err(|error| EngineError::Serialize(error.to_string()))
}

fn observable_component<T>(
    property_name: &str,
    value: &Authored<T>,
) -> Result<String, EngineError> {
    project_observable_value(property_name, &value.source).ok_or_else(|| {
        EngineError::Serialize(format!(
            "could not project {property_name} component: {}",
            value.source
        ))
    })
}

fn observable_authored_component<T>(
    property_name: &str,
    value: &Authored<T>,
) -> Result<String, EngineError> {
    if !value.authored {
        return Ok("initial".to_owned());
    }
    observable_component(property_name, value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expands_nested_gap_rule_lists_without_losing_authored_colors() {
        let expansion = expand_gap_rule(
            "1px, repeat(auto, red, 2px dotted), repeat(2, color-mix(in srgb, red, blue))",
        )
        .unwrap();
        assert_eq!(
            expansion.width_observable,
            "1px, repeat(auto, medium, 2px), repeat(2, medium)"
        );
        assert_eq!(
            expansion.style_observable,
            "none, repeat(auto, none, dotted), repeat(2, none)"
        );
        assert_eq!(
            expansion.color_observable,
            "currentcolor, repeat(auto, red, currentcolor), repeat(2, color-mix(in srgb, red, blue))"
        );
    }

    #[test]
    fn rejects_invalid_repeat_shapes() {
        for source in [
            "repeat(0, 1px)",
            "repeat(auto, 1px), repeat(auto, 2px)",
            ", repeat(auto, 1px), 2px",
            ", repeat(auto, 1px), 2px,",
            "repeat(1, repeat(1, 1px))",
            "repeat(auto)",
            "repeat(1,)",
        ] {
            assert!(expand_gap_rule(source).is_err(), "{source}");
        }
    }

    #[test]
    fn accepts_chromium_auto_repeat_with_an_optional_trailing_comma() {
        for (source, expected) in [
            ("repeat(auto, 1px)", "repeat(auto, 1px)"),
            ("repeat(auto, 1px),", "repeat(auto, 1px)"),
            ("1px, repeat(auto, 2px),", "1px, repeat(auto, 2px)"),
            ("repeat(auto, 1px), 2px", "repeat(auto, 1px), 2px"),
        ] {
            let value = parse_gap_rule_longhand("column-rule-width", source).unwrap();
            assert_eq!(value.canonical_value().unwrap(), expected, "{source}");
        }
    }

    #[test]
    fn parses_each_parallel_longhand_grammar() {
        let cases = [
            ("column-rule-width", "1px, repeat(auto, thick), 2px"),
            ("row-rule-style", "solid, repeat(auto, dotted), none"),
            ("column-rule-color", "red, repeat(auto, blue), currentcolor"),
        ];
        for (property, source) in cases {
            let value = parse_gap_rule_longhand(property, source).unwrap();
            assert!(!value.canonical_value().unwrap().is_empty());
        }
    }

    #[test]
    fn text_stroke_tracks_authored_components_without_border_defaults() {
        for (source, width, color) in [
            ("red", "initial", "red"),
            ("medium", "medium", "initial"),
            ("red 1px", "1px", "red"),
            ("1px red", "1px", "red"),
        ] {
            let expansion = expand_text_stroke(source).unwrap();
            assert_eq!(expansion.width_observable, width, "{source} width");
            assert_eq!(expansion.color_observable, color, "{source} color");
        }
        assert!(expand_text_stroke("solid").is_err());
        assert!(expand_text_stroke("red blue").is_err());
    }

    #[test]
    fn gap_rule_component_mapping_rejects_similar_suffixes() {
        assert_eq!(
            gap_rule_component("column-rule-width"),
            Some(GapRuleComponent::Width)
        );
        assert_eq!(gap_rule_component("imaginary-rule-width"), None);
        assert_eq!(gap_rule_component("border-image-width"), None);
    }
}
