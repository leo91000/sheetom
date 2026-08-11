use cssparser::{Parser, ParserInput, Token};
use lightningcss::{
    properties::{
        animation::{
            AnimationAttachmentRange, AnimationRangeEnd, AnimationRangeStart, AnimationTimeline,
        },
        masking::ClipPath,
    },
    stylesheet::PrinterOptions,
    traits::{IntoOwned, Parse, ToCss, TrySign},
    values::{
        angle::Angle,
        calc::Calc,
        ident::{CustomIdent, DashedIdent},
        length::{Length, LengthPercentage, PreservedLengthPercentage},
        number::CSSNumber,
        percentage::Percentage,
        position::Position,
        string::CSSString,
        time::Time,
    },
};

use crate::EngineError;

type ParseError<'i> = cssparser::ParseError<'i, lightningcss::error::ParserError<'i>>;

#[derive(Clone, Debug, PartialEq)]
pub enum BrowserLonghandValue {
    Keyword(&'static str),
    Length(Length),
    LengthPercentage(LengthPercentage),
    AutoLength(Option<Length>),
    ColumnCount(ColumnCountValue),
    ContainIntrinsic(ContainIntrinsicValue),
    DashedIdentList(DashedIdentListValue),
    TimeOrNormal(Option<Time>),
    ViewTimelineInset(Vec<ViewTimelineInsetValue>),
    CornerShape(CornerShapeValue),
    ColorScheme(ColorSchemeValue),
    FontFeatureSettings(Option<Vec<FontFeatureSetting>>),
    FontLanguageOverride(Option<CSSString<'static>>),
    FontSizeAdjust(FontSizeAdjustValue),
    FontVariantAlternates(FontVariantAlternatesValue),
    FontVariantKeywords(Vec<&'static str>),
    FontVariationSettings(Option<Vec<FontVariationSetting>>),
    PositionTryFallbacks(Option<Vec<PositionTryFallback>>),
    TextBoxEdge(TextBoxEdgeValue),
    TimelineRangeStart(Vec<TimelineRangeStartValue>),
    TimelineRangeEnd(Vec<TimelineRangeEndValue>),
    TimelineTriggerName(Vec<Option<DashedIdent<'static>>>),
    TimelineTriggerSource(Vec<AnimationTimeline<'static>>),
    KeywordList(Vec<&'static str>),
    AnimationIterationCount(Vec<AnimationIterationCountValue>),
    OffsetPath(OffsetPathValue),
    AxisPosition(LengthPercentage),
    Position(Position),
    AutoLengthPercentage(Option<LengthPercentage>),
    Containment(Vec<&'static str>),
    TextDecorationLine(Vec<&'static str>),
    ScrollSnapAlign {
        block: &'static str,
        inline: Option<&'static str>,
    },
    ScrollSnapType {
        axis: &'static str,
        strictness: Option<&'static str>,
    },
    ScrollbarGutter {
        both_edges: bool,
    },
    OverflowClipMargin {
        visual_box: &'static str,
        length: Option<Length>,
        calculation: bool,
    },
    TextUnderlinePosition {
        primary: Option<&'static str>,
        side: Option<&'static str>,
    },
    TouchAction(Vec<&'static str>),
    StringOrKeyword(StringOrKeywordValue),
    KeywordOrDashedIdent(KeywordOrDashedIdentValue),
    PositionVisibility {
        anchors_visible: bool,
        no_overflow: bool,
    },
    CounterList(Option<Vec<CounterEntry>>),
    CustomIdent(CustomIdent<'static>),
    Quotes(QuotesValue),
    PaintOrder(Vec<&'static str>),
    WillChange(Option<Vec<CustomIdent<'static>>>),
    Clip(ClipValue),
    DynamicRangeLimit(DynamicRangeLimitValue),
    AnimationTrigger(Vec<AnimationTriggerValue>),
    PositionArea(PositionAreaValue),
}

#[derive(Clone, Debug, PartialEq)]
pub struct ColumnCountValue {
    count: Option<Calc<PreservedLengthPercentage>>,
    wrap_calc: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ContainIntrinsicValue {
    None,
    Length { auto: bool, value: Option<Length> },
}

#[derive(Clone, Debug, PartialEq)]
pub enum ViewTimelineInsetComponent {
    Auto,
    LengthPercentage(LengthPercentage),
}

#[derive(Clone, Debug, PartialEq)]
pub struct ViewTimelineInsetValue {
    values: Vec<ViewTimelineInsetComponent>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum CornerShapeValue {
    Keyword(&'static str),
    Superellipse(CSSNumber),
}

#[derive(Clone, Debug, PartialEq)]
pub enum ColorSchemeValue {
    Normal,
    Schemes {
        values: Vec<CustomIdent<'static>>,
        only: bool,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct FontFeatureSetting {
    tag: CSSString<'static>,
    value: i32,
}

#[derive(Clone, Debug, PartialEq)]
pub enum FontSizeAdjustValue {
    None,
    Value {
        metric: Option<&'static str>,
        value: FontSizeAdjustNumber,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub enum FontSizeAdjustNumber {
    FromFont,
    Number {
        value: Calc<PreservedLengthPercentage>,
        wrap_calc: bool,
    },
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct FontVariantAlternatesValue {
    historical_forms: bool,
    functions: Vec<FontAlternateFunction>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FontAlternateFunction {
    name: &'static str,
    values: Vec<CustomIdent<'static>>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FontVariationSetting {
    tag: CSSString<'static>,
    value: CSSNumber,
    calculation: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PositionTryFallback {
    name: Option<DashedIdent<'static>>,
    area: Option<PositionAreaValue>,
    tactics: Vec<&'static str>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum TextBoxEdgeValue {
    Auto,
    Edges {
        over: &'static str,
        under: Option<&'static str>,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub enum TimelineRangeStartValue {
    Auto,
    Range(AnimationRangeStart),
}

#[derive(Clone, Debug, PartialEq)]
pub enum TimelineRangeEndValue {
    Auto,
    Range(AnimationRangeEnd),
}

#[derive(Clone, Debug, PartialEq)]
pub enum AnimationIterationCountValue {
    Infinite,
    Number(Calc<PreservedLengthPercentage>),
}

#[derive(Clone, Debug, PartialEq)]
pub enum OffsetPathValue {
    ClipPath(ClipPath<'static>),
    Path {
        fill_rule: Option<&'static str>,
        data: CSSString<'static>,
    },
    Ray(Angle),
}

#[derive(Clone, Debug, PartialEq)]
pub enum StringOrKeywordValue {
    Keyword(&'static str),
    String(CSSString<'static>),
}

#[derive(Clone, Debug, PartialEq)]
pub enum DashedIdentListValue {
    None,
    All,
    Names(Vec<DashedIdent<'static>>),
}

#[derive(Clone, Debug, PartialEq)]
pub enum KeywordOrDashedIdentValue {
    Keyword(&'static str),
    Name(DashedIdent<'static>),
}

#[derive(Clone, Debug, PartialEq)]
pub struct CounterEntry {
    name: CustomIdent<'static>,
    value: i32,
}

#[derive(Clone, Debug, PartialEq)]
pub enum QuotesValue {
    Auto,
    None,
    Pairs(Vec<(CSSString<'static>, CSSString<'static>)>),
}

#[derive(Clone, Debug, PartialEq)]
pub enum ClipValue {
    Auto,
    Rect([ClipComponent; 4]),
}

#[derive(Clone, Debug, PartialEq)]
pub enum ClipComponent {
    Auto,
    Length {
        value: Length,
        authored_function: Option<String>,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub enum DynamicRangeLimitValue {
    Keyword(&'static str),
    Mix(Vec<DynamicRangeLimitMixEntry>),
}

#[derive(Clone, Debug, PartialEq)]
pub struct DynamicRangeLimitMixEntry {
    limit: DynamicRangeLimitValue,
    percentage: Calc<Percentage>,
    authored_calculation: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub enum AnimationTriggerValue {
    None,
    Attachment {
        name: DashedIdent<'static>,
        enter: &'static str,
        exit: Option<&'static str>,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PositionAreaAxis {
    General,
    Horizontal,
    Vertical,
    Block,
    Inline,
    SelfBlock,
    SelfInline,
    StartEnd,
    SelfStartEnd,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct PositionAreaKeyword {
    value: &'static str,
    axis: PositionAreaAxis,
}

#[derive(Clone, Debug, PartialEq)]
pub enum PositionAreaValue {
    None,
    Area {
        first: &'static str,
        second: Option<&'static str>,
    },
}

impl BrowserLonghandValue {
    pub(crate) fn canonical_value(&self) -> Result<String, EngineError> {
        match self {
            BrowserLonghandValue::Keyword(value) => Ok((*value).to_owned()),
            BrowserLonghandValue::Length(value) => serialize_zero_length(value),
            BrowserLonghandValue::LengthPercentage(value) => {
                serialize_zero_length_percentage(value)
            }
            BrowserLonghandValue::AutoLength(None) => Ok("auto".to_owned()),
            BrowserLonghandValue::AutoLength(Some(value)) => serialize_zero_length(value),
            BrowserLonghandValue::ColumnCount(value) => value.canonical_value(),
            BrowserLonghandValue::ContainIntrinsic(value) => value.canonical_value(),
            BrowserLonghandValue::DashedIdentList(DashedIdentListValue::None) => {
                Ok("none".to_owned())
            }
            BrowserLonghandValue::DashedIdentList(DashedIdentListValue::All) => {
                Ok("all".to_owned())
            }
            BrowserLonghandValue::DashedIdentList(DashedIdentListValue::Names(values)) => {
                serialize_comma_separated(values)
            }
            BrowserLonghandValue::TimeOrNormal(None) => Ok("normal".to_owned()),
            BrowserLonghandValue::TimeOrNormal(Some(value)) => serialize_typed(value),
            BrowserLonghandValue::ViewTimelineInset(values) => {
                let mut entries = Vec::with_capacity(values.len());
                for value in values {
                    let mut components = Vec::with_capacity(value.values.len());
                    for component in &value.values {
                        components.push(match component {
                            ViewTimelineInsetComponent::Auto => "auto".to_owned(),
                            ViewTimelineInsetComponent::LengthPercentage(value) => {
                                serialize_typed(value)?
                            }
                        });
                    }
                    entries.push(components.join(" "));
                }
                Ok(entries.join(", "))
            }
            BrowserLonghandValue::CornerShape(value) => value.canonical_value(),
            BrowserLonghandValue::ColorScheme(ColorSchemeValue::Normal) => Ok("normal".to_owned()),
            BrowserLonghandValue::ColorScheme(ColorSchemeValue::Schemes { values, only }) => {
                let mut output = serialize_space_separated(values)?;
                if *only {
                    output.push_str(" only");
                }
                Ok(output)
            }
            BrowserLonghandValue::FontFeatureSettings(None) => Ok("normal".to_owned()),
            BrowserLonghandValue::FontFeatureSettings(Some(values)) => {
                let mut output = Vec::with_capacity(values.len());
                for value in values {
                    let mut setting = serialize_typed(&value.tag)?;
                    if value.value != 1 {
                        setting.push(' ');
                        setting.push_str(&value.value.to_string());
                    }
                    output.push(setting);
                }
                Ok(output.join(", "))
            }
            BrowserLonghandValue::FontLanguageOverride(None) => Ok("normal".to_owned()),
            BrowserLonghandValue::FontLanguageOverride(Some(value)) => serialize_typed(value),
            BrowserLonghandValue::FontSizeAdjust(value) => value.canonical_value(),
            BrowserLonghandValue::FontVariantAlternates(value) => value.canonical_value(),
            BrowserLonghandValue::FontVariantKeywords(values) => Ok(values.join(" ")),
            BrowserLonghandValue::FontVariationSettings(None) => Ok("normal".to_owned()),
            BrowserLonghandValue::FontVariationSettings(Some(values)) => {
                let mut output = Vec::with_capacity(values.len());
                for value in values {
                    let mut setting = serialize_typed(&value.tag)?;
                    setting.push(' ');
                    let number = serialize_typed(&value.value)?;
                    if value.calculation {
                        setting.push_str(&format!("calc({number})"));
                    } else {
                        setting.push_str(&number);
                    }
                    output.push(setting);
                }
                Ok(output.join(", "))
            }
            BrowserLonghandValue::PositionTryFallbacks(None) => Ok("none".to_owned()),
            BrowserLonghandValue::PositionTryFallbacks(Some(values)) => {
                let mut output = Vec::with_capacity(values.len());
                for value in values {
                    let mut components = Vec::with_capacity(value.tactics.len() + 1);
                    if let Some(name) = &value.name {
                        components.push(serialize_typed(name)?);
                    }
                    if let Some(area) = &value.area {
                        components.push(area.canonical_value());
                    }
                    components.extend(value.tactics.iter().map(|value| (*value).to_owned()));
                    output.push(components.join(" "));
                }
                Ok(output.join(", "))
            }
            BrowserLonghandValue::TextBoxEdge(value) => value.canonical_value(),
            BrowserLonghandValue::TimelineRangeStart(values) => {
                serialize_timeline_range_starts(values)
            }
            BrowserLonghandValue::TimelineRangeEnd(values) => serialize_timeline_range_ends(values),
            BrowserLonghandValue::TimelineTriggerName(values) => values
                .iter()
                .map(|value| match value {
                    Some(value) => serialize_typed(value),
                    None => Ok("none".to_owned()),
                })
                .collect::<Result<Vec<_>, _>>()
                .map(|values| values.join(", ")),
            BrowserLonghandValue::TimelineTriggerSource(values) => {
                serialize_comma_separated(values)
            }
            BrowserLonghandValue::KeywordList(values) => Ok(values.join(", ")),
            BrowserLonghandValue::AnimationIterationCount(values) => {
                let mut output = Vec::with_capacity(values.len());
                for value in values {
                    output.push(match value {
                        AnimationIterationCountValue::Infinite => "infinite".to_owned(),
                        AnimationIterationCountValue::Number(value) => serialize_typed(value)?,
                    });
                }
                Ok(output.join(", "))
            }
            BrowserLonghandValue::OffsetPath(value) => value.canonical_value(),
            BrowserLonghandValue::AxisPosition(value) => serialize_zero_length_percentage(value),
            BrowserLonghandValue::Position(value) => Ok(format!(
                "{} {}",
                serialize_position_component(&value.x)?,
                serialize_position_component(&value.y)?,
            )),
            BrowserLonghandValue::AutoLengthPercentage(None) => Ok("auto".to_owned()),
            BrowserLonghandValue::AutoLengthPercentage(Some(value)) => {
                serialize_zero_length_percentage(value)
            }
            BrowserLonghandValue::Containment(values)
            | BrowserLonghandValue::TextDecorationLine(values)
            | BrowserLonghandValue::TouchAction(values) => Ok(values.join(" ")),
            BrowserLonghandValue::ScrollSnapAlign { block, inline } => Ok(
                inline.map_or_else(|| (*block).to_owned(), |inline| format!("{block} {inline}"))
            ),
            BrowserLonghandValue::ScrollSnapType { axis, strictness } => Ok(strictness
                .map_or_else(
                    || (*axis).to_owned(),
                    |strictness| format!("{axis} {strictness}"),
                )),
            BrowserLonghandValue::ScrollbarGutter { both_edges } => Ok(if *both_edges {
                "stable both-edges".to_owned()
            } else {
                "stable".to_owned()
            }),
            BrowserLonghandValue::OverflowClipMargin {
                visual_box,
                length,
                calculation,
            } => {
                let mut output = if *visual_box == "padding-box" {
                    String::new()
                } else {
                    (*visual_box).to_owned()
                };
                if let Some(length) = length {
                    if !output.is_empty() {
                        output.push(' ');
                    }
                    let mut serialized = serialize_zero_length(length)?;
                    if *calculation && !serialized.starts_with("calc(") {
                        serialized = format!("calc({serialized})");
                    }
                    output.push_str(&serialized);
                }
                if output.is_empty() {
                    output.push_str("0px");
                }
                Ok(output)
            }
            BrowserLonghandValue::TextUnderlinePosition { primary, side } => {
                let mut values = Vec::with_capacity(2);
                if let Some(primary) = primary {
                    values.push(*primary);
                }
                if let Some(side) = side {
                    values.push(*side);
                }
                Ok(values.join(" "))
            }
            BrowserLonghandValue::StringOrKeyword(StringOrKeywordValue::Keyword(keyword)) => {
                Ok((*keyword).to_owned())
            }
            BrowserLonghandValue::StringOrKeyword(StringOrKeywordValue::String(value)) => {
                serialize_typed(value)
            }
            BrowserLonghandValue::KeywordOrDashedIdent(KeywordOrDashedIdentValue::Keyword(
                keyword,
            )) => Ok((*keyword).to_owned()),
            BrowserLonghandValue::KeywordOrDashedIdent(KeywordOrDashedIdentValue::Name(name)) => {
                serialize_typed(name)
            }
            BrowserLonghandValue::PositionVisibility {
                anchors_visible,
                no_overflow,
            } => {
                let mut values = Vec::with_capacity(2);
                if *anchors_visible {
                    values.push("anchors-visible");
                }
                if *no_overflow {
                    values.push("no-overflow");
                }
                if values.is_empty() {
                    values.push("always");
                }
                Ok(values.join(" "))
            }
            BrowserLonghandValue::CounterList(None) => Ok("none".to_owned()),
            BrowserLonghandValue::CounterList(Some(entries)) => {
                let mut values = Vec::with_capacity(entries.len() * 2);
                for entry in entries {
                    values.push(serialize_typed(&entry.name)?);
                    values.push(entry.value.to_string());
                }
                Ok(values.join(" "))
            }
            BrowserLonghandValue::CustomIdent(value) => serialize_typed(value),
            BrowserLonghandValue::Quotes(QuotesValue::Auto) => Ok("auto".to_owned()),
            BrowserLonghandValue::Quotes(QuotesValue::None) => Ok("none".to_owned()),
            BrowserLonghandValue::Quotes(QuotesValue::Pairs(pairs)) => {
                let mut values = Vec::with_capacity(pairs.len() * 2);
                for (open, close) in pairs {
                    values.push(serialize_typed(open)?);
                    values.push(serialize_typed(close)?);
                }
                Ok(values.join(" "))
            }
            BrowserLonghandValue::PaintOrder(values) => Ok(values.join(" ")),
            BrowserLonghandValue::WillChange(None) => Ok("auto".to_owned()),
            BrowserLonghandValue::WillChange(Some(values)) => serialize_comma_separated(values),
            BrowserLonghandValue::Clip(value) => value.canonical_value(),
            BrowserLonghandValue::DynamicRangeLimit(value) => value.canonical_value(),
            BrowserLonghandValue::AnimationTrigger(values) => {
                let mut serialized = Vec::with_capacity(values.len());
                for value in values {
                    serialized.push(match value {
                        AnimationTriggerValue::None => "none".to_owned(),
                        AnimationTriggerValue::Attachment { name, enter, exit } => {
                            let mut value = format!("{} {enter}", serialize_typed(name)?);
                            if let Some(exit) = exit {
                                value.push(' ');
                                value.push_str(exit);
                            }
                            value
                        }
                    });
                }
                Ok(serialized.join(", "))
            }
            BrowserLonghandValue::PositionArea(value) => Ok(value.canonical_value()),
        }
    }
}

impl ClipValue {
    fn canonical_value(&self) -> Result<String, EngineError> {
        match self {
            ClipValue::Auto => Ok("auto".to_owned()),
            ClipValue::Rect(components) => {
                let mut values = Vec::with_capacity(components.len());
                for component in components {
                    values.push(match component {
                        ClipComponent::Auto => "auto".to_owned(),
                        ClipComponent::Length {
                            value,
                            authored_function,
                        } => {
                            let mut serialized = serialize_zero_length(value)?;
                            if let Some(function) = authored_function {
                                if !serialized.contains('(') {
                                    serialized = format!("{function}({serialized})");
                                }
                            }
                            serialized
                        }
                    });
                }
                Ok(format!("rect({})", values.join(", ")))
            }
        }
    }
}

impl DynamicRangeLimitValue {
    fn canonical_value(&self) -> Result<String, EngineError> {
        match self {
            DynamicRangeLimitValue::Keyword(value) => Ok((*value).to_owned()),
            DynamicRangeLimitValue::Mix(entries) => {
                let mut values = Vec::with_capacity(entries.len());
                for entry in entries {
                    let mut percentage = serialize_typed(&entry.percentage)?;
                    if entry.authored_calculation && !percentage.contains('(') {
                        percentage = format!("calc({percentage})");
                    }
                    values.push(format!("{} {percentage}", entry.limit.canonical_value()?));
                }
                Ok(format!("dynamic-range-limit-mix({})", values.join(", ")))
            }
        }
    }
}

impl PositionAreaValue {
    fn canonical_value(&self) -> String {
        match self {
            PositionAreaValue::None => "none".to_owned(),
            PositionAreaValue::Area { first, second } => {
                second.map_or_else(|| (*first).to_owned(), |second| format!("{first} {second}"))
            }
        }
    }
}

impl OffsetPathValue {
    fn canonical_value(&self) -> Result<String, EngineError> {
        match self {
            OffsetPathValue::ClipPath(value) => serialize_typed(value),
            OffsetPathValue::Path { fill_rule, data } => {
                let mut output = "path(".to_owned();
                if let Some(fill_rule) = fill_rule {
                    output.push_str(fill_rule);
                    output.push_str(", ");
                }
                output.push_str(&serialize_typed(data)?);
                output.push(')');
                Ok(output)
            }
            OffsetPathValue::Ray(angle) => Ok(format!("ray({})", serialize_typed(angle)?)),
        }
    }
}

impl CornerShapeValue {
    fn canonical_value(&self) -> Result<String, EngineError> {
        match self {
            CornerShapeValue::Keyword(value) => Ok((*value).to_owned()),
            CornerShapeValue::Superellipse(value) => {
                Ok(format!("superellipse({})", serialize_typed(value)?))
            }
        }
    }
}

impl FontSizeAdjustValue {
    fn canonical_value(&self) -> Result<String, EngineError> {
        match self {
            FontSizeAdjustValue::None => Ok("none".to_owned()),
            FontSizeAdjustValue::Value { metric, value } => {
                let mut output = metric
                    .filter(|metric| *metric != "ex-height")
                    .map_or_else(String::new, |metric| format!("{metric} "));
                output.push_str(match value {
                    FontSizeAdjustNumber::FromFont => "from-font",
                    FontSizeAdjustNumber::Number { value, wrap_calc } => {
                        let needs_wrap =
                            *wrap_calc && matches!(value, Calc::Number(_) | Calc::Value(_));
                        let value = serialize_typed(value)?;
                        let value = if needs_wrap {
                            format!("calc({value})")
                        } else {
                            value
                        };
                        return Ok(format!("{output}{value}"));
                    }
                });
                Ok(output)
            }
        }
    }
}

impl FontVariantAlternatesValue {
    fn canonical_value(&self) -> Result<String, EngineError> {
        if !self.historical_forms && self.functions.is_empty() {
            return Ok("normal".to_owned());
        }
        let mut output =
            Vec::with_capacity(self.functions.len() + usize::from(self.historical_forms));
        for name in [
            "stylistic",
            "historical-forms",
            "styleset",
            "character-variant",
            "swash",
            "ornaments",
            "annotation",
        ] {
            if name == "historical-forms" {
                if self.historical_forms {
                    output.push(name.to_owned());
                }
                continue;
            }
            if let Some(function) = self.functions.iter().find(|function| function.name == name) {
                let values = serialize_comma_separated(&function.values)?;
                output.push(format!("{}({values})", function.name));
            }
        }
        Ok(output.join(" "))
    }
}

impl TextBoxEdgeValue {
    fn canonical_value(&self) -> Result<String, EngineError> {
        match self {
            TextBoxEdgeValue::Auto => Ok("auto".to_owned()),
            TextBoxEdgeValue::Edges {
                over: "text",
                under: Some("text"),
            } => Ok("text".to_owned()),
            TextBoxEdgeValue::Edges { over, under } => {
                let mut output = (*over).to_owned();
                if let Some(under) = under {
                    output.push(' ');
                    output.push_str(under);
                }
                Ok(output)
            }
        }
    }
}

impl ColumnCountValue {
    fn canonical_value(&self) -> Result<String, EngineError> {
        let Some(count) = &self.count else {
            return Ok("auto".to_owned());
        };
        let serialized = serialize_typed(&count)?;
        if self.wrap_calc && matches!(count, Calc::Number(_) | Calc::Value(_)) {
            return Ok(format!("calc({serialized})"));
        }
        Ok(serialized)
    }
}

impl ContainIntrinsicValue {
    fn canonical_value(&self) -> Result<String, EngineError> {
        match self {
            ContainIntrinsicValue::None => Ok("none".to_owned()),
            ContainIntrinsicValue::Length { auto, value } => {
                let mut output = if *auto {
                    "auto".to_owned()
                } else {
                    String::new()
                };
                if let Some(value) = value {
                    if !output.is_empty() {
                        output.push(' ');
                    }
                    output.push_str(&serialize_zero_length(value)?);
                } else if !output.is_empty() {
                    output.push_str(" none");
                }
                Ok(output)
            }
        }
    }
}

pub(crate) fn parse_browser_longhand(
    property_name: &str,
    source: &str,
) -> Result<Option<BrowserLonghandValue>, EngineError> {
    let value = match grammar(property_name) {
        Some(BrowserLonghandGrammar::Keyword(keywords)) => {
            parse_keyword(source, keywords).map(BrowserLonghandValue::Keyword)
        }
        Some(BrowserLonghandGrammar::KeywordAliases(keywords)) => {
            parse_keyword_alias(source, keywords).map(BrowserLonghandValue::Keyword)
        }
        Some(BrowserLonghandGrammar::Length { non_negative }) => parse_entire(source, |input| {
            let value = parse_strict_length(input)?;
            if non_negative && value.try_sign().is_some_and(|sign| sign < 0.0) {
                return Err(input.new_custom_error(lightningcss::error::ParserError::InvalidValue));
            }
            Ok(BrowserLonghandValue::Length(value))
        }),
        Some(BrowserLonghandGrammar::LengthPercentage { non_negative }) => {
            parse_entire(source, |input| {
                let value = parse_strict_length_percentage(input)?;
                if non_negative
                    && !matches!(value, LengthPercentage::Calc(_))
                    && value.try_sign().is_some_and(|sign| sign < 0.0)
                {
                    return Err(
                        input.new_custom_error(lightningcss::error::ParserError::InvalidValue)
                    );
                }
                Ok(BrowserLonghandValue::LengthPercentage(value))
            })
        }
        Some(BrowserLonghandGrammar::AutoLength) => parse_auto_length(source),
        Some(BrowserLonghandGrammar::ColumnCount) => parse_column_count(source),
        Some(BrowserLonghandGrammar::ContainIntrinsic) => parse_contain_intrinsic(source),
        Some(BrowserLonghandGrammar::DashedIdentList { allow_all }) => {
            parse_dashed_ident_list(source, allow_all)
        }
        Some(BrowserLonghandGrammar::TimeOrNormal) => parse_time_or_normal(source),
        Some(BrowserLonghandGrammar::ViewTimelineInset) => parse_view_timeline_inset(source),
        Some(BrowserLonghandGrammar::CornerShape) => parse_corner_shape(source),
        Some(BrowserLonghandGrammar::ColorScheme) => parse_color_scheme(source),
        Some(BrowserLonghandGrammar::FontFeatureSettings) => parse_font_feature_settings(source),
        Some(BrowserLonghandGrammar::FontLanguageOverride) => parse_font_language_override(source),
        Some(BrowserLonghandGrammar::FontSizeAdjust) => parse_font_size_adjust(source),
        Some(BrowserLonghandGrammar::FontVariantAlternates) => {
            parse_font_variant_alternates(source)
        }
        Some(BrowserLonghandGrammar::FontVariantKeywords) => {
            parse_font_variant_keywords(property_name, source)
        }
        Some(BrowserLonghandGrammar::FontVariationSettings) => {
            parse_font_variation_settings(source)
        }
        Some(BrowserLonghandGrammar::PositionTryFallbacks) => parse_position_try_fallbacks(source),
        Some(BrowserLonghandGrammar::TextBoxEdge) => parse_text_box_edge(source),
        Some(BrowserLonghandGrammar::TimelineRangeStart { auto }) => {
            parse_timeline_range_start(source, auto)
        }
        Some(BrowserLonghandGrammar::TimelineRangeEnd { auto }) => {
            parse_timeline_range_end(source, auto)
        }
        Some(BrowserLonghandGrammar::TimelineTriggerName) => parse_timeline_trigger_name(source),
        Some(BrowserLonghandGrammar::TimelineTriggerSource) => {
            parse_timeline_trigger_source(source)
        }
        Some(BrowserLonghandGrammar::KeywordList(keywords)) => parse_keyword_list(source, keywords),
        Some(BrowserLonghandGrammar::AnimationIterationCount) => {
            parse_animation_iteration_count(source)
        }
        Some(BrowserLonghandGrammar::OffsetPath) => parse_offset_path(source),
        Some(BrowserLonghandGrammar::AxisPosition { horizontal }) => {
            parse_axis_position(source, horizontal)
        }
        Some(BrowserLonghandGrammar::Position) => parse_position(source),
        Some(BrowserLonghandGrammar::AutoLengthPercentage) => parse_auto_length_percentage(source),
        Some(BrowserLonghandGrammar::Containment) => parse_containment(source),
        Some(BrowserLonghandGrammar::TextDecorationLine) => parse_text_decoration_line(source),
        Some(BrowserLonghandGrammar::ScrollSnapAlign) => parse_scroll_snap_align(source),
        Some(BrowserLonghandGrammar::ScrollSnapType) => parse_scroll_snap_type(source),
        Some(BrowserLonghandGrammar::ScrollbarGutter) => parse_scrollbar_gutter(source),
        Some(BrowserLonghandGrammar::OverflowClipMargin) => parse_overflow_clip_margin(source),
        Some(BrowserLonghandGrammar::TextUnderlinePosition) => {
            parse_text_underline_position(source)
        }
        Some(BrowserLonghandGrammar::TouchAction) => parse_touch_action(source),
        Some(BrowserLonghandGrammar::StringOrKeyword(keyword)) => {
            parse_string_or_keyword(source, keyword)
        }
        Some(BrowserLonghandGrammar::KeywordOrDashedIdent(keywords)) => {
            parse_keyword_or_dashed_ident(source, keywords)
        }
        Some(BrowserLonghandGrammar::PositionVisibility) => parse_position_visibility(source),
        Some(BrowserLonghandGrammar::CounterList { default_value }) => {
            parse_counter_list(source, default_value)
        }
        Some(BrowserLonghandGrammar::CustomIdent) => parse_custom_ident(source),
        Some(BrowserLonghandGrammar::Quotes) => parse_quotes(source),
        Some(BrowserLonghandGrammar::PaintOrder) => parse_paint_order(source),
        Some(BrowserLonghandGrammar::WillChange) => parse_will_change(source),
        Some(BrowserLonghandGrammar::Clip) => parse_clip(source),
        Some(BrowserLonghandGrammar::DynamicRangeLimit) => parse_dynamic_range_limit(source),
        Some(BrowserLonghandGrammar::AnimationTrigger) => parse_animation_trigger(source),
        Some(BrowserLonghandGrammar::PositionArea) => parse_position_area(source),
        None => return Ok(None),
    }?;
    Ok(Some(value))
}

/// Parses reviewed Chromium grammar branches that are newer than the vendored
/// property's otherwise complete grammar. Unlike the exclusive browser
/// longhand registry, these branches are attempted only after the standard
/// parser rejects the value.
pub(crate) fn parse_browser_fallback(
    property_name: &str,
    source: &str,
) -> Result<Option<BrowserLonghandValue>, EngineError> {
    let accepted: &'static [&'static str] = match property_name {
        "box-shadow" | "text-shadow" => &["none"],
        "font-palette" | "initial-letter" | "zoom" => &["normal"],
        "hyphenate-limit-chars" | "resize" | "rx" | "ry" => &["auto"],
        "-webkit-line-clamp" => &["none"],
        _ => return Ok(None),
    };
    parse_keyword(source, accepted)
        .map(BrowserLonghandValue::Keyword)
        .map(Some)
}

pub(crate) fn parse_timeline_range_pair(
    source: &str,
    omitted_end: &str,
) -> Result<(String, String), EngineError> {
    let (start, explicit_end) = parse_entire(source, |input| {
        let start = AnimationRangeStart::parse(input)?;
        let end = input.try_parse(AnimationRangeStart::parse).ok();
        Ok((start, end))
    })?;
    let start_css = serialize_typed(&start)?;
    let end = match explicit_end {
        Some(value) => AnimationRangeEnd(value.0),
        None => match &start.0 {
            AnimationAttachmentRange::TimelineRange { name, .. } => {
                AnimationRangeEnd(AnimationAttachmentRange::TimelineRange {
                    name: name.clone(),
                    offset: LengthPercentage::Percentage(Percentage(1.0)),
                })
            }
            _ => return Ok((start_css, omitted_end.to_owned())),
        },
    };
    Ok((start_css, serialize_typed(&end)?))
}

#[derive(Clone, Copy)]
enum BrowserLonghandGrammar {
    Keyword(&'static [&'static str]),
    KeywordAliases(&'static [(&'static str, &'static str)]),
    Length { non_negative: bool },
    LengthPercentage { non_negative: bool },
    AutoLength,
    ColumnCount,
    ContainIntrinsic,
    DashedIdentList { allow_all: bool },
    TimeOrNormal,
    ViewTimelineInset,
    CornerShape,
    ColorScheme,
    FontFeatureSettings,
    FontLanguageOverride,
    FontSizeAdjust,
    FontVariantAlternates,
    FontVariantKeywords,
    FontVariationSettings,
    PositionTryFallbacks,
    TextBoxEdge,
    TimelineRangeStart { auto: bool },
    TimelineRangeEnd { auto: bool },
    TimelineTriggerName,
    TimelineTriggerSource,
    KeywordList(&'static [&'static str]),
    AnimationIterationCount,
    OffsetPath,
    AxisPosition { horizontal: bool },
    Position,
    AutoLengthPercentage,
    Containment,
    TextDecorationLine,
    ScrollSnapAlign,
    ScrollSnapType,
    ScrollbarGutter,
    OverflowClipMargin,
    TextUnderlinePosition,
    TouchAction,
    StringOrKeyword(&'static str),
    KeywordOrDashedIdent(&'static [&'static str]),
    PositionVisibility,
    CounterList { default_value: i32 },
    CustomIdent,
    Quotes,
    PaintOrder,
    WillChange,
    Clip,
    DynamicRangeLimit,
    AnimationTrigger,
    PositionArea,
}

macro_rules! define_browser_longhand_registry {
    ($( $grammar:expr => [$( $property:literal ),+ $(,)?] ),+ $(,)?) => {
        fn grammar(property_name: &str) -> Option<BrowserLonghandGrammar> {
            match property_name {
                $( $( $property )|+ => Some($grammar), )+
                _ => None,
            }
        }

        pub(crate) fn has_browser_longhand_grammar(property_name: &str) -> bool {
            grammar(property_name).is_some()
        }

        #[cfg(test)]
        const REGISTERED_BROWSER_LONGHANDS: &[&str] = &[
            $( $( $property, )+ )+
        ];
    };
}

define_browser_longhand_registry! {
    BrowserLonghandGrammar::Length { non_negative: true } => [
        "-webkit-border-horizontal-spacing",
        "-webkit-border-vertical-spacing",
    ],
    BrowserLonghandGrammar::LengthPercentage { non_negative: false } => [
        "column-rule-inset-cap-end",
        "column-rule-inset-cap-start",
        "column-rule-inset-junction-end",
        "column-rule-inset-junction-start",
        "row-rule-inset-cap-end",
        "row-rule-inset-cap-start",
        "row-rule-inset-junction-end",
        "row-rule-inset-junction-start",
    ],
    BrowserLonghandGrammar::Length { non_negative: false } => [
        "-webkit-transform-origin-z",
        "outline-offset",
    ],
    BrowserLonghandGrammar::LengthPercentage { non_negative: false } => ["offset-distance"],
    BrowserLonghandGrammar::LengthPercentage { non_negative: true } => ["shape-margin"],
    BrowserLonghandGrammar::AutoLength => ["column-height", "column-width"],
    BrowserLonghandGrammar::ColumnCount => ["column-count"],
    BrowserLonghandGrammar::TimeOrNormal => ["interest-delay-end", "interest-delay-start"],
    BrowserLonghandGrammar::DashedIdentList { allow_all: false } => [
        "anchor-name",
        "scroll-timeline-name",
        "timeline-scope",
        "view-timeline-name",
    ],
    BrowserLonghandGrammar::DashedIdentList { allow_all: true } => [
        "anchor-scope",
        "trigger-scope",
    ],
    BrowserLonghandGrammar::ViewTimelineInset => ["view-timeline-inset"],
    BrowserLonghandGrammar::CornerShape => [
        "corner-bottom-left-shape",
        "corner-bottom-right-shape",
        "corner-end-end-shape",
        "corner-end-start-shape",
        "corner-start-end-shape",
        "corner-start-start-shape",
        "corner-top-left-shape",
        "corner-top-right-shape",
    ],
    BrowserLonghandGrammar::ColorScheme => ["color-scheme"],
    BrowserLonghandGrammar::FontFeatureSettings => ["font-feature-settings"],
    BrowserLonghandGrammar::FontLanguageOverride => ["font-language-override"],
    BrowserLonghandGrammar::FontSizeAdjust => ["font-size-adjust"],
    BrowserLonghandGrammar::FontVariantAlternates => ["font-variant-alternates"],
    BrowserLonghandGrammar::FontVariantKeywords => [
        "font-variant-east-asian",
        "font-variant-ligatures",
        "font-variant-numeric",
    ],
    BrowserLonghandGrammar::FontVariationSettings => ["font-variation-settings"],
    BrowserLonghandGrammar::PositionTryFallbacks => ["position-try-fallbacks"],
    BrowserLonghandGrammar::TextBoxEdge => ["text-box-edge"],
    BrowserLonghandGrammar::TimelineRangeStart { auto: false } => [
        "timeline-trigger-activation-range-start",
    ],
    BrowserLonghandGrammar::TimelineRangeStart { auto: true } => [
        "timeline-trigger-active-range-start",
    ],
    BrowserLonghandGrammar::TimelineRangeEnd { auto: false } => [
        "timeline-trigger-activation-range-end",
    ],
    BrowserLonghandGrammar::TimelineRangeEnd { auto: true } => [
        "timeline-trigger-active-range-end",
    ],
    BrowserLonghandGrammar::TimelineTriggerName => ["timeline-trigger-name"],
    BrowserLonghandGrammar::TimelineTriggerSource => ["timeline-trigger-source"],
    BrowserLonghandGrammar::AnimationIterationCount => [
        "-webkit-animation-iteration-count",
        "animation-iteration-count",
    ],
    BrowserLonghandGrammar::OffsetPath => ["offset-path"],
    BrowserLonghandGrammar::AxisPosition { horizontal: true } => [
        "-webkit-perspective-origin-x",
        "-webkit-transform-origin-x",
    ],
    BrowserLonghandGrammar::AxisPosition { horizontal: false } => [
        "-webkit-perspective-origin-y",
        "-webkit-transform-origin-y",
    ],
    BrowserLonghandGrammar::Position => ["object-position"],
    BrowserLonghandGrammar::AutoLengthPercentage => ["text-underline-offset"],
    BrowserLonghandGrammar::ContainIntrinsic => [
        "contain-intrinsic-block-size",
        "contain-intrinsic-height",
        "contain-intrinsic-inline-size",
        "contain-intrinsic-width",
    ],
    BrowserLonghandGrammar::Containment => ["contain"],
    BrowserLonghandGrammar::TextDecorationLine => ["-webkit-text-decorations-in-effect"],
    BrowserLonghandGrammar::ScrollSnapAlign => ["scroll-snap-align"],
    BrowserLonghandGrammar::ScrollSnapType => ["scroll-snap-type"],
    BrowserLonghandGrammar::ScrollbarGutter => ["scrollbar-gutter"],
    BrowserLonghandGrammar::OverflowClipMargin => ["overflow-clip-margin"],
    BrowserLonghandGrammar::TextUnderlinePosition => ["text-underline-position"],
    BrowserLonghandGrammar::TouchAction => ["touch-action"],
    BrowserLonghandGrammar::StringOrKeyword("auto") => [
        "-webkit-locale",
        "hyphenate-character",
    ],
    BrowserLonghandGrammar::KeywordOrDashedIdent(&["auto", "none", "normal"]) => [
        "position-anchor",
    ],
    BrowserLonghandGrammar::PositionVisibility => ["position-visibility"],
    BrowserLonghandGrammar::CounterList { default_value: 1 } => ["counter-increment"],
    BrowserLonghandGrammar::CounterList { default_value: 0 } => [
        "counter-reset",
        "counter-set",
    ],
    BrowserLonghandGrammar::CustomIdent => ["page"],
    BrowserLonghandGrammar::Quotes => ["quotes"],
    BrowserLonghandGrammar::PaintOrder => ["paint-order"],
    BrowserLonghandGrammar::WillChange => ["will-change"],
    BrowserLonghandGrammar::Clip => ["clip"],
    BrowserLonghandGrammar::DynamicRangeLimit => ["dynamic-range-limit"],
    BrowserLonghandGrammar::AnimationTrigger => ["animation-trigger"],
    BrowserLonghandGrammar::PositionArea => ["position-area"],
    BrowserLonghandGrammar::KeywordAliases(&[
        ("auto", "auto"),
        ("none", "spaces"),
        ("spaces", "spaces"),
    ]) => ["ruby-overhang"],
    BrowserLonghandGrammar::KeywordList(&["block", "inline", "x", "y"]) => [
        "scroll-timeline-axis",
        "view-timeline-axis",
    ],
    BrowserLonghandGrammar::KeywordList(&["normal", "allow-discrete"]) => [
        "transition-behavior",
    ],
    BrowserLonghandGrammar::Keyword(&["normal", "none"]) => [
        "column-rule-break",
        "row-rule-break",
    ],
    BrowserLonghandGrammar::Keyword(&["normal", "all"]) => [
        "column-rule-visibility-items",
        "row-rule-visibility-items",
    ],
    BrowserLonghandGrammar::Keyword(&["auto", "wrap", "nowrap"]) => ["column-wrap"],
    BrowserLonghandGrammar::Keyword(&["auto", "normal", "none"]) => ["font-kerning"],
    BrowserLonghandGrammar::Keyword(&["auto", "none"]) => [
        "font-optical-sizing",
        "font-synthesis-small-caps",
        "font-synthesis-style",
        "font-synthesis-weight",
    ],
    BrowserLonghandGrammar::Keyword(&["normal", "text", "emoji", "unicode"]) => [
        "font-variant-emoji",
    ],
    BrowserLonghandGrammar::Keyword(&["normal", "sub", "super"]) => [
        "font-variant-position",
    ],
    BrowserLonghandGrammar::Keyword(&["auto", "contain", "none"]) => [
        "overscroll-behavior-x",
        "overscroll-behavior-y",
    ],
    BrowserLonghandGrammar::Keyword(&[
        "normal",
        "most-width",
        "most-height",
        "most-block-size",
        "most-inline-size",
    ]) => ["position-try-order"],
    BrowserLonghandGrammar::Keyword(&["none", "trim-start", "trim-end", "trim-both"]) => [
        "text-box-trim",
    ],
    BrowserLonghandGrammar::Keyword(&["wrap", "nowrap"]) => ["text-wrap-mode"],
    BrowserLonghandGrammar::Keyword(&["auto", "balance", "stable", "pretty"]) => [
        "text-wrap-style",
    ],
    BrowserLonghandGrammar::Keyword(&[
        "collapse",
        "preserve",
        "preserve-breaks",
        "break-spaces",
    ]) => ["white-space-collapse"],
    BrowserLonghandGrammar::Keyword(&[
        "auto",
        "none",
        "antialiased",
        "subpixel-antialiased",
    ]) => ["-webkit-font-smoothing"],
    BrowserLonghandGrammar::Keyword(&[
        "auto",
        "loose",
        "normal",
        "strict",
        "after-white-space",
    ]) => ["-webkit-line-break"],
    BrowserLonghandGrammar::Keyword(&["logical", "visual"]) => ["-webkit-rtl-ordering"],
    BrowserLonghandGrammar::Keyword(&["before", "after"]) => ["-webkit-ruby-position"],
    BrowserLonghandGrammar::Keyword(&["none", "horizontal"]) => ["-webkit-text-combine"],
    BrowserLonghandGrammar::Keyword(&[
        "sideways",
        "upright",
        "sideways-right",
        "vertical-right",
    ]) => ["-webkit-text-orientation"],
    BrowserLonghandGrammar::Keyword(&["none", "disc", "circle", "square"]) => [
        "-webkit-text-security",
    ],
    BrowserLonghandGrammar::Keyword(&["auto", "none", "element"]) => ["-webkit-user-drag"],
    BrowserLonghandGrammar::Keyword(&[
        "read-only",
        "read-write",
        "read-write-plaintext-only",
    ]) => ["-webkit-user-modify"],
    BrowserLonghandGrammar::Keyword(&["horizontal-tb", "vertical-rl", "vertical-lr"]) => [
        "-webkit-writing-mode",
    ],
    BrowserLonghandGrammar::Keyword(&[
        "auto",
        "baseline",
        "alphabetic",
        "ideographic",
        "middle",
        "central",
        "mathematical",
        "before-edge",
        "text-before-edge",
        "after-edge",
        "text-after-edge",
        "hanging",
    ]) => ["alignment-baseline"],
    BrowserLonghandGrammar::Keyword(&["none", "drag", "no-drag"]) => ["app-region"],
    BrowserLonghandGrammar::KeywordList(&[
        "normal",
        "multiply",
        "screen",
        "overlay",
        "darken",
        "lighten",
        "color-dodge",
        "color-burn",
        "hard-light",
        "soft-light",
        "difference",
        "exclusion",
        "hue",
        "saturation",
        "color",
        "luminosity",
    ]) => ["background-blend-mode"],
    BrowserLonghandGrammar::Keyword(&["auto", "first", "last"]) => ["baseline-source"],
    BrowserLonghandGrammar::Keyword(&["separate", "collapse"]) => ["border-collapse"],
    BrowserLonghandGrammar::Keyword(&[
        "auto",
        "avoid",
        "avoid-column",
        "avoid-page",
        "column",
        "left",
        "page",
        "recto",
        "right",
        "verso",
    ]) => ["break-after", "break-before"],
    BrowserLonghandGrammar::Keyword(&["auto", "avoid", "avoid-column", "avoid-page"]) => [
        "break-inside",
    ],
    BrowserLonghandGrammar::Keyword(&["auto", "dynamic", "static"]) => ["buffered-rendering"],
    BrowserLonghandGrammar::Keyword(&["top", "bottom"]) => ["caption-side"],
    BrowserLonghandGrammar::Keyword(&["auto", "manual"]) => ["caret-animation"],
    BrowserLonghandGrammar::Keyword(&[
        "none",
        "left",
        "right",
        "both",
        "inline-start",
        "inline-end",
    ]) => ["clear"],
    BrowserLonghandGrammar::Keyword(&["balance", "auto"]) => ["column-fill"],
    BrowserLonghandGrammar::Keyword(&["none", "all"]) => ["column-span"],
    BrowserLonghandGrammar::Keyword(&["visible", "auto", "hidden"]) => ["content-visibility"],
    BrowserLonghandGrammar::Keyword(&[
        "auto",
        "alphabetic",
        "ideographic",
        "middle",
        "central",
        "mathematical",
        "hanging",
        "use-script",
        "no-change",
        "reset-size",
        "text-after-edge",
        "text-before-edge",
    ]) => ["dominant-baseline"],
    BrowserLonghandGrammar::Keyword(&["show", "hide"]) => ["empty-cells"],
    BrowserLonghandGrammar::Keyword(&["fixed", "content"]) => ["field-sizing"],
    BrowserLonghandGrammar::Keyword(&[
        "none",
        "left",
        "right",
        "inline-start",
        "inline-end",
    ]) => ["float"],
    BrowserLonghandGrammar::Keyword(&["auto", "none", "preserve-parent-color"]) => [
        "forced-color-adjust",
    ],
    BrowserLonghandGrammar::Keyword(&["none", "from-image"]) => ["image-orientation"],
    BrowserLonghandGrammar::Keyword(&["auto", "inert"]) => ["interactivity"],
    BrowserLonghandGrammar::Keyword(&["numeric-only", "allow-keywords"]) => [
        "interpolate-size",
    ],
    BrowserLonghandGrammar::Keyword(&["auto", "isolate"]) => ["isolation"],
    BrowserLonghandGrammar::Keyword(&["normal", "compact"]) => ["math-shift", "math-style"],
    BrowserLonghandGrammar::Keyword(&["fill", "contain", "cover", "none", "scale-down"]) => [
        "object-fit",
    ],
    BrowserLonghandGrammar::Keyword(&["visible", "none", "auto"]) => ["overflow-anchor"],
    BrowserLonghandGrammar::Keyword(&["visible", "hidden", "clip", "scroll", "auto"]) => [
        "overflow-block",
        "overflow-inline",
    ],
    BrowserLonghandGrammar::Keyword(&["none", "auto"]) => ["overlay"],
    BrowserLonghandGrammar::Keyword(&["auto", "contain", "none"]) => [
        "overscroll-behavior-block",
        "overscroll-behavior-inline",
    ],
    BrowserLonghandGrammar::Keyword(&["none", "clamp", "add"]) => ["page-margin-safety"],
    BrowserLonghandGrammar::Keyword(&[
        "none",
        "auto",
        "stroke",
        "fill",
        "painted",
        "visible",
        "visiblestroke",
        "visiblefill",
        "visiblepainted",
        "bounding-box",
        "all",
    ]) => ["pointer-events"],
    BrowserLonghandGrammar::Keyword(&[
        "normal",
        "flex-visual",
        "flex-flow",
        "grid-rows",
        "grid-columns",
        "grid-order",
        "source-order",
    ]) => ["reading-flow"],
    BrowserLonghandGrammar::Keyword(&["space-around", "start", "center", "space-between"]) => [
        "ruby-align",
    ],
    BrowserLonghandGrammar::Keyword(&["over", "under"]) => ["ruby-position"],
    BrowserLonghandGrammar::Keyword(&["row-over-column", "column-over-row"]) => ["rule-overlap"],
    BrowserLonghandGrammar::Keyword(&["auto", "smooth"]) => ["scroll-behavior"],
    BrowserLonghandGrammar::Keyword(&["none", "nearest"]) => ["scroll-initial-target"],
    BrowserLonghandGrammar::Keyword(&["none", "after", "before"]) => ["scroll-marker-group"],
    BrowserLonghandGrammar::Keyword(&["normal", "always"]) => ["scroll-snap-stop"],
    BrowserLonghandGrammar::Keyword(&["none", "auto"]) => ["scroll-target-group"],
    BrowserLonghandGrammar::Keyword(&["auto", "thin", "none"]) => ["scrollbar-width"],
    BrowserLonghandGrammar::Keyword(&[
        "none",
        "normal",
        "spell-out",
        "digits",
        "literal-punctuation",
        "no-punctuation",
    ]) => ["speak"],
    BrowserLonghandGrammar::Keyword(&["auto", "fixed"]) => ["table-layout"],
    BrowserLonghandGrammar::Keyword(&["none", "shrink", "grow"]) => ["text-fit"],
    BrowserLonghandGrammar::Keyword(&["start", "middle", "end"]) => ["text-anchor"],
    BrowserLonghandGrammar::Keyword(&["none", "all"]) => ["text-combine-upright"],
    BrowserLonghandGrammar::Keyword(&[
        "sideways",
        "mixed",
        "upright",
        "sideways-right",
    ]) => ["text-orientation"],
    BrowserLonghandGrammar::Keyword(&["normal", "space-all", "space-first", "trim-start"]) => [
        "text-spacing-trim",
    ],
    BrowserLonghandGrammar::Keyword(&["none", "non-scaling-stroke"]) => ["vector-effect"],
    BrowserLonghandGrammar::Keyword(&["normal", "no-autospace"]) => ["text-autospace"],
    BrowserLonghandGrammar::Keyword(&["none", "all"]) => ["view-transition-scope"],
    BrowserLonghandGrammar::Keyword(&["upright", "rotate-left", "rotate-right"]) => [
        "page-orientation",
    ],
    BrowserLonghandGrammar::Keyword(&[
        "horizontal-tb",
        "vertical-rl",
        "vertical-lr",
        "sideways-rl",
        "sideways-lr",
        "lr",
        "rl",
        "tb",
        "lr-tb",
        "rl-tb",
        "tb-rl",
    ]) => ["writing-mode"],
}

fn parse_keyword(
    source: &str,
    accepted: &'static [&'static str],
) -> Result<&'static str, EngineError> {
    parse_entire(source, |input| {
        let location = input.current_source_location();
        let identifier = input.expect_ident_cloned()?;
        accepted
            .iter()
            .copied()
            .find(|candidate| identifier.eq_ignore_ascii_case(candidate))
            .ok_or_else(|| {
                location.new_custom_error(lightningcss::error::ParserError::InvalidValue)
            })
    })
}

fn parse_keyword_alias(
    source: &str,
    accepted: &'static [(&'static str, &'static str)],
) -> Result<&'static str, EngineError> {
    parse_entire(source, |input| {
        let location = input.current_source_location();
        let identifier = input.expect_ident_cloned()?;
        accepted
            .iter()
            .find_map(|(candidate, canonical)| {
                identifier
                    .eq_ignore_ascii_case(candidate)
                    .then_some(*canonical)
            })
            .ok_or_else(|| {
                location.new_custom_error(lightningcss::error::ParserError::InvalidValue)
            })
    })
}

fn parse_keyword_list(
    source: &str,
    accepted: &'static [&'static str],
) -> Result<BrowserLonghandValue, EngineError> {
    parse_entire(source, |input| {
        let values = input.parse_comma_separated(|input| {
            let location = input.current_source_location();
            let identifier = input.expect_ident_cloned()?;
            accepted
                .iter()
                .copied()
                .find(|candidate| identifier.eq_ignore_ascii_case(candidate))
                .ok_or_else(|| {
                    location.new_custom_error(lightningcss::error::ParserError::InvalidValue)
                })
        })?;
        Ok(BrowserLonghandValue::KeywordList(values))
    })
}

fn parse_axis_position(
    source: &str,
    horizontal: bool,
) -> Result<BrowserLonghandValue, EngineError> {
    parse_entire(source, |input| {
        let keywords = if horizontal {
            [("left", 0.0), ("center", 0.5), ("right", 1.0)]
        } else {
            [("top", 0.0), ("center", 0.5), ("bottom", 1.0)]
        };
        for (keyword, percentage) in keywords {
            if input
                .try_parse(|input| input.expect_ident_matching(keyword))
                .is_ok()
            {
                return Ok(BrowserLonghandValue::AxisPosition(
                    LengthPercentage::Percentage(Percentage(percentage)),
                ));
            }
        }
        parse_strict_length_percentage(input).map(BrowserLonghandValue::AxisPosition)
    })
}

fn parse_position(source: &str) -> Result<BrowserLonghandValue, EngineError> {
    reject_top_level_nonzero_numbers(source)?;
    parse_entire(source, |input| {
        Position::parse(input).map(|value| BrowserLonghandValue::Position(value.into_owned()))
    })
}

fn parse_auto_length_percentage(source: &str) -> Result<BrowserLonghandValue, EngineError> {
    parse_entire(source, |input| {
        if input
            .try_parse(|input| input.expect_ident_matching("auto"))
            .is_ok()
        {
            return Ok(BrowserLonghandValue::AutoLengthPercentage(None));
        }
        parse_strict_length_percentage(input)
            .map(|value| BrowserLonghandValue::AutoLengthPercentage(Some(value)))
    })
}

fn parse_containment(source: &str) -> Result<BrowserLonghandValue, EngineError> {
    const CANONICAL: &[&str] = &["size", "inline-size", "layout", "style", "paint"];
    parse_entire(source, |input| {
        for exclusive in ["none", "strict", "content"] {
            if input
                .try_parse(|input| input.expect_ident_matching(exclusive))
                .is_ok()
            {
                return Ok(BrowserLonghandValue::Containment(vec![exclusive]));
            }
        }

        let mut selected = Vec::with_capacity(CANONICAL.len());
        while !input.is_exhausted() {
            let value = parse_one_keyword(input, CANONICAL)?;
            if selected.contains(&value)
                || value == "size" && selected.contains(&"inline-size")
                || value == "inline-size" && selected.contains(&"size")
            {
                return Err(input.new_custom_error(lightningcss::error::ParserError::InvalidValue));
            }
            selected.push(value);
        }
        if selected.is_empty() {
            return Err(input.new_custom_error(lightningcss::error::ParserError::InvalidValue));
        }
        selected.sort_by_key(|value| CANONICAL.iter().position(|candidate| candidate == value));
        Ok(BrowserLonghandValue::Containment(selected))
    })
}

fn parse_text_decoration_line(source: &str) -> Result<BrowserLonghandValue, EngineError> {
    const CANONICAL: &[&str] = &[
        "underline",
        "overline",
        "line-through",
        "blink",
        "spelling-error",
        "grammar-error",
    ];
    parse_entire(source, |input| {
        if input
            .try_parse(|input| input.expect_ident_matching("none"))
            .is_ok()
        {
            return Ok(BrowserLonghandValue::TextDecorationLine(vec!["none"]));
        }
        let mut selected = Vec::with_capacity(CANONICAL.len());
        while !input.is_exhausted() {
            let value = parse_one_keyword(input, CANONICAL)?;
            if selected.contains(&value)
                || matches!(value, "spelling-error" | "grammar-error") && !selected.is_empty()
                || selected
                    .iter()
                    .any(|value| matches!(*value, "spelling-error" | "grammar-error"))
            {
                return Err(input.new_custom_error(lightningcss::error::ParserError::InvalidValue));
            }
            selected.push(value);
        }
        if selected.is_empty() {
            return Err(input.new_custom_error(lightningcss::error::ParserError::InvalidValue));
        }
        selected.sort_by_key(|value| CANONICAL.iter().position(|candidate| candidate == value));
        Ok(BrowserLonghandValue::TextDecorationLine(selected))
    })
}

fn parse_scroll_snap_align(source: &str) -> Result<BrowserLonghandValue, EngineError> {
    parse_entire(source, |input| {
        let block = parse_one_keyword(input, &["none", "start", "end", "center"])?;
        let inline = input
            .try_parse(|input| parse_one_keyword(input, &["none", "start", "end", "center"]))
            .ok();
        Ok(BrowserLonghandValue::ScrollSnapAlign { block, inline })
    })
}

fn parse_scroll_snap_type(source: &str) -> Result<BrowserLonghandValue, EngineError> {
    parse_entire(source, |input| {
        if input
            .try_parse(|input| input.expect_ident_matching("none"))
            .is_ok()
        {
            return Ok(BrowserLonghandValue::ScrollSnapType {
                axis: "none",
                strictness: None,
            });
        }
        let axis = parse_one_keyword(input, &["x", "y", "block", "inline", "both"])?;
        let strictness = input
            .try_parse(|input| parse_one_keyword(input, &["proximity", "mandatory"]))
            .ok()
            .filter(|strictness| *strictness != "proximity");
        Ok(BrowserLonghandValue::ScrollSnapType { axis, strictness })
    })
}

fn parse_scrollbar_gutter(source: &str) -> Result<BrowserLonghandValue, EngineError> {
    parse_entire(source, |input| {
        if input
            .try_parse(|input| input.expect_ident_matching("auto"))
            .is_ok()
        {
            return Ok(BrowserLonghandValue::Keyword("auto"));
        }
        input.expect_ident_matching("stable")?;
        let both_edges = input
            .try_parse(|input| input.expect_ident_matching("both-edges"))
            .is_ok();
        Ok(BrowserLonghandValue::ScrollbarGutter { both_edges })
    })
}

fn parse_overflow_clip_margin(source: &str) -> Result<BrowserLonghandValue, EngineError> {
    parse_entire(source, |input| {
        let mut visual_box = None;
        let mut length = None;
        let mut unitless_number = false;
        let mut calculation = false;
        while !input.is_exhausted() {
            if visual_box.is_none() {
                if let Ok(value) = input.try_parse(|input| {
                    parse_one_keyword(input, &["content-box", "padding-box", "border-box"])
                }) {
                    visual_box = Some(value);
                    continue;
                }
            }
            if length.is_none() {
                let state = input.state();
                let token = input.next()?.clone();
                unitless_number = matches!(token, Token::Number { .. });
                calculation = matches!(token, Token::Function(_));
                input.reset(&state);
                let value = parse_strict_length(input)?;
                if value.try_sign().is_some_and(|sign| sign < 0.0) {
                    return Err(
                        input.new_custom_error(lightningcss::error::ParserError::InvalidValue)
                    );
                }
                length = Some(value);
                continue;
            }
            return Err(input.new_custom_error(lightningcss::error::ParserError::InvalidValue));
        }
        if visual_box.is_none() && length.is_none()
            || visual_box.is_none() && (unitless_number || calculation)
        {
            return Err(input.new_custom_error(lightningcss::error::ParserError::InvalidValue));
        }
        Ok(BrowserLonghandValue::OverflowClipMargin {
            visual_box: visual_box.unwrap_or("padding-box"),
            length,
            calculation,
        })
    })
}

fn parse_text_underline_position(source: &str) -> Result<BrowserLonghandValue, EngineError> {
    parse_entire(source, |input| {
        let mut primary = None;
        let mut side = None;
        while !input.is_exhausted() {
            if primary.is_none() {
                if let Ok(value) = input
                    .try_parse(|input| parse_one_keyword(input, &["auto", "from-font", "under"]))
                {
                    primary = Some(value);
                    continue;
                }
            }
            if side.is_none() {
                side = Some(parse_one_keyword(input, &["left", "right"])?);
                continue;
            }
            return Err(input.new_custom_error(lightningcss::error::ParserError::InvalidValue));
        }
        if primary.is_none() && side.is_none() {
            return Err(input.new_custom_error(lightningcss::error::ParserError::InvalidValue));
        }
        Ok(BrowserLonghandValue::TextUnderlinePosition { primary, side })
    })
}

fn parse_touch_action(source: &str) -> Result<BrowserLonghandValue, EngineError> {
    parse_entire(source, |input| {
        for exclusive in ["auto", "none", "manipulation"] {
            if input
                .try_parse(|input| input.expect_ident_matching(exclusive))
                .is_ok()
            {
                return Ok(BrowserLonghandValue::TouchAction(vec![exclusive]));
            }
        }
        let mut horizontal = None;
        let mut vertical = None;
        let mut pinch_zoom = false;
        while !input.is_exhausted() {
            let value = parse_one_keyword(
                input,
                &[
                    "pan-x",
                    "pan-left",
                    "pan-right",
                    "pan-y",
                    "pan-up",
                    "pan-down",
                    "pinch-zoom",
                ],
            )?;
            match value {
                "pan-x" | "pan-left" | "pan-right" if horizontal.is_none() => {
                    horizontal = Some(value)
                }
                "pan-y" | "pan-up" | "pan-down" if vertical.is_none() => vertical = Some(value),
                "pinch-zoom" if !pinch_zoom => pinch_zoom = true,
                _ => {
                    return Err(
                        input.new_custom_error(lightningcss::error::ParserError::InvalidValue)
                    )
                }
            }
        }
        if horizontal.is_none() && vertical.is_none() && !pinch_zoom {
            return Err(input.new_custom_error(lightningcss::error::ParserError::InvalidValue));
        }
        let mut values = Vec::with_capacity(3);
        if let Some(horizontal) = horizontal {
            values.push(horizontal);
        }
        if let Some(vertical) = vertical {
            values.push(vertical);
        }
        if pinch_zoom {
            values.push("pinch-zoom");
        }
        Ok(BrowserLonghandValue::TouchAction(values))
    })
}

fn parse_string_or_keyword(
    source: &str,
    keyword: &'static str,
) -> Result<BrowserLonghandValue, EngineError> {
    parse_entire(source, |input| {
        if input
            .try_parse(|input| input.expect_ident_matching(keyword))
            .is_ok()
        {
            return Ok(BrowserLonghandValue::StringOrKeyword(
                StringOrKeywordValue::Keyword(keyword),
            ));
        }
        let string = CSSString::parse(input)?.into_owned();
        Ok(BrowserLonghandValue::StringOrKeyword(
            StringOrKeywordValue::String(string),
        ))
    })
}

fn parse_keyword_or_dashed_ident(
    source: &str,
    keywords: &'static [&'static str],
) -> Result<BrowserLonghandValue, EngineError> {
    parse_entire(source, |input| {
        if let Ok(keyword) = input.try_parse(|input| parse_one_keyword(input, keywords)) {
            return Ok(BrowserLonghandValue::KeywordOrDashedIdent(
                KeywordOrDashedIdentValue::Keyword(keyword),
            ));
        }
        let name = DashedIdent::parse(input)?.into_owned();
        Ok(BrowserLonghandValue::KeywordOrDashedIdent(
            KeywordOrDashedIdentValue::Name(name),
        ))
    })
}

fn parse_position_visibility(source: &str) -> Result<BrowserLonghandValue, EngineError> {
    parse_entire(source, |input| {
        if input
            .try_parse(|input| input.expect_ident_matching("always"))
            .is_ok()
        {
            return Ok(BrowserLonghandValue::PositionVisibility {
                anchors_visible: false,
                no_overflow: false,
            });
        }
        let mut anchors_visible = false;
        let mut no_overflow = false;
        while !input.is_exhausted() {
            let keyword = parse_one_keyword(input, &["anchors-visible", "no-overflow"])?;
            match keyword {
                "anchors-visible" if !anchors_visible => anchors_visible = true,
                "no-overflow" if !no_overflow => no_overflow = true,
                _ => {
                    return Err(
                        input.new_custom_error(lightningcss::error::ParserError::InvalidValue)
                    )
                }
            }
        }
        if !anchors_visible && !no_overflow {
            return Err(input.new_custom_error(lightningcss::error::ParserError::InvalidValue));
        }
        Ok(BrowserLonghandValue::PositionVisibility {
            anchors_visible,
            no_overflow,
        })
    })
}

fn parse_counter_list(
    source: &str,
    default_value: i32,
) -> Result<BrowserLonghandValue, EngineError> {
    parse_entire(source, |input| {
        if input
            .try_parse(|input| input.expect_ident_matching("none"))
            .is_ok()
        {
            return Ok(BrowserLonghandValue::CounterList(None));
        }
        let mut entries = Vec::new();
        while !input.is_exhausted() {
            let name = CustomIdent::parse(input)?.into_owned();
            if name.eq_ignore_ascii_case("none") {
                return Err(input.new_custom_error(lightningcss::error::ParserError::InvalidValue));
            }
            let value = input
                .try_parse(|input| input.expect_integer())
                .unwrap_or(default_value);
            entries.push(CounterEntry { name, value });
        }
        if entries.is_empty() {
            return Err(input.new_custom_error(lightningcss::error::ParserError::InvalidValue));
        }
        Ok(BrowserLonghandValue::CounterList(Some(entries)))
    })
}

fn parse_custom_ident(source: &str) -> Result<BrowserLonghandValue, EngineError> {
    parse_entire(source, |input| {
        CustomIdent::parse(input)
            .map(IntoOwned::into_owned)
            .map(BrowserLonghandValue::CustomIdent)
    })
}

fn parse_quotes(source: &str) -> Result<BrowserLonghandValue, EngineError> {
    parse_entire(source, |input| {
        if input
            .try_parse(|input| input.expect_ident_matching("auto"))
            .is_ok()
        {
            return Ok(BrowserLonghandValue::Quotes(QuotesValue::Auto));
        }
        if input
            .try_parse(|input| input.expect_ident_matching("none"))
            .is_ok()
        {
            return Ok(BrowserLonghandValue::Quotes(QuotesValue::None));
        }
        let mut pairs = Vec::new();
        while !input.is_exhausted() {
            let open = CSSString::parse(input)?.into_owned();
            let close = CSSString::parse(input)?.into_owned();
            pairs.push((open, close));
        }
        if pairs.is_empty() {
            return Err(input.new_custom_error(lightningcss::error::ParserError::InvalidValue));
        }
        Ok(BrowserLonghandValue::Quotes(QuotesValue::Pairs(pairs)))
    })
}

fn parse_paint_order(source: &str) -> Result<BrowserLonghandValue, EngineError> {
    const DEFAULT_ORDER: &[&str] = &["fill", "stroke", "markers"];
    parse_entire(source, |input| {
        if input
            .try_parse(|input| input.expect_ident_matching("normal"))
            .is_ok()
        {
            return Ok(BrowserLonghandValue::PaintOrder(vec!["normal"]));
        }
        let mut specified = Vec::with_capacity(DEFAULT_ORDER.len());
        while !input.is_exhausted() {
            let value = parse_one_keyword(input, DEFAULT_ORDER)?;
            if specified.contains(&value) {
                return Err(input.new_custom_error(lightningcss::error::ParserError::InvalidValue));
            }
            specified.push(value);
        }
        if specified.is_empty() {
            return Err(input.new_custom_error(lightningcss::error::ParserError::InvalidValue));
        }
        let mut full_order = specified.clone();
        full_order.extend(
            DEFAULT_ORDER
                .iter()
                .copied()
                .filter(|value| !specified.contains(value)),
        );
        for prefix_length in 1..=full_order.len() {
            let mut reconstructed = full_order[..prefix_length].to_vec();
            let missing = DEFAULT_ORDER
                .iter()
                .copied()
                .filter(|value| !reconstructed.contains(value))
                .collect::<Vec<_>>();
            reconstructed.extend(missing);
            if reconstructed == full_order {
                return Ok(BrowserLonghandValue::PaintOrder(
                    full_order[..prefix_length].to_vec(),
                ));
            }
        }
        Err(input.new_custom_error(lightningcss::error::ParserError::InvalidValue))
    })
}

fn parse_will_change(source: &str) -> Result<BrowserLonghandValue, EngineError> {
    parse_entire(source, |input| {
        if input
            .try_parse(|input| input.expect_ident_matching("auto"))
            .is_ok()
        {
            return Ok(BrowserLonghandValue::WillChange(None));
        }
        let values = input.parse_comma_separated(|input| {
            let value = CustomIdent::parse(input)?.into_owned();
            if value.eq_ignore_ascii_case("auto")
                || value.eq_ignore_ascii_case("none")
                || value.eq_ignore_ascii_case("will-change")
            {
                return Err(input.new_custom_error(lightningcss::error::ParserError::InvalidValue));
            }
            Ok(value)
        })?;
        Ok(BrowserLonghandValue::WillChange(Some(values)))
    })
}

fn parse_animation_iteration_count(source: &str) -> Result<BrowserLonghandValue, EngineError> {
    parse_entire(source, |input| {
        let values = input.parse_comma_separated(|input| {
            if input
                .try_parse(|input| input.expect_ident_matching("infinite"))
                .is_ok()
            {
                return Ok(AnimationIterationCountValue::Infinite);
            }
            let value = match input.try_parse(Calc::<PreservedLengthPercentage>::parse) {
                Ok(value) => value,
                Err(_) => Calc::Number(CSSNumber::parse(input)?),
            };
            if !value.resolves_to_number()
                || (!value.contains_unresolved_sign()
                    && value.try_sign().is_some_and(|sign| sign < 0.0))
            {
                return Err(input.new_custom_error(lightningcss::error::ParserError::InvalidValue));
            }
            Ok(AnimationIterationCountValue::Number(value))
        })?;
        Ok(BrowserLonghandValue::AnimationIterationCount(values))
    })
}

fn parse_offset_path(source: &str) -> Result<BrowserLonghandValue, EngineError> {
    parse_entire(source, |input| {
        if let Ok(value) = input.try_parse(ClipPath::parse) {
            return Ok(BrowserLonghandValue::OffsetPath(OffsetPathValue::ClipPath(
                value.into_owned(),
            )));
        }
        let location = input.current_source_location();
        let function = input.expect_function()?.clone();
        if function.eq_ignore_ascii_case("path") {
            let (fill_rule, data) = input.parse_nested_block(|input| {
                let fill_rule = input.try_parse(parse_fill_rule_and_comma).ok();
                let data = CSSString::parse(input)?.into_owned();
                Ok((fill_rule, data))
            })?;
            return Ok(BrowserLonghandValue::OffsetPath(OffsetPathValue::Path {
                fill_rule,
                data,
            }));
        }
        if function.eq_ignore_ascii_case("ray") {
            let angle = input.parse_nested_block(Angle::parse)?;
            return Ok(BrowserLonghandValue::OffsetPath(OffsetPathValue::Ray(
                angle,
            )));
        }
        Err(location.new_custom_error(lightningcss::error::ParserError::InvalidValue))
    })
}

fn parse_fill_rule_and_comma<'i, 't>(
    input: &mut Parser<'i, 't>,
) -> Result<&'static str, ParseError<'i>> {
    let fill_rule = parse_one_keyword(input, &["nonzero", "evenodd"])?;
    input.expect_comma()?;
    Ok(fill_rule)
}

fn parse_auto_length(source: &str) -> Result<BrowserLonghandValue, EngineError> {
    parse_entire(source, |input| {
        if input
            .try_parse(|input| input.expect_ident_matching("auto"))
            .is_ok()
        {
            return Ok(BrowserLonghandValue::AutoLength(None));
        }
        let value = parse_strict_length(input)?;
        if value.try_sign().is_some_and(|sign| sign < 0.0) {
            return Err(input.new_custom_error(lightningcss::error::ParserError::InvalidValue));
        }
        Ok(BrowserLonghandValue::AutoLength(Some(value)))
    })
}

fn parse_column_count(source: &str) -> Result<BrowserLonghandValue, EngineError> {
    parse_entire(source, |input| {
        if input
            .try_parse(|input| input.expect_ident_matching("auto"))
            .is_ok()
        {
            return Ok(BrowserLonghandValue::ColumnCount(ColumnCountValue {
                count: None,
                wrap_calc: false,
            }));
        }
        let state = input.state();
        let wrap_calc = input
            .expect_function()
            .is_ok_and(|function| function.eq_ignore_ascii_case("calc"));
        input.reset(&state);
        let count = match input.try_parse(Calc::<PreservedLengthPercentage>::parse) {
            Ok(value) => value,
            Err(_) => Calc::Number(CSSNumber::parse(input)?),
        };
        if !count.resolves_to_number()
            || matches!(&count, Calc::Number(value) if *value < 1.0 || value.fract() != 0.0)
        {
            return Err(input.new_custom_error(lightningcss::error::ParserError::InvalidValue));
        }
        Ok(BrowserLonghandValue::ColumnCount(ColumnCountValue {
            count: Some(count),
            wrap_calc,
        }))
    })
}

fn parse_contain_intrinsic(source: &str) -> Result<BrowserLonghandValue, EngineError> {
    parse_entire(source, |input| {
        if input
            .try_parse(|input| input.expect_ident_matching("none"))
            .is_ok()
        {
            return Ok(BrowserLonghandValue::ContainIntrinsic(
                ContainIntrinsicValue::None,
            ));
        }
        let auto = input
            .try_parse(|input| input.expect_ident_matching("auto"))
            .is_ok();
        if auto
            && input
                .try_parse(|input| input.expect_ident_matching("none"))
                .is_ok()
        {
            return Ok(BrowserLonghandValue::ContainIntrinsic(
                ContainIntrinsicValue::Length { auto, value: None },
            ));
        }
        let value = parse_strict_length(input)?;
        if value.try_sign().is_some_and(|sign| sign < 0.0) {
            return Err(input.new_custom_error(lightningcss::error::ParserError::InvalidValue));
        }
        Ok(BrowserLonghandValue::ContainIntrinsic(
            ContainIntrinsicValue::Length {
                auto,
                value: Some(value),
            },
        ))
    })
}

fn parse_dashed_ident_list(
    source: &str,
    allow_all: bool,
) -> Result<BrowserLonghandValue, EngineError> {
    parse_entire(source, |input| {
        if input
            .try_parse(|input| input.expect_ident_matching("none"))
            .is_ok()
        {
            return Ok(BrowserLonghandValue::DashedIdentList(
                DashedIdentListValue::None,
            ));
        }
        if allow_all
            && input
                .try_parse(|input| input.expect_ident_matching("all"))
                .is_ok()
        {
            return Ok(BrowserLonghandValue::DashedIdentList(
                DashedIdentListValue::All,
            ));
        }
        let values = input
            .parse_comma_separated(DashedIdent::parse)?
            .into_iter()
            .map(IntoOwned::into_owned)
            .collect();
        Ok(BrowserLonghandValue::DashedIdentList(
            DashedIdentListValue::Names(values),
        ))
    })
}

fn parse_time_or_normal(source: &str) -> Result<BrowserLonghandValue, EngineError> {
    parse_entire(source, |input| {
        if input
            .try_parse(|input| input.expect_ident_matching("normal"))
            .is_ok()
        {
            return Ok(BrowserLonghandValue::TimeOrNormal(None));
        }
        let value = Time::parse(input)?;
        if value.to_ms() < 0.0 {
            return Err(input.new_custom_error(lightningcss::error::ParserError::InvalidValue));
        }
        Ok(BrowserLonghandValue::TimeOrNormal(Some(value)))
    })
}

fn parse_view_timeline_inset(source: &str) -> Result<BrowserLonghandValue, EngineError> {
    parse_entire(source, |input| {
        let entries = input.parse_comma_separated(|input| {
            let mut values = Vec::with_capacity(2);
            while !input.is_exhausted() && values.len() < 2 {
                if input
                    .try_parse(|input| input.expect_ident_matching("auto"))
                    .is_ok()
                {
                    values.push(ViewTimelineInsetComponent::Auto);
                    continue;
                }
                values.push(ViewTimelineInsetComponent::LengthPercentage(
                    parse_strict_length_percentage(input)?,
                ));
            }
            if values.is_empty() {
                return Err(input.new_custom_error(lightningcss::error::ParserError::InvalidValue));
            }
            Ok(ViewTimelineInsetValue { values })
        })?;
        Ok(BrowserLonghandValue::ViewTimelineInset(entries))
    })
}

fn parse_corner_shape(source: &str) -> Result<BrowserLonghandValue, EngineError> {
    parse_entire(source, |input| {
        if let Ok(identifier) = input.try_parse(|input| input.expect_ident_cloned()) {
            let Some(keyword) = ["round", "bevel", "scoop", "notch", "square", "squircle"]
                .into_iter()
                .find(|candidate| identifier.eq_ignore_ascii_case(candidate))
            else {
                return Err(input.new_custom_error(lightningcss::error::ParserError::InvalidValue));
            };
            return Ok(BrowserLonghandValue::CornerShape(
                CornerShapeValue::Keyword(keyword),
            ));
        }
        let function = input.expect_function()?.clone();
        if !function.eq_ignore_ascii_case("superellipse") {
            return Err(input.new_custom_error(lightningcss::error::ParserError::InvalidValue));
        }
        let value = input.parse_nested_block(CSSNumber::parse)?;
        Ok(BrowserLonghandValue::CornerShape(
            CornerShapeValue::Superellipse(value),
        ))
    })
}

fn parse_color_scheme(source: &str) -> Result<BrowserLonghandValue, EngineError> {
    parse_entire(source, |input| {
        if input
            .try_parse(|input| input.expect_ident_matching("normal"))
            .is_ok()
        {
            return Ok(BrowserLonghandValue::ColorScheme(ColorSchemeValue::Normal));
        }

        #[derive(Clone, Copy, PartialEq)]
        enum OnlyPosition {
            Prefix,
            Suffix,
        }

        let mut values = Vec::new();
        let mut only = None;
        while !input.is_exhausted() {
            if input
                .try_parse(|input| input.expect_ident_matching("only"))
                .is_ok()
            {
                if only.is_some() {
                    return Err(
                        input.new_custom_error(lightningcss::error::ParserError::InvalidValue)
                    );
                }
                only = Some(if values.is_empty() {
                    OnlyPosition::Prefix
                } else {
                    OnlyPosition::Suffix
                });
                continue;
            }
            if only == Some(OnlyPosition::Suffix) {
                return Err(input.new_custom_error(lightningcss::error::ParserError::InvalidValue));
            }
            if input
                .try_parse(|input| input.expect_ident_matching("normal"))
                .is_ok()
            {
                return Err(input.new_custom_error(lightningcss::error::ParserError::InvalidValue));
            }
            values.push(CustomIdent::parse(input)?.into_owned());
        }
        if values.is_empty() {
            return Err(input.new_custom_error(lightningcss::error::ParserError::InvalidValue));
        }
        Ok(BrowserLonghandValue::ColorScheme(
            ColorSchemeValue::Schemes {
                values,
                only: only.is_some(),
            },
        ))
    })
}

fn parse_font_feature_settings(source: &str) -> Result<BrowserLonghandValue, EngineError> {
    parse_entire(source, |input| {
        if input
            .try_parse(|input| input.expect_ident_matching("normal"))
            .is_ok()
        {
            return Ok(BrowserLonghandValue::FontFeatureSettings(None));
        }
        let settings = input.parse_comma_separated(|input| {
            let tag = parse_opentype_tag(input)?;
            let value = if input
                .try_parse(|input| input.expect_ident_matching("on"))
                .is_ok()
            {
                1
            } else if input
                .try_parse(|input| input.expect_ident_matching("off"))
                .is_ok()
            {
                0
            } else {
                input.try_parse(|input| input.expect_integer()).unwrap_or(1)
            };
            Ok(FontFeatureSetting { tag, value })
        })?;
        Ok(BrowserLonghandValue::FontFeatureSettings(Some(settings)))
    })
}

fn parse_font_language_override(source: &str) -> Result<BrowserLonghandValue, EngineError> {
    parse_entire(source, |input| {
        if input
            .try_parse(|input| input.expect_ident_matching("normal"))
            .is_ok()
        {
            return Ok(BrowserLonghandValue::FontLanguageOverride(None));
        }
        let value = CSSString::parse(input)?;
        if value.0.is_empty() || !value.0.is_ascii() || value.0.len() > 4 {
            return Err(input.new_custom_error(lightningcss::error::ParserError::InvalidValue));
        }
        Ok(BrowserLonghandValue::FontLanguageOverride(Some(
            value.into_owned(),
        )))
    })
}

fn parse_font_size_adjust(source: &str) -> Result<BrowserLonghandValue, EngineError> {
    parse_entire(source, |input| {
        if input
            .try_parse(|input| input.expect_ident_matching("none"))
            .is_ok()
        {
            return Ok(BrowserLonghandValue::FontSizeAdjust(
                FontSizeAdjustValue::None,
            ));
        }
        let metric = input
            .try_parse(|input| {
                parse_one_keyword(
                    input,
                    &[
                        "ex-height",
                        "cap-height",
                        "ch-width",
                        "ic-width",
                        "ic-height",
                    ],
                )
            })
            .ok();
        let value = if input
            .try_parse(|input| input.expect_ident_matching("from-font"))
            .is_ok()
        {
            FontSizeAdjustNumber::FromFont
        } else {
            let state = input.state();
            let wrap_calc = input
                .expect_function()
                .is_ok_and(|function| function.eq_ignore_ascii_case("calc"));
            input.reset(&state);
            let value = match input.try_parse(Calc::<PreservedLengthPercentage>::parse) {
                Ok(value) => value,
                Err(_) => Calc::Number(CSSNumber::parse(input)?),
            };
            if !value.resolves_to_number() || matches!(&value, Calc::Number(value) if *value < 0.0)
            {
                return Err(input.new_custom_error(lightningcss::error::ParserError::InvalidValue));
            }
            FontSizeAdjustNumber::Number { value, wrap_calc }
        };
        Ok(BrowserLonghandValue::FontSizeAdjust(
            FontSizeAdjustValue::Value { metric, value },
        ))
    })
}

fn parse_font_variant_alternates(source: &str) -> Result<BrowserLonghandValue, EngineError> {
    parse_entire(source, |input| {
        if input
            .try_parse(|input| input.expect_ident_matching("normal"))
            .is_ok()
        {
            return Ok(BrowserLonghandValue::FontVariantAlternates(
                FontVariantAlternatesValue::default(),
            ));
        }
        let mut value = FontVariantAlternatesValue::default();
        while !input.is_exhausted() {
            if input
                .try_parse(|input| input.expect_ident_matching("historical-forms"))
                .is_ok()
            {
                value.historical_forms = true;
                continue;
            }
            let location = input.current_source_location();
            let function = input.expect_function()?.clone();
            let Some(name) = [
                "stylistic",
                "styleset",
                "character-variant",
                "swash",
                "ornaments",
                "annotation",
            ]
            .into_iter()
            .find(|candidate| function.eq_ignore_ascii_case(candidate)) else {
                return Err(
                    location.new_custom_error(lightningcss::error::ParserError::InvalidValue)
                );
            };
            if value.functions.iter().any(|existing| existing.name == name) {
                return Err(
                    location.new_custom_error(lightningcss::error::ParserError::InvalidValue)
                );
            }
            let values = input.parse_nested_block(|input| {
                let values = if matches!(name, "styleset" | "character-variant") {
                    input.parse_comma_separated(CustomIdent::parse)?
                } else {
                    vec![CustomIdent::parse(input)?]
                };
                Ok(values.into_iter().map(IntoOwned::into_owned).collect())
            })?;
            value.functions.push(FontAlternateFunction { name, values });
        }
        if !value.historical_forms && value.functions.is_empty() {
            return Err(input.new_custom_error(lightningcss::error::ParserError::InvalidValue));
        }
        Ok(BrowserLonghandValue::FontVariantAlternates(value))
    })
}

fn parse_font_variant_keywords(
    property_name: &str,
    source: &str,
) -> Result<BrowserLonghandValue, EngineError> {
    parse_entire(source, |input| {
        if input
            .try_parse(|input| input.expect_ident_matching("normal"))
            .is_ok()
        {
            return Ok(BrowserLonghandValue::FontVariantKeywords(vec!["normal"]));
        }
        if property_name == "font-variant-ligatures"
            && input
                .try_parse(|input| input.expect_ident_matching("none"))
                .is_ok()
        {
            return Ok(BrowserLonghandValue::FontVariantKeywords(vec!["none"]));
        }
        let groups = font_variant_keyword_groups(property_name);
        let mut values = Vec::new();
        let mut used_groups = Vec::new();
        while !input.is_exhausted() {
            let location = input.current_source_location();
            let identifier = input.expect_ident_cloned()?;
            let Some((keyword, group)) = groups.iter().find_map(|(keyword, group)| {
                identifier
                    .eq_ignore_ascii_case(keyword)
                    .then_some((*keyword, *group))
            }) else {
                return Err(
                    location.new_custom_error(lightningcss::error::ParserError::InvalidValue)
                );
            };
            if used_groups.contains(&group) {
                return Err(
                    location.new_custom_error(lightningcss::error::ParserError::InvalidValue)
                );
            }
            used_groups.push(group);
            values.push((keyword, group));
        }
        if values.is_empty() {
            return Err(input.new_custom_error(lightningcss::error::ParserError::InvalidValue));
        }
        if property_name == "font-variant-east-asian" {
            values.sort_by_key(|(_, group)| *group);
        }
        Ok(BrowserLonghandValue::FontVariantKeywords(
            values.into_iter().map(|(keyword, _)| keyword).collect(),
        ))
    })
}

fn font_variant_keyword_groups(property_name: &str) -> &'static [(&'static str, u8)] {
    match property_name {
        "font-variant-east-asian" => &[
            ("jis78", 0),
            ("jis83", 0),
            ("jis90", 0),
            ("jis04", 0),
            ("simplified", 0),
            ("traditional", 0),
            ("full-width", 1),
            ("proportional-width", 1),
            ("ruby", 2),
        ],
        "font-variant-ligatures" => &[
            ("common-ligatures", 0),
            ("no-common-ligatures", 0),
            ("discretionary-ligatures", 1),
            ("no-discretionary-ligatures", 1),
            ("historical-ligatures", 2),
            ("no-historical-ligatures", 2),
            ("contextual", 3),
            ("no-contextual", 3),
        ],
        "font-variant-numeric" => &[
            ("lining-nums", 0),
            ("oldstyle-nums", 0),
            ("proportional-nums", 1),
            ("tabular-nums", 1),
            ("diagonal-fractions", 2),
            ("stacked-fractions", 2),
            ("ordinal", 3),
            ("slashed-zero", 4),
        ],
        _ => &[],
    }
}

fn parse_font_variation_settings(source: &str) -> Result<BrowserLonghandValue, EngineError> {
    parse_entire(source, |input| {
        if input
            .try_parse(|input| input.expect_ident_matching("normal"))
            .is_ok()
        {
            return Ok(BrowserLonghandValue::FontVariationSettings(None));
        }
        let settings = input.parse_comma_separated(|input| {
            let tag = parse_opentype_tag(input)?;
            let state = input.state();
            let calculation = input.expect_function().is_ok();
            input.reset(&state);
            let value = CSSNumber::parse(input)?;
            Ok(FontVariationSetting {
                tag,
                value,
                calculation,
            })
        })?;
        Ok(BrowserLonghandValue::FontVariationSettings(Some(settings)))
    })
}

fn parse_opentype_tag<'i, 't>(
    input: &mut Parser<'i, 't>,
) -> Result<CSSString<'static>, ParseError<'i>> {
    let value = CSSString::parse(input)?;
    if value.0.len() != 4 || !value.0.is_ascii() {
        return Err(input.new_custom_error(lightningcss::error::ParserError::InvalidValue));
    }
    Ok(value.into_owned())
}

fn parse_position_try_fallbacks(source: &str) -> Result<BrowserLonghandValue, EngineError> {
    parse_entire(source, |input| {
        if input
            .try_parse(|input| input.expect_ident_matching("none"))
            .is_ok()
        {
            return Ok(BrowserLonghandValue::PositionTryFallbacks(None));
        }
        let fallbacks = input.parse_comma_separated(|input| {
            let area = input
                .try_parse(|input| parse_position_area_value(input, false))
                .ok();
            let name = if area.is_none() {
                input
                    .try_parse(DashedIdent::parse)
                    .ok()
                    .map(IntoOwned::into_owned)
            } else {
                None
            };
            let mut tactics = Vec::with_capacity(3);
            while !input.is_exhausted() {
                let location = input.current_source_location();
                let identifier = input.expect_ident_cloned()?;
                let Some(tactic) = ["flip-block", "flip-inline", "flip-start"]
                    .into_iter()
                    .find(|candidate| identifier.eq_ignore_ascii_case(candidate))
                else {
                    return Err(
                        location.new_custom_error(lightningcss::error::ParserError::InvalidValue)
                    );
                };
                if tactics.contains(&tactic) {
                    return Err(
                        location.new_custom_error(lightningcss::error::ParserError::InvalidValue)
                    );
                }
                tactics.push(tactic);
            }
            if name.is_none() && area.is_none() && tactics.is_empty() {
                return Err(input.new_custom_error(lightningcss::error::ParserError::InvalidValue));
            }
            Ok(PositionTryFallback {
                name,
                area,
                tactics,
            })
        })?;
        Ok(BrowserLonghandValue::PositionTryFallbacks(Some(fallbacks)))
    })
}

fn parse_text_box_edge(source: &str) -> Result<BrowserLonghandValue, EngineError> {
    parse_entire(source, |input| {
        if input
            .try_parse(|input| input.expect_ident_matching("auto"))
            .is_ok()
        {
            return Ok(BrowserLonghandValue::TextBoxEdge(TextBoxEdgeValue::Auto));
        }
        let over = parse_one_keyword(input, &["text", "cap", "ex"])?;
        let under = input
            .try_parse(|input| parse_one_keyword(input, &["text", "alphabetic"]))
            .ok();
        if under.is_none() && over != "text" {
            return Err(input.new_custom_error(lightningcss::error::ParserError::InvalidValue));
        }
        Ok(BrowserLonghandValue::TextBoxEdge(TextBoxEdgeValue::Edges {
            over,
            under,
        }))
    })
}

fn parse_timeline_range_start(
    source: &str,
    accepts_auto: bool,
) -> Result<BrowserLonghandValue, EngineError> {
    parse_entire(source, |input| {
        let values = input.parse_comma_separated(|input| {
            if accepts_auto
                && input
                    .try_parse(|input| input.expect_ident_matching("auto"))
                    .is_ok()
            {
                return Ok(TimelineRangeStartValue::Auto);
            }
            AnimationRangeStart::parse(input).map(TimelineRangeStartValue::Range)
        })?;
        Ok(BrowserLonghandValue::TimelineRangeStart(values))
    })
}

fn parse_timeline_range_end(
    source: &str,
    accepts_auto: bool,
) -> Result<BrowserLonghandValue, EngineError> {
    parse_entire(source, |input| {
        let values = input.parse_comma_separated(|input| {
            if accepts_auto
                && input
                    .try_parse(|input| input.expect_ident_matching("auto"))
                    .is_ok()
            {
                return Ok(TimelineRangeEndValue::Auto);
            }
            AnimationRangeEnd::parse(input).map(TimelineRangeEndValue::Range)
        })?;
        Ok(BrowserLonghandValue::TimelineRangeEnd(values))
    })
}

fn parse_timeline_trigger_name(source: &str) -> Result<BrowserLonghandValue, EngineError> {
    parse_entire(source, |input| {
        let names = input.parse_comma_separated(|input| {
            if input
                .try_parse(|input| input.expect_ident_matching("none"))
                .is_ok()
            {
                return Ok(None);
            }
            DashedIdent::parse(input)
                .map(IntoOwned::into_owned)
                .map(Some)
        })?;
        Ok(BrowserLonghandValue::TimelineTriggerName(names))
    })
}

fn parse_timeline_trigger_source(source: &str) -> Result<BrowserLonghandValue, EngineError> {
    parse_entire(source, |input| {
        let timelines = input
            .parse_comma_separated(AnimationTimeline::parse)?
            .into_iter()
            .map(IntoOwned::into_owned)
            .collect();
        Ok(BrowserLonghandValue::TimelineTriggerSource(timelines))
    })
}

fn parse_clip(source: &str) -> Result<BrowserLonghandValue, EngineError> {
    parse_entire(source, |input| {
        if input
            .try_parse(|input| input.expect_ident_matching("auto"))
            .is_ok()
        {
            return Ok(BrowserLonghandValue::Clip(ClipValue::Auto));
        }

        let location = input.current_source_location();
        let function = input.expect_function()?.clone();
        if !function.eq_ignore_ascii_case("rect") {
            return Err(location.new_custom_error(lightningcss::error::ParserError::InvalidValue));
        }
        let components = input.parse_nested_block(|input| {
            let top = parse_clip_component(input)?;
            let comma_separated = input.try_parse(|input| input.expect_comma()).is_ok();
            let right = parse_clip_component(input)?;
            if comma_separated {
                input.expect_comma()?;
            }
            let bottom = parse_clip_component(input)?;
            if comma_separated {
                input.expect_comma()?;
            }
            let left = parse_clip_component(input)?;
            Ok([top, right, bottom, left])
        })?;
        Ok(BrowserLonghandValue::Clip(ClipValue::Rect(components)))
    })
}

fn parse_clip_component<'i, 't>(
    input: &mut Parser<'i, 't>,
) -> Result<ClipComponent, ParseError<'i>> {
    if input
        .try_parse(|input| input.expect_ident_matching("auto"))
        .is_ok()
    {
        return Ok(ClipComponent::Auto);
    }
    let state = input.state();
    let authored_function = match input.next()?.clone() {
        Token::Function(name) => Some(name.to_string()),
        _ => None,
    };
    input.reset(&state);
    let value = if authored_function.is_some() {
        let calculation = Calc::<Length>::parse_preserving_math_functions(input)?;
        if calculation.resolves_to_number() {
            return Err(input.new_custom_error(lightningcss::error::ParserError::InvalidValue));
        }
        match calculation {
            Calc::Value(value) => *value,
            value => Length::Calc(Box::new(value)),
        }
    } else {
        parse_strict_length(input)?
    };
    Ok(ClipComponent::Length {
        value,
        authored_function,
    })
}

fn parse_dynamic_range_limit(source: &str) -> Result<BrowserLonghandValue, EngineError> {
    parse_entire(source, |input| {
        parse_dynamic_range_limit_value(input).map(BrowserLonghandValue::DynamicRangeLimit)
    })
}

fn parse_dynamic_range_limit_value<'i, 't>(
    input: &mut Parser<'i, 't>,
) -> Result<DynamicRangeLimitValue, ParseError<'i>> {
    if let Ok(keyword) =
        input.try_parse(|input| parse_one_keyword(input, &["standard", "no-limit", "constrained"]))
    {
        return Ok(DynamicRangeLimitValue::Keyword(keyword));
    }

    let location = input.current_source_location();
    let function = input.expect_function()?.clone();
    if !function.eq_ignore_ascii_case("dynamic-range-limit-mix") {
        return Err(location.new_custom_error(lightningcss::error::ParserError::InvalidValue));
    }
    let entries = input.parse_nested_block(|input| {
        let entries = input.parse_comma_separated(|input| {
            let limit = parse_dynamic_range_limit_value(input)?;
            let state = input.state();
            let authored_calculation = matches!(input.next()?.clone(), Token::Function(_));
            input.reset(&state);
            let percentage = if authored_calculation {
                Calc::<Percentage>::parse_preserving_math_functions(input)?
            } else {
                Percentage::parse(input)?.into()
            };
            if !authored_calculation
                && matches!(&percentage, Calc::Value(value) if value.0 < 0.0 || value.0 > 1.0)
            {
                return Err(input.new_custom_error(lightningcss::error::ParserError::InvalidValue));
            }
            Ok(DynamicRangeLimitMixEntry {
                limit,
                percentage,
                authored_calculation,
            })
        })?;
        let all_literal_zero = entries.iter().all(|entry| {
            !entry.authored_calculation
                && matches!(&entry.percentage, Calc::Value(value) if value.0 == 0.0)
        });
        if all_literal_zero {
            return Err(input.new_custom_error(lightningcss::error::ParserError::InvalidValue));
        }
        Ok(entries)
    })?;
    Ok(DynamicRangeLimitValue::Mix(entries))
}

fn parse_animation_trigger(source: &str) -> Result<BrowserLonghandValue, EngineError> {
    parse_entire(source, |input| {
        let values = input.parse_comma_separated(|input| {
            if input
                .try_parse(|input| input.expect_ident_matching("none"))
                .is_ok()
            {
                return Ok(AnimationTriggerValue::None);
            }
            let name = DashedIdent::parse(input)?.into_owned();
            let enter = parse_animation_trigger_behavior(input)?;
            let exit = input.try_parse(parse_animation_trigger_behavior).ok();
            Ok(AnimationTriggerValue::Attachment { name, enter, exit })
        })?;
        Ok(BrowserLonghandValue::AnimationTrigger(values))
    })
}

fn parse_animation_trigger_behavior<'i, 't>(
    input: &mut Parser<'i, 't>,
) -> Result<&'static str, ParseError<'i>> {
    parse_one_keyword(
        input,
        &[
            "play",
            "pause",
            "reset",
            "play-once",
            "play-alternate",
            "play-forwards",
            "play-backwards",
            "play-pause",
            "replay",
            "none",
        ],
    )
}

fn parse_position_area(source: &str) -> Result<BrowserLonghandValue, EngineError> {
    parse_entire(source, |input| {
        parse_position_area_value(input, true).map(BrowserLonghandValue::PositionArea)
    })
}

fn parse_position_area_value<'i, 't>(
    input: &mut Parser<'i, 't>,
    allow_none: bool,
) -> Result<PositionAreaValue, ParseError<'i>> {
    if allow_none
        && input
            .try_parse(|input| input.expect_ident_matching("none"))
            .is_ok()
    {
        return Ok(PositionAreaValue::None);
    }
    let mut first = parse_position_area_keyword(input)?;
    let Some(mut second) = input.try_parse(parse_position_area_keyword).ok() else {
        return Ok(PositionAreaValue::Area {
            first: first.value,
            second: None,
        });
    };
    if matches!(
        first.axis,
        PositionAreaAxis::Vertical | PositionAreaAxis::Inline | PositionAreaAxis::SelfInline
    ) || matches!(
        second.axis,
        PositionAreaAxis::Horizontal | PositionAreaAxis::Block | PositionAreaAxis::SelfBlock
    ) {
        std::mem::swap(&mut first, &mut second);
    }
    if !position_area_pair_is_compatible(first.axis, second.axis) {
        return Err(input.new_custom_error(lightningcss::error::ParserError::InvalidValue));
    }
    if first.value == second.value {
        return Ok(PositionAreaValue::Area {
            first: first.value,
            second: None,
        });
    }
    if first.value == "span-all" && !position_area_value_repeats(second.value) {
        return Ok(PositionAreaValue::Area {
            first: second.value,
            second: None,
        });
    }
    if second.value == "span-all" && !position_area_value_repeats(first.value) {
        return Ok(PositionAreaValue::Area {
            first: first.value,
            second: None,
        });
    }
    Ok(PositionAreaValue::Area {
        first: first.value,
        second: Some(second.value),
    })
}

fn parse_position_area_keyword<'i, 't>(
    input: &mut Parser<'i, 't>,
) -> Result<PositionAreaKeyword, ParseError<'i>> {
    const KEYWORDS: &[(&str, PositionAreaAxis)] = &[
        ("span-all", PositionAreaAxis::General),
        ("center", PositionAreaAxis::General),
        ("left", PositionAreaAxis::Horizontal),
        ("right", PositionAreaAxis::Horizontal),
        ("span-left", PositionAreaAxis::Horizontal),
        ("span-right", PositionAreaAxis::Horizontal),
        ("x-start", PositionAreaAxis::Horizontal),
        ("x-end", PositionAreaAxis::Horizontal),
        ("span-x-start", PositionAreaAxis::Horizontal),
        ("span-x-end", PositionAreaAxis::Horizontal),
        ("self-x-start", PositionAreaAxis::Horizontal),
        ("self-x-end", PositionAreaAxis::Horizontal),
        ("span-self-x-start", PositionAreaAxis::Horizontal),
        ("span-self-x-end", PositionAreaAxis::Horizontal),
        ("top", PositionAreaAxis::Vertical),
        ("bottom", PositionAreaAxis::Vertical),
        ("span-top", PositionAreaAxis::Vertical),
        ("span-bottom", PositionAreaAxis::Vertical),
        ("y-start", PositionAreaAxis::Vertical),
        ("y-end", PositionAreaAxis::Vertical),
        ("span-y-start", PositionAreaAxis::Vertical),
        ("span-y-end", PositionAreaAxis::Vertical),
        ("self-y-start", PositionAreaAxis::Vertical),
        ("self-y-end", PositionAreaAxis::Vertical),
        ("span-self-y-start", PositionAreaAxis::Vertical),
        ("span-self-y-end", PositionAreaAxis::Vertical),
        ("block-start", PositionAreaAxis::Block),
        ("block-end", PositionAreaAxis::Block),
        ("span-block-start", PositionAreaAxis::Block),
        ("span-block-end", PositionAreaAxis::Block),
        ("inline-start", PositionAreaAxis::Inline),
        ("inline-end", PositionAreaAxis::Inline),
        ("span-inline-start", PositionAreaAxis::Inline),
        ("span-inline-end", PositionAreaAxis::Inline),
        ("self-block-start", PositionAreaAxis::SelfBlock),
        ("self-block-end", PositionAreaAxis::SelfBlock),
        ("span-self-block-start", PositionAreaAxis::SelfBlock),
        ("span-self-block-end", PositionAreaAxis::SelfBlock),
        ("self-inline-start", PositionAreaAxis::SelfInline),
        ("self-inline-end", PositionAreaAxis::SelfInline),
        ("span-self-inline-start", PositionAreaAxis::SelfInline),
        ("span-self-inline-end", PositionAreaAxis::SelfInline),
        ("start", PositionAreaAxis::StartEnd),
        ("end", PositionAreaAxis::StartEnd),
        ("span-start", PositionAreaAxis::StartEnd),
        ("span-end", PositionAreaAxis::StartEnd),
        ("self-start", PositionAreaAxis::SelfStartEnd),
        ("self-end", PositionAreaAxis::SelfStartEnd),
        ("span-self-start", PositionAreaAxis::SelfStartEnd),
        ("span-self-end", PositionAreaAxis::SelfStartEnd),
    ];
    let location = input.current_source_location();
    let identifier = input.expect_ident_cloned()?;
    KEYWORDS
        .iter()
        .find_map(|(value, axis)| {
            identifier
                .eq_ignore_ascii_case(value)
                .then_some(PositionAreaKeyword { value, axis: *axis })
        })
        .ok_or_else(|| location.new_custom_error(lightningcss::error::ParserError::InvalidValue))
}

fn position_area_pair_is_compatible(first: PositionAreaAxis, second: PositionAreaAxis) -> bool {
    use PositionAreaAxis::*;
    matches!(first, General)
        || matches!(second, General)
        || matches!(
            (first, second),
            (Horizontal, Vertical)
                | (Block, Inline)
                | (SelfBlock, SelfInline)
                | (StartEnd, StartEnd)
                | (SelfStartEnd, SelfStartEnd)
        )
}

fn position_area_value_repeats(value: &str) -> bool {
    matches!(
        value,
        "span-all"
            | "center"
            | "start"
            | "end"
            | "span-start"
            | "span-end"
            | "self-start"
            | "self-end"
            | "span-self-start"
            | "span-self-end"
    )
}

fn parse_one_keyword<'i, 't>(
    input: &mut Parser<'i, 't>,
    accepted: &'static [&'static str],
) -> Result<&'static str, ParseError<'i>> {
    let location = input.current_source_location();
    let identifier = input.expect_ident_cloned()?;
    accepted
        .iter()
        .copied()
        .find(|candidate| identifier.eq_ignore_ascii_case(candidate))
        .ok_or_else(|| location.new_custom_error(lightningcss::error::ParserError::InvalidValue))
}

fn parse_strict_length<'i, 't>(input: &mut Parser<'i, 't>) -> Result<Length, ParseError<'i>> {
    reject_nonzero_number_token(input)?;
    Length::parse(input)
}

fn parse_strict_length_percentage<'i, 't>(
    input: &mut Parser<'i, 't>,
) -> Result<LengthPercentage, ParseError<'i>> {
    reject_nonzero_number_token(input)?;
    LengthPercentage::parse(input)
}

fn reject_nonzero_number_token<'i, 't>(input: &mut Parser<'i, 't>) -> Result<(), ParseError<'i>> {
    let state = input.state();
    let token = input.next()?.clone();
    input.reset(&state);
    if matches!(token, Token::Number { value, .. } if value != 0.0) {
        return Err(input.new_custom_error(lightningcss::error::ParserError::InvalidValue));
    }
    Ok(())
}

fn reject_top_level_nonzero_numbers(source: &str) -> Result<(), EngineError> {
    parse_entire(source, |input| consume_component_values(input, true))
}

fn consume_component_values<'i, 't>(
    input: &mut Parser<'i, 't>,
    inspect_numbers: bool,
) -> Result<(), ParseError<'i>> {
    while !input.is_exhausted() {
        let token = input.next()?.clone();
        if inspect_numbers && matches!(token, Token::Number { value, .. } if value != 0.0) {
            return Err(input.new_custom_error(lightningcss::error::ParserError::InvalidValue));
        }
        if matches!(
            token,
            Token::Function(_)
                | Token::ParenthesisBlock
                | Token::SquareBracketBlock
                | Token::CurlyBracketBlock
        ) {
            input.parse_nested_block(|input| consume_component_values(input, false))?;
        }
    }
    Ok(())
}

fn serialize_zero_length(value: &Length) -> Result<String, EngineError> {
    let serialized = serialize_typed(value)?;
    if serialized == "0" {
        return Ok("0px".to_owned());
    }
    Ok(serialized)
}

fn serialize_zero_length_percentage(value: &LengthPercentage) -> Result<String, EngineError> {
    let serialized = serialize_typed(value)?;
    if serialized == "0" {
        return Ok("0px".to_owned());
    }
    Ok(serialized)
}

fn serialize_position_component<T: ToCss>(value: &T) -> Result<String, EngineError> {
    let serialized = serialize_typed(value)?;
    if serialized == "0" {
        return Ok("0px".to_owned());
    }
    Ok(serialized)
}

fn serialize_comma_separated<T: ToCss>(values: &[T]) -> Result<String, EngineError> {
    let mut serialized = Vec::with_capacity(values.len());
    for value in values {
        serialized.push(serialize_typed(value)?);
    }
    Ok(serialized.join(", "))
}

fn serialize_space_separated<T: ToCss>(values: &[T]) -> Result<String, EngineError> {
    let mut serialized = Vec::with_capacity(values.len());
    for value in values {
        serialized.push(serialize_typed(value)?);
    }
    Ok(serialized.join(" "))
}

fn serialize_timeline_range_starts(
    values: &[TimelineRangeStartValue],
) -> Result<String, EngineError> {
    values
        .iter()
        .map(|value| match value {
            TimelineRangeStartValue::Auto => Ok("auto".to_owned()),
            TimelineRangeStartValue::Range(value) => serialize_typed(value),
        })
        .collect::<Result<Vec<_>, _>>()
        .map(|values| values.join(", "))
}

fn serialize_timeline_range_ends(values: &[TimelineRangeEndValue]) -> Result<String, EngineError> {
    values
        .iter()
        .map(|value| match value {
            TimelineRangeEndValue::Auto => Ok("auto".to_owned()),
            TimelineRangeEndValue::Range(value) => serialize_typed(value),
        })
        .collect::<Result<Vec<_>, _>>()
        .map(|values| values.join(", "))
}

fn serialize_typed<T: ToCss>(value: &T) -> Result<String, EngineError> {
    value
        .to_css_string(PrinterOptions::default())
        .map_err(|error| EngineError::Serialize(error.to_string()))
}

fn parse_entire<'i, T, F>(source: &'i str, parser: F) -> Result<T, EngineError>
where
    F: for<'t> FnOnce(&mut Parser<'i, 't>) -> Result<T, ParseError<'i>>,
{
    let mut input = ParserInput::new(source);
    let mut css = Parser::new(&mut input);
    css.parse_entirely(parser)
        .map_err(|_| EngineError::Parse("invalid browser longhand value".to_owned()))
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    fn canonical(property: &str, source: &str) -> Result<String, EngineError> {
        parse_browser_longhand(property, source)?
            .ok_or_else(|| EngineError::Parse("missing browser longhand grammar".to_owned()))?
            .canonical_value()
    }

    #[test]
    fn parses_complete_keyword_branches() {
        for (property, source) in [
            ("column-wrap", "nowrap"),
            ("font-kerning", "none"),
            ("overscroll-behavior-x", "contain"),
            ("position-try-order", "most-inline-size"),
            ("scroll-timeline-axis", "x"),
            ("text-box-trim", "trim-both"),
            ("transition-behavior", "allow-discrete"),
            ("white-space-collapse", "break-spaces"),
        ] {
            assert_eq!(canonical(property, source).unwrap(), source);
        }
        assert!(canonical("column-wrap", "balance").is_err());
        assert!(canonical("overscroll-behavior-x", "normal").is_err());
    }

    #[test]
    fn registered_grammars_are_unique_and_parse_every_declared_keyword() {
        let mut properties = HashSet::new();
        for property in REGISTERED_BROWSER_LONGHANDS {
            assert!(
                properties.insert(*property),
                "duplicate grammar for {property}"
            );
            let grammar = grammar(property).unwrap_or_else(|| panic!("missing grammar {property}"));
            match grammar {
                BrowserLonghandGrammar::Keyword(keywords) => {
                    for keyword in keywords {
                        assert_eq!(
                            canonical(property, keyword).unwrap(),
                            *keyword,
                            "{property}"
                        );
                    }
                    assert!(
                        canonical(property, "--not-a-keyword").is_err(),
                        "{property}"
                    );
                }
                BrowserLonghandGrammar::KeywordList(keywords) => {
                    for keyword in keywords {
                        assert_eq!(
                            canonical(property, keyword).unwrap(),
                            *keyword,
                            "{property}"
                        );
                    }
                    if keywords.len() > 1 {
                        let input = format!("{},{}", keywords[0], keywords[1]);
                        let expected = format!("{}, {}", keywords[0], keywords[1]);
                        assert_eq!(canonical(property, &input).unwrap(), expected, "{property}");
                    }
                    assert!(
                        canonical(property, "--not-a-keyword").is_err(),
                        "{property}"
                    );
                }
                _ => {}
            }
        }
    }

    #[test]
    fn composite_browser_contracts_are_owned_by_the_registry() {
        let contract: serde_json::Value = serde_json::from_str(include_str!(
            "../../../compatibility/browser-longhand-composite-contracts.json"
        ))
        .unwrap();
        let properties = contract["properties"].as_array().unwrap();
        let mut branch_count = 0;
        for entry in properties {
            let property = entry["property"].as_str().unwrap();
            assert!(
                grammar(property).is_some(),
                "missing grammar for {property}"
            );
            branch_count += entry["branches"].as_array().unwrap().len();
        }
        assert_eq!(properties.len(), 43);
        assert_eq!(branch_count, 204);
    }

    #[test]
    fn parses_numeric_and_list_branches() {
        assert_eq!(
            canonical("animation-iteration-count", "sign(1em), 1").unwrap(),
            "sign(1em), 1"
        );
        assert_eq!(canonical("column-count", "calc(1 + 1)").unwrap(), "calc(2)");
        assert!(canonical("column-count", "0").is_err());
        assert_eq!(canonical("column-width", "0").unwrap(), "0px");
        assert!(canonical("column-width", "10%").is_err());
        assert_eq!(
            canonical("contain-intrinsic-width", "auto 10px").unwrap(),
            "auto 10px"
        );
        assert_eq!(
            canonical("scroll-timeline-name", "--x,--y").unwrap(),
            "--x, --y"
        );
        assert_eq!(
            canonical("view-timeline-inset", "auto 10px").unwrap(),
            "auto 10px"
        );
        for (source, expected) in [
            ("red", "red"),
            ("light dark", "light dark"),
            ("only dark", "dark only"),
        ] {
            assert_eq!(
                canonical("color-scheme", source).unwrap(),
                expected,
                "{source}"
            );
        }
        for source in [
            "normal dark",
            "only",
            "dark only only",
            "dark only light",
            "1",
        ] {
            assert!(canonical("color-scheme", source).is_err(), "{source}");
        }
        for (source, expected) in [
            ("-10px", "-10px"),
            ("10%", "10%"),
            ("calc(10px + 5%)", "calc(5% + 10px)"),
        ] {
            assert_eq!(
                canonical("column-rule-inset-cap-start", source).unwrap(),
                expected,
                "{source}"
            );
        }
    }

    #[test]
    fn parses_box_geometry_and_interaction_branches() {
        for (property, source, expected) in [
            ("-webkit-locale", "\"en-US\"", "\"en-US\""),
            ("-webkit-perspective-origin-x", "left", "0%"),
            ("-webkit-perspective-origin-y", "bottom", "100%"),
            ("-webkit-transform-origin-x", "center", "50%"),
            ("-webkit-transform-origin-y", "top", "0%"),
            ("-webkit-transform-origin-z", "0", "0px"),
            (
                "-webkit-text-decorations-in-effect",
                "line-through underline",
                "underline line-through",
            ),
            ("contain", "paint layout", "layout paint"),
            (
                "contain",
                "style size paint layout",
                "size layout style paint",
            ),
            ("contain-intrinsic-block-size", "auto none", "auto none"),
            ("contain-intrinsic-inline-size", "auto 10px", "auto 10px"),
            ("object-position", "center", "center center"),
            (
                "object-position",
                "right 10px bottom 20%",
                "right 10px bottom 20%",
            ),
            ("outline-offset", "0", "0px"),
            ("overflow-clip-margin", "padding-box 0", "0px"),
            (
                "overflow-clip-margin",
                "content-box 10px",
                "content-box 10px",
            ),
            ("page-orientation", "rotate-left", "rotate-left"),
            ("ruby-overhang", "none", "spaces"),
            ("scroll-snap-align", "start end", "start end"),
            ("scroll-snap-type", "block proximity", "block"),
            ("scroll-snap-type", "inline mandatory", "inline mandatory"),
            ("scrollbar-gutter", "stable both-edges", "stable both-edges"),
            ("shape-margin", "0", "0px"),
            ("text-fit", "grow", "grow"),
            ("text-underline-offset", "auto", "auto"),
            (
                "text-underline-position",
                "right from-font",
                "from-font right",
            ),
            (
                "touch-action",
                "pinch-zoom pan-y pan-left",
                "pan-left pan-y pinch-zoom",
            ),
        ] {
            let actual = canonical(property, source);
            assert_eq!(
                actual.as_deref(),
                Ok(expected),
                "{property}: {source}: {actual:?}"
            );
        }

        for (property, source) in [
            ("-webkit-locale", "en-US"),
            ("-webkit-perspective-origin-x", "top"),
            ("-webkit-transform-origin-z", "20%"),
            (
                "-webkit-text-decorations-in-effect",
                "underline spelling-error",
            ),
            ("contain", "strict paint"),
            ("contain", "size inline-size"),
            ("contain-intrinsic-block-size", "20%"),
            ("object-position", "left right"),
            ("outline-offset", "20%"),
            ("overflow-clip-margin", "0"),
            ("scroll-snap-align", "start end center"),
            ("scroll-snap-type", "mandatory"),
            ("scrollbar-gutter", "both-edges"),
            ("shape-margin", "-1px"),
            ("text-underline-offset", "from-font"),
            ("text-underline-position", "under left right"),
            ("touch-action", "pan-left pan-right"),
        ] {
            assert!(canonical(property, source).is_err(), "{property}: {source}");
        }
    }

    #[test]
    fn parses_names_counters_scopes_and_string_pairs() {
        for (property, source, expected) in [
            ("anchor-name", "--a,--b", "--a, --b"),
            ("anchor-scope", "all", "all"),
            ("timeline-scope", "--timeline", "--timeline"),
            ("trigger-scope", "--a,--b", "--a, --b"),
            ("view-transition-scope", "all", "all"),
            ("position-anchor", "--anchor", "--anchor"),
            (
                "position-visibility",
                "no-overflow anchors-visible",
                "anchors-visible no-overflow",
            ),
            ("counter-increment", "chapter", "chapter 1"),
            ("counter-increment", "foo 0 bar", "foo 0 bar 1"),
            ("counter-reset", "chapter", "chapter 0"),
            ("counter-set", "chapter -1 item 3", "chapter -1 item 3"),
            ("page", "chapter-1", "chapter-1"),
            (
                "quotes",
                "\"«\" \"»\" \"‹\" \"›\"",
                "\"«\" \"»\" \"‹\" \"›\"",
            ),
            ("paint-order", "stroke fill", "stroke"),
            ("paint-order", "markers stroke", "markers stroke"),
            ("text-autospace", "no-autospace", "no-autospace"),
            ("will-change", "transform,opacity", "transform, opacity"),
            ("hyphenate-character", "\"ab\"", "\"ab\""),
        ] {
            assert_eq!(canonical(property, source).unwrap(), expected, "{property}");
        }

        for (property, source) in [
            ("anchor-name", "--a --b"),
            ("anchor-scope", "all --a"),
            ("timeline-scope", "all"),
            ("trigger-scope", "foo"),
            ("view-transition-scope", "--a"),
            ("position-anchor", "--a, --b"),
            ("position-visibility", "always no-overflow"),
            ("counter-increment", "reversed(chapter)"),
            ("counter-reset", "chapter 1.5"),
            ("counter-set", "none chapter"),
            ("page", "first left"),
            ("quotes", "\"a\" \"b\" \"c\""),
            ("paint-order", "fill fill"),
            ("text-autospace", "auto"),
            ("will-change", "will-change"),
            ("hyphenate-character", "-"),
        ] {
            assert!(canonical(property, source).is_err(), "{property}: {source}");
        }
    }

    #[test]
    fn parses_clip_mixes_triggers_and_position_areas() {
        for (property, source, expected) in [
            (
                "clip",
                "rect(auto 1px 2px 3px)",
                "rect(auto, 1px, 2px, 3px)",
            ),
            ("clip", "rect(0 0 0 0)", "rect(0px, 0px, 0px, 0px)"),
            (
                "dynamic-range-limit",
                "dynamic-range-limit-mix(standard calc(50%), no-limit 50%)",
                "dynamic-range-limit-mix(standard calc(50%), no-limit 50%)",
            ),
            (
                "dynamic-range-limit",
                "dynamic-range-limit-mix(standard min(20%, 30%), constrained 80%)",
                "dynamic-range-limit-mix(standard min(20%, 30%), constrained 80%)",
            ),
            (
                "animation-trigger",
                "--x play pause,--y reset",
                "--x play pause, --y reset",
            ),
            ("position-area", "bottom right", "right bottom"),
            (
                "position-area",
                "inline-end block-start",
                "block-start inline-end",
            ),
            ("position-area", "span-all top", "top"),
            ("position-area", "start start", "start"),
        ] {
            let actual = canonical(property, source);
            assert_eq!(
                actual.as_deref(),
                Ok(expected),
                "{property}: {source}: {actual:?}"
            );
        }

        for (property, source) in [
            ("clip", "rect(1px, 2px 3px, 4px)"),
            ("clip", "inset(1px)"),
            (
                "dynamic-range-limit",
                "dynamic-range-limit-mix(standard 0%, no-limit 0%)",
            ),
            (
                "dynamic-range-limit",
                "dynamic-range-limit-mix(standard 101%)",
            ),
            ("animation-trigger", "--x play pause reset"),
            ("animation-trigger", "foo play"),
            ("position-area", "left right"),
            ("position-area", "block-start block-end"),
        ] {
            assert!(canonical(property, source).is_err(), "{property}: {source}");
        }
    }

    #[test]
    fn parses_font_branches_and_rejects_adjacent_invalid_values() {
        for (property, source, expected) in [
            ("font-feature-settings", "\"liga\" on", "\"liga\""),
            (
                "font-feature-settings",
                "\"liga\" off, \"kern\" 2",
                "\"liga\" 0, \"kern\" 2",
            ),
            ("font-language-override", "\"ENG\"", "\"ENG\""),
            ("font-size-adjust", "ex-height from-font", "from-font"),
            ("font-size-adjust", "cap-height 0.5", "cap-height .5"),
            (
                "font-variant-alternates",
                "annotation(note) swash(flow) character-variant(cv) styleset(foo,bar) historical-forms stylistic(alt)",
                "stylistic(alt) historical-forms styleset(foo, bar) character-variant(cv) swash(flow) annotation(note)",
            ),
            (
                "font-variant-east-asian",
                "ruby full-width jis78",
                "jis78 full-width ruby",
            ),
            (
                "font-variant-ligatures",
                "no-common-ligatures discretionary-ligatures",
                "no-common-ligatures discretionary-ligatures",
            ),
            (
                "font-variant-numeric",
                "ordinal slashed-zero tabular-nums",
                "ordinal slashed-zero tabular-nums",
            ),
            (
                "font-variation-settings",
                "\"wght\" calc(1 + 1)",
                "\"wght\" calc(2)",
            ),
        ] {
            assert_eq!(canonical(property, source).unwrap(), expected, "{property}");
        }

        for (property, source) in [
            ("font-feature-settings", "\"abcde\""),
            ("font-feature-settings", "\"liga\" 1.5"),
            ("font-language-override", "\"ABCDE\""),
            ("font-size-adjust", "-1"),
            ("font-variant-alternates", "stylistic(foo) stylistic(bar)"),
            ("font-variant-east-asian", "jis78 jis83"),
            (
                "font-variant-ligatures",
                "common-ligatures no-common-ligatures",
            ),
            ("font-variant-numeric", "lining-nums oldstyle-nums"),
            ("font-variation-settings", "\"a\" 1"),
        ] {
            assert!(canonical(property, source).is_err(), "{property}: {source}");
        }
    }

    #[test]
    fn parses_position_and_timeline_branches() {
        for (property, source, expected) in [
            (
                "position-try-fallbacks",
                "--foo, flip-block flip-inline",
                "--foo, flip-block flip-inline",
            ),
            ("position-try-fallbacks", "center", "center"),
            ("position-try-fallbacks", "left top", "left top"),
            ("text-box-edge", "text text", "text"),
            ("text-box-edge", "cap alphabetic", "cap alphabetic"),
            (
                "timeline-trigger-activation-range-start",
                "cover 10%",
                "cover 10%",
            ),
            (
                "timeline-trigger-activation-range-start",
                "scroll",
                "scroll",
            ),
            ("timeline-trigger-active-range-start", "auto", "auto"),
            ("timeline-trigger-source", "--foo,--bar", "--foo, --bar"),
            (
                "timeline-trigger-source",
                "scroll(block root), view(block 1px)",
                "scroll(root), view(1px)",
            ),
            (
                "timeline-trigger-active-range-start",
                "auto, cover 10%",
                "auto, cover 10%",
            ),
            (
                "timeline-trigger-activation-range-end",
                "normal, entry-crossing 1px",
                "normal, entry-crossing 1px",
            ),
        ] {
            assert_eq!(canonical(property, source).unwrap(), expected, "{property}");
        }
        for (property, source) in [
            ("position-try-fallbacks", "flip-block flip-block"),
            ("text-box-edge", "cap"),
            ("timeline-trigger-activation-range-start", "auto"),
            ("timeline-trigger-activation-range-start", "red"),
            ("timeline-trigger-source", "foo"),
            ("timeline-trigger-source", "scroll(root root)"),
            ("timeline-trigger-active-range-start", "auto,"),
        ] {
            assert!(canonical(property, source).is_err(), "{property}: {source}");
        }
    }

    #[test]
    fn parses_every_corner_shape_branch() {
        for source in [
            "round",
            "bevel",
            "scoop",
            "notch",
            "square",
            "squircle",
            "superellipse(-1)",
        ] {
            assert_eq!(canonical("corner-top-left-shape", source).unwrap(), source);
        }
        assert!(canonical("corner-top-left-shape", "superellipse()").is_err());
    }
}
