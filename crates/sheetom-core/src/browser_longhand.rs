use cssparser::{Parser, ParserInput};
use lightningcss::{
    properties::{
        animation::{AnimationRangeEnd, AnimationRangeStart},
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
    DashedIdentList(Option<Vec<DashedIdent<'static>>>),
    TimeOrNormal(Option<Time>),
    ViewTimelineInset(Vec<ViewTimelineInsetValue>),
    CornerShape(CornerShapeValue),
    FontFeatureSettings(Option<Vec<FontFeatureSetting>>),
    FontLanguageOverride(Option<CSSString<'static>>),
    FontSizeAdjust(FontSizeAdjustValue),
    FontVariantAlternates(FontVariantAlternatesValue),
    FontVariantKeywords(Vec<&'static str>),
    FontVariationSettings(Option<Vec<FontVariationSetting>>),
    PositionTryFallbacks(Option<Vec<PositionTryFallback>>),
    TextBoxEdge(TextBoxEdgeValue),
    TimelineRangeStart(TimelineRangeStartValue),
    TimelineRangeEnd(TimelineRangeEndValue),
    TimelineTriggerSource(TimelineTriggerSourceValue),
    KeywordList(Vec<&'static str>),
    AnimationIterationCount(Vec<AnimationIterationCountValue>),
    OffsetPath(OffsetPathValue),
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
pub enum TimelineTriggerSourceValue {
    Auto,
    None,
    Names(Vec<DashedIdent<'static>>),
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

impl BrowserLonghandValue {
    pub(crate) fn canonical_value(&self) -> Result<String, EngineError> {
        match self {
            BrowserLonghandValue::Keyword(value) => Ok((*value).to_owned()),
            BrowserLonghandValue::Length(value) => serialize_zero_length(value),
            BrowserLonghandValue::LengthPercentage(value) => serialize_typed(value),
            BrowserLonghandValue::AutoLength(None) => Ok("auto".to_owned()),
            BrowserLonghandValue::AutoLength(Some(value)) => serialize_zero_length(value),
            BrowserLonghandValue::ColumnCount(value) => value.canonical_value(),
            BrowserLonghandValue::ContainIntrinsic(value) => value.canonical_value(),
            BrowserLonghandValue::DashedIdentList(None) => Ok("none".to_owned()),
            BrowserLonghandValue::DashedIdentList(Some(values)) => {
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
                    components.extend(value.tactics.iter().map(|value| (*value).to_owned()));
                    output.push(components.join(" "));
                }
                Ok(output.join(", "))
            }
            BrowserLonghandValue::TextBoxEdge(value) => value.canonical_value(),
            BrowserLonghandValue::TimelineRangeStart(TimelineRangeStartValue::Auto) => {
                Ok("auto".to_owned())
            }
            BrowserLonghandValue::TimelineRangeStart(TimelineRangeStartValue::Range(value)) => {
                serialize_typed(value)
            }
            BrowserLonghandValue::TimelineRangeEnd(TimelineRangeEndValue::Auto) => {
                Ok("auto".to_owned())
            }
            BrowserLonghandValue::TimelineRangeEnd(TimelineRangeEndValue::Range(value)) => {
                serialize_typed(value)
            }
            BrowserLonghandValue::TimelineTriggerSource(TimelineTriggerSourceValue::Auto) => {
                Ok("auto".to_owned())
            }
            BrowserLonghandValue::TimelineTriggerSource(TimelineTriggerSourceValue::None) => {
                Ok("none".to_owned())
            }
            BrowserLonghandValue::TimelineTriggerSource(TimelineTriggerSourceValue::Names(
                values,
            )) => serialize_comma_separated(values),
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
        if self.historical_forms {
            output.push("historical-forms".to_owned());
        }
        for function in &self.functions {
            let values = serialize_comma_separated(&function.values)?;
            output.push(format!("{}({values})", function.name));
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
        Some(BrowserLonghandGrammar::Length { non_negative }) => parse_entire(source, |input| {
            let value = Length::parse(input)?;
            if non_negative && value.try_sign().is_some_and(|sign| sign < 0.0) {
                return Err(input.new_custom_error(lightningcss::error::ParserError::InvalidValue));
            }
            Ok(BrowserLonghandValue::Length(value))
        }),
        Some(BrowserLonghandGrammar::LengthPercentage) => parse_entire(source, |input| {
            LengthPercentage::parse(input).map(BrowserLonghandValue::LengthPercentage)
        }),
        Some(BrowserLonghandGrammar::AutoLength) => parse_auto_length(source),
        Some(BrowserLonghandGrammar::ColumnCount) => parse_column_count(source),
        Some(BrowserLonghandGrammar::ContainIntrinsic) => parse_contain_intrinsic(source),
        Some(BrowserLonghandGrammar::DashedIdentList) => parse_dashed_ident_list(source),
        Some(BrowserLonghandGrammar::TimeOrNormal) => parse_time_or_normal(source),
        Some(BrowserLonghandGrammar::ViewTimelineInset) => parse_view_timeline_inset(source),
        Some(BrowserLonghandGrammar::CornerShape) => parse_corner_shape(source),
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
        Some(BrowserLonghandGrammar::TimelineTriggerSource) => {
            parse_timeline_trigger_source(source)
        }
        Some(BrowserLonghandGrammar::KeywordList(keywords)) => parse_keyword_list(source, keywords),
        Some(BrowserLonghandGrammar::AnimationIterationCount) => {
            parse_animation_iteration_count(source)
        }
        Some(BrowserLonghandGrammar::OffsetPath) => parse_offset_path(source),
        None => return Ok(None),
    }?;
    Ok(Some(value))
}

#[derive(Clone, Copy)]
enum BrowserLonghandGrammar {
    Keyword(&'static [&'static str]),
    Length { non_negative: bool },
    LengthPercentage,
    AutoLength,
    ColumnCount,
    ContainIntrinsic,
    DashedIdentList,
    TimeOrNormal,
    ViewTimelineInset,
    CornerShape,
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
    TimelineTriggerSource,
    KeywordList(&'static [&'static str]),
    AnimationIterationCount,
    OffsetPath,
}

fn grammar(property_name: &str) -> Option<BrowserLonghandGrammar> {
    let grammar = match property_name {
        "-webkit-border-horizontal-spacing"
        | "-webkit-border-vertical-spacing"
        | "column-rule-inset-cap-end"
        | "column-rule-inset-cap-start"
        | "column-rule-inset-junction-end"
        | "column-rule-inset-junction-start"
        | "row-rule-inset-cap-end"
        | "row-rule-inset-cap-start"
        | "row-rule-inset-junction-end"
        | "row-rule-inset-junction-start" => BrowserLonghandGrammar::Length { non_negative: true },
        "offset-distance" => BrowserLonghandGrammar::LengthPercentage,
        "column-height" | "column-width" => BrowserLonghandGrammar::AutoLength,
        "column-count" => BrowserLonghandGrammar::ColumnCount,
        "contain-intrinsic-height" | "contain-intrinsic-width" => {
            BrowserLonghandGrammar::ContainIntrinsic
        }
        "interest-delay-end" | "interest-delay-start" => BrowserLonghandGrammar::TimeOrNormal,
        "scroll-timeline-name" | "timeline-trigger-name" | "view-timeline-name" => {
            BrowserLonghandGrammar::DashedIdentList
        }
        "view-timeline-inset" => BrowserLonghandGrammar::ViewTimelineInset,
        "corner-bottom-left-shape"
        | "corner-bottom-right-shape"
        | "corner-end-end-shape"
        | "corner-end-start-shape"
        | "corner-start-end-shape"
        | "corner-start-start-shape"
        | "corner-top-left-shape"
        | "corner-top-right-shape" => BrowserLonghandGrammar::CornerShape,
        "font-feature-settings" => BrowserLonghandGrammar::FontFeatureSettings,
        "font-language-override" => BrowserLonghandGrammar::FontLanguageOverride,
        "font-size-adjust" => BrowserLonghandGrammar::FontSizeAdjust,
        "font-variant-alternates" => BrowserLonghandGrammar::FontVariantAlternates,
        "font-variant-east-asian" | "font-variant-ligatures" | "font-variant-numeric" => {
            BrowserLonghandGrammar::FontVariantKeywords
        }
        "font-variation-settings" => BrowserLonghandGrammar::FontVariationSettings,
        "position-try-fallbacks" => BrowserLonghandGrammar::PositionTryFallbacks,
        "text-box-edge" => BrowserLonghandGrammar::TextBoxEdge,
        "timeline-trigger-activation-range-start" => {
            BrowserLonghandGrammar::TimelineRangeStart { auto: false }
        }
        "timeline-trigger-active-range-start" => {
            BrowserLonghandGrammar::TimelineRangeStart { auto: true }
        }
        "timeline-trigger-activation-range-end" => {
            BrowserLonghandGrammar::TimelineRangeEnd { auto: false }
        }
        "timeline-trigger-active-range-end" => {
            BrowserLonghandGrammar::TimelineRangeEnd { auto: true }
        }
        "timeline-trigger-source" => BrowserLonghandGrammar::TimelineTriggerSource,
        "animation-iteration-count" | "-webkit-animation-iteration-count" => {
            BrowserLonghandGrammar::AnimationIterationCount
        }
        "offset-path" => BrowserLonghandGrammar::OffsetPath,
        "scroll-timeline-axis" | "view-timeline-axis" => {
            BrowserLonghandGrammar::KeywordList(&["block", "inline", "x", "y"])
        }
        "transition-behavior" => BrowserLonghandGrammar::KeywordList(&["normal", "allow-discrete"]),
        "text-wrap-style" => {
            BrowserLonghandGrammar::Keyword(&["auto", "balance", "stable", "pretty"])
        }
        name => BrowserLonghandGrammar::Keyword(keyword_grammar(name)?),
    };
    Some(grammar)
}

fn keyword_grammar(property_name: &str) -> Option<&'static [&'static str]> {
    let values: &'static [&'static str] = match property_name {
        "column-rule-break" | "row-rule-break" => &["normal", "none"],
        "column-rule-visibility-items" | "row-rule-visibility-items" => &["normal", "all"],
        "column-wrap" => &["auto", "wrap", "nowrap"],
        "font-kerning" => &["auto", "normal", "none"],
        "font-optical-sizing"
        | "font-synthesis-small-caps"
        | "font-synthesis-style"
        | "font-synthesis-weight" => &["auto", "none"],
        "font-variant-emoji" => &["normal", "text", "emoji", "unicode"],
        "font-variant-position" => &["normal", "sub", "super"],
        "overscroll-behavior-x" | "overscroll-behavior-y" => &["auto", "contain", "none"],
        "position-try-order" => &[
            "normal",
            "most-width",
            "most-height",
            "most-block-size",
            "most-inline-size",
        ],
        "text-box-trim" => &["none", "trim-start", "trim-end", "trim-both"],
        "text-wrap-mode" => &["wrap", "nowrap"],
        "white-space-collapse" => &["collapse", "preserve", "preserve-breaks", "break-spaces"],
        _ => return None,
    };
    Some(values)
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
        let value = Length::parse(input)?;
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
        let value = Length::parse(input)?;
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

fn parse_dashed_ident_list(source: &str) -> Result<BrowserLonghandValue, EngineError> {
    parse_entire(source, |input| {
        if input
            .try_parse(|input| input.expect_ident_matching("none"))
            .is_ok()
        {
            return Ok(BrowserLonghandValue::DashedIdentList(None));
        }
        let values = input
            .parse_comma_separated(DashedIdent::parse)?
            .into_iter()
            .map(IntoOwned::into_owned)
            .collect();
        Ok(BrowserLonghandValue::DashedIdentList(Some(values)))
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
                    LengthPercentage::parse(input)?,
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
            values.push(keyword);
        }
        if values.is_empty() {
            return Err(input.new_custom_error(lightningcss::error::ParserError::InvalidValue));
        }
        Ok(BrowserLonghandValue::FontVariantKeywords(values))
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
            let name = input
                .try_parse(DashedIdent::parse)
                .ok()
                .map(IntoOwned::into_owned);
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
            if name.is_none() && tactics.is_empty() {
                return Err(input.new_custom_error(lightningcss::error::ParserError::InvalidValue));
            }
            Ok(PositionTryFallback { name, tactics })
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
        if accepts_auto
            && input
                .try_parse(|input| input.expect_ident_matching("auto"))
                .is_ok()
        {
            return Ok(BrowserLonghandValue::TimelineRangeStart(
                TimelineRangeStartValue::Auto,
            ));
        }
        AnimationRangeStart::parse(input)
            .map(TimelineRangeStartValue::Range)
            .map(BrowserLonghandValue::TimelineRangeStart)
    })
}

fn parse_timeline_range_end(
    source: &str,
    accepts_auto: bool,
) -> Result<BrowserLonghandValue, EngineError> {
    parse_entire(source, |input| {
        if accepts_auto
            && input
                .try_parse(|input| input.expect_ident_matching("auto"))
                .is_ok()
        {
            return Ok(BrowserLonghandValue::TimelineRangeEnd(
                TimelineRangeEndValue::Auto,
            ));
        }
        AnimationRangeEnd::parse(input)
            .map(TimelineRangeEndValue::Range)
            .map(BrowserLonghandValue::TimelineRangeEnd)
    })
}

fn parse_timeline_trigger_source(source: &str) -> Result<BrowserLonghandValue, EngineError> {
    parse_entire(source, |input| {
        if input
            .try_parse(|input| input.expect_ident_matching("auto"))
            .is_ok()
        {
            return Ok(BrowserLonghandValue::TimelineTriggerSource(
                TimelineTriggerSourceValue::Auto,
            ));
        }
        if input
            .try_parse(|input| input.expect_ident_matching("none"))
            .is_ok()
        {
            return Ok(BrowserLonghandValue::TimelineTriggerSource(
                TimelineTriggerSourceValue::None,
            ));
        }
        let names = input
            .parse_comma_separated(DashedIdent::parse)?
            .into_iter()
            .map(IntoOwned::into_owned)
            .collect();
        Ok(BrowserLonghandValue::TimelineTriggerSource(
            TimelineTriggerSourceValue::Names(names),
        ))
    })
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

fn serialize_zero_length(value: &Length) -> Result<String, EngineError> {
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
                "historical-forms styleset(foo,bar)",
                "historical-forms styleset(foo, bar)",
            ),
            (
                "font-variant-east-asian",
                "jis78 full-width ruby",
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
            ("text-box-edge", "text text", "text"),
            ("text-box-edge", "cap alphabetic", "cap alphabetic"),
            (
                "timeline-trigger-activation-range-start",
                "cover 10%",
                "cover 10%",
            ),
            ("timeline-trigger-active-range-start", "auto", "auto"),
            ("timeline-trigger-source", "--foo,--bar", "--foo, --bar"),
        ] {
            assert_eq!(canonical(property, source).unwrap(), expected, "{property}");
        }
        for (property, source) in [
            ("position-try-fallbacks", "flip-block flip-block"),
            ("text-box-edge", "cap"),
            ("timeline-trigger-activation-range-start", "auto"),
            ("timeline-trigger-source", "foo"),
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
