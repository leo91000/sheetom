use cssparser::{Parser, ParserInput, Token};
use lightningcss::{
    properties::{
        border_image::{BorderImageRepeat, BorderImageSideWidth, BorderImageSlice},
        list::{CounterStyle, PredefinedCounterStyle},
    },
    stylesheet::PrinterOptions,
    traits::{IntoOwned, Parse, Sign, ToCss},
    values::{
        angle::Angle,
        calc::Calc,
        ident::CustomIdent,
        image::Image,
        length::{
            Length, LengthOrNumber, LengthPercentage, LengthValue, PreservedLengthPercentage,
        },
        number::CSSNumber,
        percentage::{DimensionPercentage, Percentage},
        position::{HorizontalPosition, Position, PositionComponent, VerticalPosition},
        rect::Rect,
        string::CSSString,
    },
};

use crate::{
    browser_longhand::{parse_browser_longhand, BrowserLonghandValue},
    catalog::PropertyGrammarExtension,
    geometric_value::{parse_geometric_property, GeometricValue},
    shorthand::canonicalize_webkit_border_image,
    syntax::{split_top_level_delimiter, split_top_level_whitespace},
    EngineError,
};

#[derive(Clone, Debug, PartialEq)]
pub enum SemanticExtensionValue {
    AspectRatio(AspectRatioValue),
    BrowserLonghand(BrowserLonghandValue),
    Content(ContentValue),
    Geometric(Box<GeometricValue>),
    IntegerCalculation(IntegerCalculationValue),
    CrossDimensionCalculation(CrossDimensionCalculationValue),
    OffsetPosition(OffsetPositionValue),
    OffsetRotate(OffsetRotateValue),
    PageSize(PageSizeValue),
    WebkitBorderImage(WebkitBorderImageValue),
    WebkitBoxReflect(WebkitBoxReflectValue),
    WebkitMaskBoxImageSlice(WebkitBorderImageValue),
    WebkitPerspective(WebkitPerspectiveValue),
}

impl SemanticExtensionValue {
    pub fn canonical_value(&self) -> Result<String, EngineError> {
        match self {
            SemanticExtensionValue::AspectRatio(value) => value.canonical_value(),
            SemanticExtensionValue::BrowserLonghand(value) => value.canonical_value(),
            SemanticExtensionValue::Content(value) => value.canonical_value(),
            SemanticExtensionValue::Geometric(value) => value.canonical_value(),
            SemanticExtensionValue::IntegerCalculation(value) => value.canonical_value(),
            SemanticExtensionValue::CrossDimensionCalculation(value) => value.canonical_value(),
            SemanticExtensionValue::OffsetPosition(value) => value.canonical_value(),
            SemanticExtensionValue::OffsetRotate(value) => value.canonical_value(),
            SemanticExtensionValue::PageSize(value) => value.canonical_value(),
            SemanticExtensionValue::WebkitBorderImage(value) => value.canonical_value(),
            SemanticExtensionValue::WebkitBoxReflect(value) => value.canonical_value(),
            SemanticExtensionValue::WebkitMaskBoxImageSlice(value) => value.canonical_value(),
            SemanticExtensionValue::WebkitPerspective(value) => value.canonical_value(),
        }
    }

    pub(crate) fn retains_context_dependent_math(&self) -> bool {
        match self {
            SemanticExtensionValue::AspectRatio(value) => value.retains_context_dependent_math(),
            SemanticExtensionValue::CrossDimensionCalculation(value) => {
                value.retains_context_dependent_math()
            }
            SemanticExtensionValue::WebkitBorderImage(value) => {
                value.retains_context_dependent_math()
            }
            SemanticExtensionValue::WebkitMaskBoxImageSlice(value) => {
                value.retains_context_dependent_math()
            }
            SemanticExtensionValue::WebkitPerspective(_) => false,
            _ => false,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum ContentValue {
    Normal,
    None,
    List {
        items: Vec<ContentItem>,
        alternative: Vec<ContentAlternative>,
    },
}

impl ContentValue {
    fn canonical_value(&self) -> Result<String, EngineError> {
        match self {
            ContentValue::Normal => Ok("normal".to_owned()),
            ContentValue::None => Ok("none".to_owned()),
            ContentValue::List { items, alternative } => {
                let mut output = serialize_space_separated(items)?;
                if !alternative.is_empty() {
                    output.push_str(" / ");
                    output.push_str(&serialize_space_separated(alternative)?);
                }
                Ok(output)
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum ContentItem {
    Image(Image<'static>),
    String(CSSString<'static>),
    Quote(ContentQuote),
    Counter(ContentCounter),
}

impl ToCss for ContentItem {
    fn to_css<W>(
        &self,
        dest: &mut lightningcss::printer::Printer<W>,
    ) -> Result<(), lightningcss::error::PrinterError>
    where
        W: std::fmt::Write,
    {
        match self {
            ContentItem::Image(value) => value.to_css(dest),
            ContentItem::String(value) => lightningcss::traits::ToCss::to_css(value, dest),
            ContentItem::Quote(value) => value.to_css(dest),
            ContentItem::Counter(value) => value.to_css(dest),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum ContentAlternative {
    String(CSSString<'static>),
    Counter(ContentCounter),
}

impl ToCss for ContentAlternative {
    fn to_css<W>(
        &self,
        dest: &mut lightningcss::printer::Printer<W>,
    ) -> Result<(), lightningcss::error::PrinterError>
    where
        W: std::fmt::Write,
    {
        match self {
            ContentAlternative::String(value) => lightningcss::traits::ToCss::to_css(value, dest),
            ContentAlternative::Counter(value) => value.to_css(dest),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ContentQuote {
    Open,
    Close,
    NoOpen,
    NoClose,
}

impl ToCss for ContentQuote {
    fn to_css<W>(
        &self,
        dest: &mut lightningcss::printer::Printer<W>,
    ) -> Result<(), lightningcss::error::PrinterError>
    where
        W: std::fmt::Write,
    {
        dest.write_str(match self {
            ContentQuote::Open => "open-quote",
            ContentQuote::Close => "close-quote",
            ContentQuote::NoOpen => "no-open-quote",
            ContentQuote::NoClose => "no-close-quote",
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ContentCounter {
    name: CustomIdent<'static>,
    separator: Option<CSSString<'static>>,
    style: CounterStyle<'static>,
}

impl ToCss for ContentCounter {
    fn to_css<W>(
        &self,
        dest: &mut lightningcss::printer::Printer<W>,
    ) -> Result<(), lightningcss::error::PrinterError>
    where
        W: std::fmt::Write,
    {
        let function = if self.separator.is_some() {
            "counters("
        } else {
            "counter("
        };
        dest.write_str(function)?;
        self.name.to_css(dest)?;
        if let Some(separator) = &self.separator {
            dest.delim(',', false)?;
            lightningcss::traits::ToCss::to_css(separator, dest)?;
        }
        if !matches!(
            self.style,
            CounterStyle::Predefined(PredefinedCounterStyle::Decimal)
        ) {
            dest.delim(',', false)?;
            self.style.to_css(dest)?;
        }
        dest.write_char(')')
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WebkitBoxReflectDirection {
    Above,
    Below,
    Left,
    Right,
}

impl ToCss for WebkitBoxReflectDirection {
    fn to_css<W>(
        &self,
        dest: &mut lightningcss::printer::Printer<W>,
    ) -> Result<(), lightningcss::error::PrinterError>
    where
        W: std::fmt::Write,
    {
        dest.write_str(match self {
            WebkitBoxReflectDirection::Above => "above",
            WebkitBoxReflectDirection::Below => "below",
            WebkitBoxReflectDirection::Left => "left",
            WebkitBoxReflectDirection::Right => "right",
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct WebkitBoxReflectValue {
    direction: WebkitBoxReflectDirection,
    offset: LengthPercentage,
    mask: Option<WebkitReflectMaskValue>,
}

impl WebkitBoxReflectValue {
    fn canonical_value(&self) -> Result<String, EngineError> {
        let mut output = serialize_typed(&self.direction)?;
        output.push(' ');
        let offset = serialize_typed(&self.offset)?;
        output.push_str(if offset == "0" { "0px" } else { &offset });
        if let Some(mask) = &self.mask {
            output.push(' ');
            output.push_str(&serialize_typed(mask)?);
        }
        Ok(output)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct WebkitReflectMaskValue {
    source: Option<Image<'static>>,
    slice: Option<BorderImageSlice>,
    width: Option<Rect<BorderImageSideWidth>>,
    outset: Option<Rect<LengthOrNumber>>,
    repeat: Option<BorderImageRepeat>,
}

type PropertyParseError<'i> = cssparser::ParseError<'i, lightningcss::error::ParserError<'i>>;
type WebkitReflectWidthOutset = (
    Option<Rect<BorderImageSideWidth>>,
    Option<Rect<LengthOrNumber>>,
);

impl ToCss for WebkitReflectMaskValue {
    fn to_css<W>(
        &self,
        dest: &mut lightningcss::printer::Printer<W>,
    ) -> Result<(), lightningcss::error::PrinterError>
    where
        W: std::fmt::Write,
    {
        let mut wrote_value = false;
        if let Some(source) = &self.source {
            source.to_css(dest)?;
            wrote_value = true;
        }
        if let Some(slice) = &self.slice {
            if wrote_value {
                dest.write_str(" ")?;
            }
            let mut slice = slice.clone();
            slice.fill = true;
            slice.to_css(dest)?;
            wrote_value = true;
        }
        if let Some(width) = &self.width {
            dest.delim('/', true)?;
            width.to_css(dest)?;
            if let Some(outset) = &self.outset {
                dest.delim('/', true)?;
                outset.to_css(dest)?;
            }
        } else if let Some(outset) = &self.outset {
            dest.delim('/', true)?;
            outset.to_css(dest)?;
        }
        if let Some(repeat) = &self.repeat {
            if wrote_value || self.width.is_some() || self.outset.is_some() {
                dest.write_str(" ")?;
            }
            repeat.to_css(dest)?;
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct AspectRatioValue {
    numerator: Calc<PreservedLengthPercentage>,
    denominator: Calc<PreservedLengthPercentage>,
}

impl AspectRatioValue {
    fn canonical_value(&self) -> Result<String, EngineError> {
        Ok(format!(
            "{} / {}",
            serialize_typed(&self.numerator)?,
            serialize_typed(&self.denominator)?
        ))
    }

    fn retains_context_dependent_math(&self) -> bool {
        self.numerator.contains_unresolved_sign() || self.denominator.contains_unresolved_sign()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum WebkitBorderImageValue {
    Compound(String),
    Slice(Calc<PreservedLengthPercentage>),
}

impl WebkitBorderImageValue {
    fn canonical_value(&self) -> Result<String, EngineError> {
        match self {
            WebkitBorderImageValue::Compound(value) => Ok(value.clone()),
            WebkitBorderImageValue::Slice(slice) => Ok(format!("{} fill", serialize_typed(slice)?)),
        }
    }

    fn retains_context_dependent_math(&self) -> bool {
        match self {
            WebkitBorderImageValue::Compound(_) => true,
            WebkitBorderImageValue::Slice(slice) => slice.contains_unresolved_sign(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum WebkitPerspectiveValue {
    None,
    DirectNumber(CSSNumber),
    DirectLength(Length),
    NumberCalculation(Calc<PreservedLengthPercentage>),
    LengthCalculation(Calc<Length>),
}

impl WebkitPerspectiveValue {
    fn canonical_value(&self) -> Result<String, EngineError> {
        match self {
            Self::None => Ok("none".to_owned()),
            Self::DirectNumber(value) => Ok(format!("{}px", serialize_number(*value)?)),
            Self::DirectLength(value) => serialize_direct_length(value),
            Self::NumberCalculation(value) => {
                let value = serialize_typed(value)?;
                if value.parse::<CSSNumber>().is_ok() {
                    return Ok(format!("{value}px"));
                }
                Ok(format!("calc(1px * ({value}))"))
            }
            Self::LengthCalculation(value) => serialize_typed(value),
        }
    }

    pub(crate) fn observable_value(&self) -> Result<String, EngineError> {
        let canonical = self.canonical_value()?;
        if matches!(self, Self::LengthCalculation(_)) && leading_math_function(&canonical).is_none()
        {
            return Ok(format!("calc({canonical})"));
        }
        Ok(canonical)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum CrossDimensionCalculationValue {
    DirectNumber { value: CSSNumber, zero_as_px: bool },
    DirectPercentage { value: Percentage, as_number: bool },
    DirectLength(Length),
    DirectLengthPercentage(LengthPercentage),
    Auto,
    CommaList(Vec<CrossDimensionCalculationValue>),
    SpaceList(Vec<CrossDimensionCalculationValue>),
    LengthNumber(Calc<Length>),
    LengthOrNumber(Calc<Length>),
    PercentageCalculation(Calc<Percentage>),
    LengthPercentageNumber(Calc<PreservedLengthPercentage>),
    LengthPercentageOrNumber(Calc<PreservedLengthPercentage>),
}

impl CrossDimensionCalculationValue {
    fn canonical_value(&self) -> Result<String, EngineError> {
        match self {
            CrossDimensionCalculationValue::DirectNumber { value, zero_as_px } => {
                if *zero_as_px && *value == 0.0 {
                    return Ok("0px".to_owned());
                }
                serialize_number(*value)
            }
            CrossDimensionCalculationValue::DirectPercentage { value, as_number } => {
                if *as_number {
                    return serialize_number(value.0);
                }
                serialize_typed(value)
            }
            CrossDimensionCalculationValue::DirectLength(value) => serialize_direct_length(value),
            CrossDimensionCalculationValue::DirectLengthPercentage(value) => {
                serialize_direct_length_percentage(value)
            }
            CrossDimensionCalculationValue::Auto => Ok("auto".to_owned()),
            CrossDimensionCalculationValue::CommaList(values) => {
                let mut serialized = Vec::with_capacity(values.len());
                for value in values {
                    serialized.push(value.canonical_value()?);
                }
                Ok(serialized.join(", "))
            }
            CrossDimensionCalculationValue::SpaceList(values) => {
                let mut serialized = Vec::with_capacity(values.len());
                for value in values {
                    serialized.push(value.canonical_value()?);
                }
                Ok(serialized.join(" "))
            }
            CrossDimensionCalculationValue::LengthNumber(value) => serialize_calculation(value),
            CrossDimensionCalculationValue::LengthOrNumber(value) => serialize_calculation(value),
            CrossDimensionCalculationValue::PercentageCalculation(value) => {
                serialize_calculation(value)
            }
            CrossDimensionCalculationValue::LengthPercentageNumber(value)
            | CrossDimensionCalculationValue::LengthPercentageOrNumber(value) => {
                serialize_calculation(value)
            }
        }
    }

    fn retains_context_dependent_math(&self) -> bool {
        match self {
            CrossDimensionCalculationValue::DirectNumber { .. }
            | CrossDimensionCalculationValue::DirectPercentage { .. }
            | CrossDimensionCalculationValue::DirectLength(_)
            | CrossDimensionCalculationValue::DirectLengthPercentage(_)
            | CrossDimensionCalculationValue::Auto => false,
            CrossDimensionCalculationValue::CommaList(values)
            | CrossDimensionCalculationValue::SpaceList(values) => values
                .iter()
                .any(CrossDimensionCalculationValue::retains_context_dependent_math),
            CrossDimensionCalculationValue::LengthNumber(value) => value.contains_unresolved_sign(),
            CrossDimensionCalculationValue::LengthOrNumber(value) => {
                value.contains_unresolved_sign()
            }
            CrossDimensionCalculationValue::PercentageCalculation(value) => {
                value.contains_unresolved_sign()
            }
            CrossDimensionCalculationValue::LengthPercentageNumber(value)
            | CrossDimensionCalculationValue::LengthPercentageOrNumber(value) => {
                value.contains_unresolved_sign()
            }
        }
    }

    pub(crate) fn list_observable_value(&self, source: &str) -> Option<String> {
        let CrossDimensionCalculationValue::CommaList(values) = self else {
            return None;
        };
        let groups = split_top_level_delimiter(source, b',')?;
        let mut authored_components = Vec::with_capacity(values.len());
        for group in groups {
            authored_components.extend(split_top_level_whitespace(group)?);
        }
        if authored_components.len() != values.len() {
            return None;
        }

        let mut observable = Vec::with_capacity(values.len());
        for (value, authored) in values.iter().zip(authored_components) {
            let canonical = value.canonical_value().ok()?;
            if leading_math_function(authored).is_none() {
                observable.push(canonical);
                continue;
            }
            if leading_math_function(&canonical).is_some() {
                observable.push(canonical);
            } else {
                observable.push(format!("calc({canonical})"));
            }
        }
        Some(observable.join(", "))
    }
}

fn leading_math_function(source: &str) -> Option<String> {
    let mut input = ParserInput::new(source);
    let mut parser = Parser::new(&mut input);
    let function = match parser.next().ok()? {
        Token::Function(function) => function,
        _ => return None,
    };
    [
        "calc", "min", "max", "clamp", "round", "rem", "mod", "abs", "sign", "hypot", "sin", "cos",
        "tan", "asin", "acos", "atan", "atan2", "pow", "sqrt", "log", "exp",
    ]
    .iter()
    .find(|candidate| function.eq_ignore_ascii_case(candidate))
    .map(|candidate| (*candidate).to_owned())
}

pub(crate) fn is_numeric_extension_candidate(source: &str) -> bool {
    let mut input = ParserInput::new(source);
    let mut parser = Parser::new(&mut input);
    match parser.next().ok() {
        Some(Token::Number { .. } | Token::Percentage { .. } | Token::Dimension { .. }) => true,
        Some(Token::Function(_)) => leading_math_function(source).is_some(),
        _ => false,
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct IntegerCalculationValue {
    number: CSSNumber,
}

impl IntegerCalculationValue {
    pub fn number(&self) -> CSSNumber {
        self.number
    }

    fn canonical_value(&self) -> Result<String, EngineError> {
        Ok(format!("calc({})", serialize_number(self.number)?))
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum OffsetPositionValue {
    Auto,
    Normal,
    Position(Position),
}

impl OffsetPositionValue {
    fn canonical_value(&self) -> Result<String, EngineError> {
        match self {
            OffsetPositionValue::Auto => Ok("auto".to_owned()),
            OffsetPositionValue::Normal => Ok("normal".to_owned()),
            OffsetPositionValue::Position(position) => Ok(format!(
                "{} {}",
                serialize_horizontal_position(&position.x)?,
                serialize_vertical_position(&position.y)?
            )),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OffsetRotateDirection {
    Auto,
    Reverse,
}

#[derive(Clone, Debug, PartialEq)]
pub struct OffsetRotateValue {
    direction: Option<OffsetRotateDirection>,
    angle: Option<Angle>,
}

impl OffsetRotateValue {
    pub fn direction(&self) -> Option<OffsetRotateDirection> {
        self.direction
    }

    pub fn angle(&self) -> Option<&Angle> {
        self.angle.as_ref()
    }

    fn canonical_value(&self) -> Result<String, EngineError> {
        let mut components = Vec::with_capacity(2);
        if let Some(direction) = self.direction {
            components.push(match direction {
                OffsetRotateDirection::Auto => "auto".to_owned(),
                OffsetRotateDirection::Reverse => "reverse".to_owned(),
            });
        }
        if let Some(angle) = &self.angle {
            components.push(serialize_typed(angle)?);
        }
        Ok(components.join(" "))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PageOrientation {
    Portrait,
    Landscape,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NamedPageSize {
    A5,
    A4,
    A3,
    B5,
    B4,
    JisB5,
    JisB4,
    Ledger,
    Legal,
    Letter,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PageLength {
    value: Length,
    simplified_math: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub enum PageSizeValue {
    Auto,
    Orientation(PageOrientation),
    Named {
        size: NamedPageSize,
        orientation: Option<PageOrientation>,
    },
    Dimensions {
        width: PageLength,
        height: Option<PageLength>,
    },
}

impl PageSizeValue {
    fn canonical_value(&self) -> Result<String, EngineError> {
        match self {
            PageSizeValue::Auto => Ok("auto".to_owned()),
            PageSizeValue::Orientation(orientation) => Ok(orientation.as_str().to_owned()),
            PageSizeValue::Named { size, orientation } => {
                let mut output = size.as_str().to_owned();
                if let Some(orientation) = orientation {
                    output.push(' ');
                    output.push_str(orientation.as_str());
                }
                Ok(output)
            }
            PageSizeValue::Dimensions { width, height } => {
                let mut output = width.canonical_value()?;
                if let Some(height) = height {
                    output.push(' ');
                    output.push_str(&height.canonical_value()?);
                }
                Ok(output)
            }
        }
    }
}

impl PageLength {
    pub fn value(&self) -> &Length {
        &self.value
    }

    pub fn was_simplified_math(&self) -> bool {
        self.simplified_math
    }

    fn canonical_value(&self) -> Result<String, EngineError> {
        if matches!(&self.value, Length::Value(LengthValue::Px(value)) if *value == 0.0) {
            return Ok("0px".to_owned());
        }
        let serialized = serialize_typed(&self.value)?;
        if self.simplified_math {
            return Ok(format!("calc({serialized})"));
        }
        Ok(serialized)
    }
}

impl PageOrientation {
    fn parse(value: &str) -> Option<Self> {
        if value.eq_ignore_ascii_case("portrait") {
            return Some(PageOrientation::Portrait);
        }
        if value.eq_ignore_ascii_case("landscape") {
            return Some(PageOrientation::Landscape);
        }
        None
    }

    fn as_str(self) -> &'static str {
        match self {
            PageOrientation::Portrait => "portrait",
            PageOrientation::Landscape => "landscape",
        }
    }
}

impl NamedPageSize {
    fn parse(value: &str) -> Option<Self> {
        if value.eq_ignore_ascii_case("a5") {
            return Some(NamedPageSize::A5);
        }
        if value.eq_ignore_ascii_case("a4") {
            return Some(NamedPageSize::A4);
        }
        if value.eq_ignore_ascii_case("a3") {
            return Some(NamedPageSize::A3);
        }
        if value.eq_ignore_ascii_case("b5") {
            return Some(NamedPageSize::B5);
        }
        if value.eq_ignore_ascii_case("b4") {
            return Some(NamedPageSize::B4);
        }
        if value.eq_ignore_ascii_case("jis-b5") {
            return Some(NamedPageSize::JisB5);
        }
        if value.eq_ignore_ascii_case("jis-b4") {
            return Some(NamedPageSize::JisB4);
        }
        if value.eq_ignore_ascii_case("ledger") {
            return Some(NamedPageSize::Ledger);
        }
        if value.eq_ignore_ascii_case("legal") {
            return Some(NamedPageSize::Legal);
        }
        if value.eq_ignore_ascii_case("letter") {
            return Some(NamedPageSize::Letter);
        }
        None
    }

    fn as_str(self) -> &'static str {
        match self {
            NamedPageSize::A5 => "a5",
            NamedPageSize::A4 => "a4",
            NamedPageSize::A3 => "a3",
            NamedPageSize::B5 => "b5",
            NamedPageSize::B4 => "b4",
            NamedPageSize::JisB5 => "jis-b5",
            NamedPageSize::JisB4 => "jis-b4",
            NamedPageSize::Ledger => "ledger",
            NamedPageSize::Legal => "legal",
            NamedPageSize::Letter => "letter",
        }
    }
}

pub(crate) fn parse_extension_value(
    extensions: &[PropertyGrammarExtension],
    property_name: &str,
    source: &str,
) -> Result<Option<SemanticExtensionValue>, EngineError> {
    let mut last_error = None;
    for extension in extensions {
        let value = match extension {
            PropertyGrammarExtension::AspectRatio => Some(parse_aspect_ratio(source)),
            PropertyGrammarExtension::BrowserLonghand => {
                match parse_browser_longhand(property_name, source) {
                    Ok(Some(value)) => Some(Ok(SemanticExtensionValue::BrowserLonghand(value))),
                    Ok(None) => None,
                    Err(error) => Some(Err(error)),
                }
            }
            PropertyGrammarExtension::Content => Some(parse_content(source)),
            PropertyGrammarExtension::Geometric => {
                match parse_geometric_property(property_name, source) {
                    Ok(Some(value)) => Some(Ok(SemanticExtensionValue::Geometric(Box::new(value)))),
                    Ok(None) => None,
                    Err(error) => Some(Err(error)),
                }
            }
            PropertyGrammarExtension::IntegerCalculation => Some(parse_integer_calculation(source)),
            PropertyGrammarExtension::LengthNumberCalculation => {
                Some(parse_length_number_calculation(source))
            }
            PropertyGrammarExtension::LengthPercentageNumberCalculation => Some(
                parse_length_percentage_number_calculation(property_name, source),
            ),
            PropertyGrammarExtension::LengthPercentageOrNumberCalculation => Some(
                parse_length_percentage_or_number_calculation(property_name, source),
            ),
            PropertyGrammarExtension::OffsetPosition => {
                Some(parse_offset_position(property_name, source))
            }
            PropertyGrammarExtension::OffsetRotate => Some(parse_offset_rotate(source)),
            PropertyGrammarExtension::PageSize => Some(parse_page_size(source)),
            PropertyGrammarExtension::WebkitBorderImage => Some(parse_webkit_border_image(source)),
            PropertyGrammarExtension::WebkitBoxReflect => Some(parse_webkit_box_reflect(source)),
            PropertyGrammarExtension::WebkitMaskBoxImageSlice => {
                Some(parse_webkit_mask_box_image_slice(source))
            }
            PropertyGrammarExtension::WebkitPerspective => Some(parse_webkit_perspective(source)),
        };
        match value {
            Some(Ok(value)) => return Ok(Some(value)),
            Some(Err(error)) => last_error = Some(error),
            None => {}
        }
    }
    match last_error {
        Some(error) => Err(error),
        None => Ok(None),
    }
}

fn parse_content(source: &str) -> Result<SemanticExtensionValue, EngineError> {
    let value = parse_entire(source, |input| {
        if input
            .try_parse(|input| input.expect_ident_matching("normal"))
            .is_ok()
        {
            return Ok(ContentValue::Normal);
        }
        if input
            .try_parse(|input| input.expect_ident_matching("none"))
            .is_ok()
        {
            return Ok(ContentValue::None);
        }

        let mut items = Vec::new();
        let mut has_alternative = false;
        while !input.is_exhausted() {
            if input.try_parse(|input| input.expect_delim('/')).is_ok() {
                has_alternative = true;
                break;
            }
            items.push(parse_content_item(input)?);
        }
        if items.is_empty() {
            return Err(input.new_custom_error(lightningcss::error::ParserError::InvalidValue));
        }

        let mut alternative = Vec::new();
        while !input.is_exhausted() {
            alternative.push(parse_content_alternative(input)?);
        }
        if has_alternative && alternative.is_empty() {
            return Err(input.new_custom_error(lightningcss::error::ParserError::InvalidValue));
        }
        Ok(ContentValue::List { items, alternative })
    })?;
    Ok(SemanticExtensionValue::Content(value))
}

fn parse_content_item<'i, 't>(
    input: &mut Parser<'i, 't>,
) -> Result<ContentItem, cssparser::ParseError<'i, lightningcss::error::ParserError<'i>>> {
    if let Ok(image) = input.try_parse(Image::parse) {
        if !matches!(image, Image::None) {
            return Ok(ContentItem::Image(image.into_owned()));
        }
    }
    if let Ok(value) = input.try_parse(CSSString::parse) {
        return Ok(ContentItem::String(value.into_owned()));
    }
    if let Ok(value) = input.try_parse(parse_content_quote) {
        return Ok(ContentItem::Quote(value));
    }
    parse_content_counter(input).map(ContentItem::Counter)
}

fn parse_content_alternative<'i, 't>(
    input: &mut Parser<'i, 't>,
) -> Result<ContentAlternative, cssparser::ParseError<'i, lightningcss::error::ParserError<'i>>> {
    if let Ok(value) = input.try_parse(CSSString::parse) {
        return Ok(ContentAlternative::String(value.into_owned()));
    }
    parse_content_counter(input).map(ContentAlternative::Counter)
}

fn parse_content_quote<'i, 't>(
    input: &mut Parser<'i, 't>,
) -> Result<ContentQuote, cssparser::ParseError<'i, lightningcss::error::ParserError<'i>>> {
    let location = input.current_source_location();
    let identifier = input.expect_ident_cloned()?;
    if identifier.eq_ignore_ascii_case("open-quote") {
        return Ok(ContentQuote::Open);
    }
    if identifier.eq_ignore_ascii_case("close-quote") {
        return Ok(ContentQuote::Close);
    }
    if identifier.eq_ignore_ascii_case("no-open-quote") {
        return Ok(ContentQuote::NoOpen);
    }
    if identifier.eq_ignore_ascii_case("no-close-quote") {
        return Ok(ContentQuote::NoClose);
    }
    Err(location.new_custom_error(lightningcss::error::ParserError::InvalidValue))
}

fn parse_content_counter<'i, 't>(
    input: &mut Parser<'i, 't>,
) -> Result<ContentCounter, cssparser::ParseError<'i, lightningcss::error::ParserError<'i>>> {
    let location = input.current_source_location();
    let function = input.expect_function()?.clone();
    let counters = if function.eq_ignore_ascii_case("counter") {
        false
    } else if function.eq_ignore_ascii_case("counters") {
        true
    } else {
        return Err(location.new_custom_error(lightningcss::error::ParserError::InvalidValue));
    };
    input.parse_nested_block(|input| {
        let name = CustomIdent::parse(input)?.into_owned();
        let separator = if counters {
            input.expect_comma()?;
            Some(CSSString::parse(input)?.into_owned())
        } else {
            None
        };
        let style = if input.try_parse(|input| input.expect_comma()).is_ok() {
            let style = CounterStyle::parse(input)?;
            if matches!(style, CounterStyle::Symbols { .. }) {
                return Err(input.new_custom_error(lightningcss::error::ParserError::InvalidValue));
            }
            style.into_owned()
        } else {
            CounterStyle::Predefined(PredefinedCounterStyle::Decimal)
        };
        Ok(ContentCounter {
            name,
            separator,
            style,
        })
    })
}

fn parse_webkit_box_reflect(source: &str) -> Result<SemanticExtensionValue, EngineError> {
    let value = parse_entire(source, |input| {
        let direction = parse_webkit_box_reflect_direction(input)?;
        if input.is_exhausted() {
            return Ok(WebkitBoxReflectValue {
                direction,
                offset: LengthPercentage::px(0.0),
                mask: None,
            });
        }
        let offset = LengthPercentage::parse(input)?;
        let mask = if input.is_exhausted() {
            None
        } else {
            Some(parse_webkit_reflect_mask(input)?)
        };
        Ok(WebkitBoxReflectValue {
            direction,
            offset,
            mask,
        })
    })?;
    Ok(SemanticExtensionValue::WebkitBoxReflect(value))
}

fn parse_webkit_reflect_mask<'i, 't>(
    input: &mut Parser<'i, 't>,
) -> Result<WebkitReflectMaskValue, cssparser::ParseError<'i, lightningcss::error::ParserError<'i>>>
{
    let mut source = None;
    let mut slice = None;
    let mut width = None;
    let mut outset = None;
    let mut repeat = None;

    loop {
        if slice.is_none() {
            if let Ok(value) = input.try_parse(BorderImageSlice::parse) {
                slice = Some(value);
                let width_outset = input.try_parse(parse_webkit_reflect_width_outset);
                if let Ok((parsed_width, parsed_outset)) = width_outset {
                    width = parsed_width;
                    outset = parsed_outset;
                }
                continue;
            }
        }
        if source.is_none() {
            if let Ok(value) = input.try_parse(Image::parse) {
                if !matches!(value, Image::None) {
                    source = Some(value.into_owned());
                    continue;
                }
            }
        }
        if repeat.is_none() {
            if let Ok(value) = input.try_parse(BorderImageRepeat::parse) {
                repeat = Some(value);
                continue;
            }
        }
        break;
    }

    if source.is_none() && slice.is_none() && repeat.is_none() {
        return Err(input.new_custom_error(lightningcss::error::ParserError::InvalidValue));
    }
    Ok(WebkitReflectMaskValue {
        source,
        slice,
        width,
        outset,
        repeat,
    })
}

fn parse_webkit_reflect_width_outset<'i, 't>(
    input: &mut Parser<'i, 't>,
) -> Result<WebkitReflectWidthOutset, PropertyParseError<'i>> {
    input.expect_delim('/')?;
    let width = input.try_parse(Rect::parse).ok();
    let outset = input
        .try_parse(|input| {
            input.expect_delim('/')?;
            Rect::parse(input)
        })
        .ok();
    if width.is_none() && outset.is_none() {
        return Err(input.new_custom_error(lightningcss::error::ParserError::InvalidValue));
    }
    Ok((width, outset))
}

fn parse_webkit_box_reflect_direction<'i, 't>(
    input: &mut Parser<'i, 't>,
) -> Result<
    WebkitBoxReflectDirection,
    cssparser::ParseError<'i, lightningcss::error::ParserError<'i>>,
> {
    let location = input.current_source_location();
    let identifier = input.expect_ident_cloned()?;
    if identifier.eq_ignore_ascii_case("above") {
        return Ok(WebkitBoxReflectDirection::Above);
    }
    if identifier.eq_ignore_ascii_case("below") {
        return Ok(WebkitBoxReflectDirection::Below);
    }
    if identifier.eq_ignore_ascii_case("left") {
        return Ok(WebkitBoxReflectDirection::Left);
    }
    if identifier.eq_ignore_ascii_case("right") {
        return Ok(WebkitBoxReflectDirection::Right);
    }
    Err(location.new_custom_error(lightningcss::error::ParserError::InvalidValue))
}

fn serialize_space_separated<T: ToCss>(values: &[T]) -> Result<String, EngineError> {
    let mut serialized = Vec::with_capacity(values.len());
    for value in values {
        serialized.push(serialize_typed(value)?);
    }
    Ok(serialized.join(" "))
}

pub(crate) fn parse_preferred_extension_value(
    extensions: &[PropertyGrammarExtension],
    property_name: &str,
    source: &str,
) -> Option<SemanticExtensionValue> {
    let preferred = extensions.iter().copied().filter(|extension| {
        matches!(
            extension,
            PropertyGrammarExtension::AspectRatio
                | PropertyGrammarExtension::LengthNumberCalculation
                | PropertyGrammarExtension::LengthPercentageNumberCalculation
                | PropertyGrammarExtension::LengthPercentageOrNumberCalculation
                | PropertyGrammarExtension::WebkitBorderImage
                | PropertyGrammarExtension::WebkitMaskBoxImageSlice
        )
    });
    for extension in preferred {
        let Ok(Some(value)) = parse_extension_value(&[extension], property_name, source) else {
            continue;
        };
        if matches!(
            extension,
            PropertyGrammarExtension::LengthPercentageNumberCalculation
                | PropertyGrammarExtension::LengthPercentageOrNumberCalculation
        ) || value.retains_context_dependent_math()
        {
            return Some(value);
        }
    }
    None
}

fn parse_contextual_number(source: &str) -> Result<Calc<PreservedLengthPercentage>, EngineError> {
    let value = match parse_entire(source, Calc::<PreservedLengthPercentage>::parse) {
        Ok(value) => value,
        Err(_) => Calc::Number(parse_entire(source, CSSNumber::parse)?),
    };
    if !value.resolves_to_number() {
        return Err(EngineError::Parse(
            "calculation does not resolve to a number".to_owned(),
        ));
    }
    Ok(value)
}

fn parse_aspect_ratio(source: &str) -> Result<SemanticExtensionValue, EngineError> {
    let sections = split_top_level_delimiter(source, b'/')
        .filter(|sections| !sections.is_empty() && sections.len() <= 2)
        .ok_or_else(|| EngineError::Parse("invalid aspect-ratio structure".to_owned()))?;
    let numerator = parse_contextual_number(sections[0].trim())?;
    let denominator = match sections.get(1) {
        Some(value) => parse_contextual_number(value.trim())?,
        None => Calc::Number(1.0),
    };
    Ok(SemanticExtensionValue::AspectRatio(AspectRatioValue {
        numerator,
        denominator,
    }))
}

fn parse_webkit_border_image(source: &str) -> Result<SemanticExtensionValue, EngineError> {
    let canonical = (|| {
        let components = split_top_level_whitespace(source)?;
        if components.len() > 2 || components.get(1).is_some_and(|value| *value != "fill") {
            return None;
        }
        let slice = parse_contextual_number(components.first().copied()?).ok()?;
        Some(format!("{} fill", serialize_typed(&slice).ok()?))
    })()
    .or_else(|| canonicalize_webkit_border_image(source))
    .ok_or_else(|| EngineError::Parse("invalid -webkit-border-image structure".to_owned()))?;
    Ok(SemanticExtensionValue::WebkitBorderImage(
        WebkitBorderImageValue::Compound(canonical),
    ))
}

fn parse_webkit_mask_box_image_slice(source: &str) -> Result<SemanticExtensionValue, EngineError> {
    let components = split_top_level_whitespace(source)
        .filter(|components| components.len() == 2 && components[1] == "fill")
        .ok_or_else(|| {
            EngineError::Parse("invalid -webkit-mask-box-image-slice structure".to_owned())
        })?;
    let slice = parse_contextual_number(components[0])?;
    Ok(SemanticExtensionValue::WebkitMaskBoxImageSlice(
        WebkitBorderImageValue::Slice(slice),
    ))
}

fn parse_webkit_perspective(source: &str) -> Result<SemanticExtensionValue, EngineError> {
    let value = if source.eq_ignore_ascii_case("none") {
        WebkitPerspectiveValue::None
    } else if leading_math_function(source).is_none() {
        if let Ok(value) = parse_entire(source, CSSNumber::parse) {
            if value < 0.0 {
                return Err(invalid_numeric_value());
            }
            WebkitPerspectiveValue::DirectNumber(value)
        } else {
            let value = parse_entire(source, Length::parse)?;
            if direct_length_is_negative(&value) {
                return Err(invalid_numeric_value());
            }
            WebkitPerspectiveValue::DirectLength(value)
        }
    } else {
        let number = parse_entire(source, Calc::<PreservedLengthPercentage>::parse);
        if let Ok(value) = number {
            if value.resolves_to_number() {
                return Ok(SemanticExtensionValue::WebkitPerspective(
                    WebkitPerspectiveValue::NumberCalculation(value),
                ));
            }
        }
        WebkitPerspectiveValue::LengthCalculation(parse_entire(source, Calc::<Length>::parse)?)
    };
    Ok(SemanticExtensionValue::WebkitPerspective(value))
}

pub(crate) fn parse_contextual_dimension_calculation(source: &str) -> Option<String> {
    let value = parse_entire(source, Calc::<PreservedLengthPercentage>::parse).ok()?;
    if value.resolves_to_number() || !value.contains_unresolved_sign() {
        return None;
    }
    serialize_typed(&value).ok()
}

fn parse_length_number_calculation(source: &str) -> Result<SemanticExtensionValue, EngineError> {
    let value = parse_entire(source, Calc::<Length>::parse)?;
    if !value.resolves_to_number() {
        return Err(EngineError::Parse(
            "calculation does not resolve to a number".to_owned(),
        ));
    }
    Ok(SemanticExtensionValue::CrossDimensionCalculation(
        CrossDimensionCalculationValue::LengthNumber(value),
    ))
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum NumericRange {
    Any,
    NonNegative,
    Positive,
    Integer,
    NonZeroInteger,
    PositiveInteger,
}

impl NumericRange {
    fn accepts(self, value: CSSNumber) -> bool {
        match self {
            NumericRange::Any => true,
            NumericRange::NonNegative => value >= 0.0,
            NumericRange::Positive => value > 0.0,
            NumericRange::Integer => value.is_finite() && value.fract() == 0.0,
            NumericRange::NonZeroInteger => {
                value.is_finite() && value != 0.0 && value.fract() == 0.0
            }
            NumericRange::PositiveInteger => {
                value.is_finite() && value > 0.0 && value.fract() == 0.0
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct NumberPercentageProfile {
    allow_percentage: bool,
    percentage_as_number: bool,
    range: NumericRange,
}

fn number_percentage_profile(property_name: &str) -> NumberPercentageProfile {
    let allow_percentage = matches!(
        property_name,
        "-webkit-mask-box-image-slice"
            | "-webkit-opacity"
            | "-webkit-shape-image-threshold"
            | "border-image-slice"
            | "fill-opacity"
            | "flood-opacity"
            | "opacity"
            | "scale"
            | "shape-image-threshold"
            | "stop-opacity"
            | "stroke-opacity"
            | "zoom"
    );
    let range = match property_name {
        "-webkit-box-ordinal-group"
        | "-webkit-column-count"
        | "-webkit-line-clamp"
        | "column-count"
        | "flex-line-count"
        | "hyphenate-limit-chars"
        | "orphans"
        | "widows" => NumericRange::PositiveInteger,
        "grid-column-end" | "grid-column-start" | "grid-row-end" | "grid-row-start" => {
            NumericRange::NonZeroInteger
        }
        "-webkit-order" | "math-depth" | "order" | "reading-order" => NumericRange::Integer,
        "font-weight" | "initial-letter" => NumericRange::Positive,
        "-webkit-animation-iteration-count"
        | "-webkit-flex-grow"
        | "-webkit-flex-shrink"
        | "-webkit-mask-box-image-slice"
        | "animation-iteration-count"
        | "border-image-slice"
        | "flex-grow"
        | "flex-shrink"
        | "font-size-adjust"
        | "stroke-miterlimit"
        | "zoom" => NumericRange::NonNegative,
        _ => NumericRange::Any,
    };
    NumberPercentageProfile {
        allow_percentage,
        percentage_as_number: allow_percentage
            && !matches!(
                property_name,
                "-webkit-mask-box-image-slice" | "border-image-slice" | "zoom"
            ),
        range,
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct DimensionNumberProfile {
    allow_percentage: bool,
    non_negative: bool,
    zero_as_px: bool,
}

fn dimension_number_profile(property_name: &str) -> DimensionNumberProfile {
    DimensionNumberProfile {
        allow_percentage: !matches!(
            property_name,
            "-webkit-mask-box-image-outset" | "border-image-outset" | "tab-size"
        ),
        non_negative: !matches!(
            property_name,
            "baseline-shift" | "cx" | "cy" | "stroke-dashoffset" | "x" | "y"
        ),
        zero_as_px: matches!(property_name, "cx" | "cy" | "r" | "rx" | "ry" | "x" | "y"),
    }
}

fn invalid_numeric_value() -> EngineError {
    EngineError::Parse("invalid numeric extension value".to_owned())
}

fn validate_direct_number(range: NumericRange, value: CSSNumber) -> Result<(), EngineError> {
    range
        .accepts(value)
        .then_some(())
        .ok_or_else(invalid_numeric_value)
}

fn parse_length_percentage_number_calculation(
    property_name: &str,
    source: &str,
) -> Result<SemanticExtensionValue, EngineError> {
    let profile = number_percentage_profile(property_name);
    if leading_math_function(source).is_none() {
        if let Ok(value) = parse_entire(source, CSSNumber::parse) {
            validate_direct_number(profile.range, value)?;
            return Ok(SemanticExtensionValue::CrossDimensionCalculation(
                CrossDimensionCalculationValue::DirectNumber {
                    value,
                    zero_as_px: false,
                },
            ));
        }
        if profile.allow_percentage {
            let value = parse_entire(source, Percentage::parse)?;
            validate_direct_number(profile.range, value.0)?;
            return Ok(SemanticExtensionValue::CrossDimensionCalculation(
                CrossDimensionCalculationValue::DirectPercentage {
                    value,
                    as_number: profile.percentage_as_number,
                },
            ));
        }
        return Err(invalid_numeric_value());
    }

    if profile.allow_percentage {
        if let Ok(value) = parse_entire(source, Calc::<Percentage>::parse) {
            return Ok(SemanticExtensionValue::CrossDimensionCalculation(
                CrossDimensionCalculationValue::PercentageCalculation(value),
            ));
        }
    }
    let value = parse_entire(source, Calc::<PreservedLengthPercentage>::parse)?;
    if value.resolves_to_number() {
        return Ok(SemanticExtensionValue::CrossDimensionCalculation(
            CrossDimensionCalculationValue::LengthPercentageNumber(value),
        ));
    }
    Err(invalid_numeric_value())
}

fn parse_dimension_number_scalar(
    property_name: &str,
    source: &str,
) -> Result<CrossDimensionCalculationValue, EngineError> {
    let profile = dimension_number_profile(property_name);
    if leading_math_function(source).is_none() {
        if let Ok(value) = parse_entire(source, CSSNumber::parse) {
            if profile.non_negative && value < 0.0 {
                return Err(invalid_numeric_value());
            }
            return Ok(CrossDimensionCalculationValue::DirectNumber {
                value,
                zero_as_px: profile.zero_as_px,
            });
        }
        if profile.allow_percentage {
            let value = parse_entire(source, LengthPercentage::parse)?;
            if profile.non_negative && direct_length_percentage_is_negative(&value) {
                return Err(invalid_numeric_value());
            }
            return Ok(CrossDimensionCalculationValue::DirectLengthPercentage(
                value,
            ));
        }
        let value = parse_entire(source, Length::parse)?;
        if profile.non_negative && direct_length_is_negative(&value) {
            return Err(invalid_numeric_value());
        }
        return Ok(CrossDimensionCalculationValue::DirectLength(value));
    }

    if let Ok(value) = parse_entire(source, Calc::<PreservedLengthPercentage>::parse) {
        if profile.allow_percentage || value.resolves_to_number() {
            return Ok(CrossDimensionCalculationValue::LengthPercentageOrNumber(
                value,
            ));
        }
    }
    if let Ok(value) = parse_entire(source, Calc::<Length>::parse) {
        return Ok(CrossDimensionCalculationValue::LengthOrNumber(value));
    }
    Err(invalid_numeric_value())
}

fn direct_length_is_negative(value: &Length) -> bool {
    matches!(value, Length::Value(value) if value.to_unit_value().0 < 0.0)
}

fn direct_length_percentage_is_negative(value: &LengthPercentage) -> bool {
    match value {
        DimensionPercentage::Dimension(value) => value.to_unit_value().0 < 0.0,
        DimensionPercentage::Percentage(value) => value.0 < 0.0,
        DimensionPercentage::Calc(_) => false,
    }
}

fn parse_stroke_dasharray(source: &str) -> Result<CrossDimensionCalculationValue, EngineError> {
    let comma_groups = split_top_level_delimiter(source, b',').ok_or_else(invalid_numeric_value)?;
    let mut values = Vec::new();
    for group in comma_groups {
        let components = split_top_level_whitespace(group).ok_or_else(invalid_numeric_value)?;
        if components.is_empty() {
            return Err(invalid_numeric_value());
        }
        for component in components {
            values.push(parse_dimension_number_scalar(
                "stroke-dasharray",
                component,
            )?);
        }
    }
    if values.is_empty() {
        return Err(invalid_numeric_value());
    }
    Ok(CrossDimensionCalculationValue::CommaList(values))
}

fn parse_border_image_dimension_list(
    property_name: &str,
    source: &str,
) -> Result<CrossDimensionCalculationValue, EngineError> {
    let components = split_top_level_whitespace(source).ok_or_else(invalid_numeric_value)?;
    if !(1..=4).contains(&components.len()) {
        return Err(invalid_numeric_value());
    }
    let allows_auto = property_name.ends_with("image-width");
    let mut values = Vec::with_capacity(components.len());
    for component in components {
        if allows_auto && component.eq_ignore_ascii_case("auto") {
            values.push(CrossDimensionCalculationValue::Auto);
            continue;
        }
        values.push(parse_dimension_number_scalar(property_name, component)?);
    }
    Ok(CrossDimensionCalculationValue::SpaceList(values))
}

fn parse_length_percentage_or_number_calculation(
    property_name: &str,
    source: &str,
) -> Result<SemanticExtensionValue, EngineError> {
    let value = match property_name {
        "stroke-dasharray" => parse_stroke_dasharray(source)?,
        "-webkit-mask-box-image-outset"
        | "-webkit-mask-box-image-width"
        | "border-image-outset"
        | "border-image-width" => parse_border_image_dimension_list(property_name, source)?,
        _ => parse_dimension_number_scalar(property_name, source)?,
    };
    Ok(SemanticExtensionValue::CrossDimensionCalculation(value))
}

fn parse_integer_calculation(source: &str) -> Result<SemanticExtensionValue, EngineError> {
    let number = parse_entire(source, CSSNumber::parse)?;
    Ok(SemanticExtensionValue::IntegerCalculation(
        IntegerCalculationValue { number },
    ))
}

fn parse_offset_position(
    property_name: &str,
    source: &str,
) -> Result<SemanticExtensionValue, EngineError> {
    let value = parse_entire(source, |input| {
        if input
            .try_parse(|input| input.expect_ident_matching("auto"))
            .is_ok()
        {
            return Ok(OffsetPositionValue::Auto);
        }
        if property_name == "offset-position"
            && input
                .try_parse(|input| input.expect_ident_matching("normal"))
                .is_ok()
        {
            return Ok(OffsetPositionValue::Normal);
        }
        Position::parse(input).map(OffsetPositionValue::Position)
    })?;
    Ok(SemanticExtensionValue::OffsetPosition(value))
}

fn parse_offset_rotate(source: &str) -> Result<SemanticExtensionValue, EngineError> {
    let value = parse_entire(source, |input| {
        let mut direction = None;
        let mut angle = None;
        while !input.is_exhausted() {
            if direction.is_none() {
                if input
                    .try_parse(|input| input.expect_ident_matching("auto"))
                    .is_ok()
                {
                    direction = Some(OffsetRotateDirection::Auto);
                    continue;
                }
                if input
                    .try_parse(|input| input.expect_ident_matching("reverse"))
                    .is_ok()
                {
                    direction = Some(OffsetRotateDirection::Reverse);
                    continue;
                }
            }
            if angle.is_none() {
                if let Ok(parsed) = input.try_parse(Angle::parse) {
                    angle = Some(parsed);
                    continue;
                }
            }
            return Err(input.new_custom_error(lightningcss::error::ParserError::InvalidValue));
        }
        if direction.is_none() && angle.is_none() {
            return Err(input.new_custom_error(lightningcss::error::ParserError::InvalidValue));
        }
        Ok(OffsetRotateValue { direction, angle })
    })?;
    Ok(SemanticExtensionValue::OffsetRotate(value))
}

fn parse_page_size(source: &str) -> Result<SemanticExtensionValue, EngineError> {
    let value = parse_entire(source, |input| {
        if let Ok(first) = input.try_parse(|input| input.expect_ident_cloned()) {
            if first.eq_ignore_ascii_case("auto") && input.is_exhausted() {
                return Ok(PageSizeValue::Auto);
            }
            let first_size = NamedPageSize::parse(&first);
            let first_orientation = PageOrientation::parse(&first);
            if input.is_exhausted() {
                if let Some(size) = first_size {
                    return Ok(PageSizeValue::Named {
                        size,
                        orientation: None,
                    });
                }
                if let Some(orientation) = first_orientation {
                    return Ok(PageSizeValue::Orientation(orientation));
                }
            }

            let second = input.expect_ident_cloned()?;
            let second_size = NamedPageSize::parse(&second);
            let second_orientation = PageOrientation::parse(&second);
            let (Some(size), Some(orientation)) = (match (
                first_size,
                first_orientation,
                second_size,
                second_orientation,
            ) {
                (Some(size), None, None, Some(orientation))
                | (None, Some(orientation), Some(size), None) => (Some(size), Some(orientation)),
                _ => (None, None),
            }) else {
                return Err(input.new_custom_error(lightningcss::error::ParserError::InvalidValue));
            };
            return Ok(PageSizeValue::Named {
                size,
                orientation: Some(orientation),
            });
        }

        let width = parse_page_length(input)?;
        let height = input.try_parse(parse_page_length).ok();
        Ok(PageSizeValue::Dimensions { width, height })
    })?;
    Ok(SemanticExtensionValue::PageSize(value))
}

fn parse_page_length<'i, 't>(
    input: &mut Parser<'i, 't>,
) -> Result<PageLength, cssparser::ParseError<'i, lightningcss::error::ParserError<'i>>> {
    if let Ok(number) = input.try_parse(|input| input.expect_number()) {
        if number == 0.0 {
            return Ok(PageLength {
                value: Length::Value(LengthValue::Px(0.0)),
                simplified_math: false,
            });
        }
        return Err(input.new_custom_error(lightningcss::error::ParserError::InvalidValue));
    }

    let state = input.state();
    let math_function = input
        .expect_function()
        .is_ok_and(|function| is_math_function(function));
    input.reset(&state);
    let value = Length::parse(input)?;
    if !math_function && matches!(&value, Length::Value(length) if length.sign() < 0.0) {
        return Err(input.new_custom_error(lightningcss::error::ParserError::InvalidValue));
    }
    let simplified_math = math_function && matches!(value, Length::Value(_));
    Ok(PageLength {
        value,
        simplified_math,
    })
}

fn is_math_function(name: &str) -> bool {
    [
        "calc", "min", "max", "clamp", "round", "mod", "rem", "hypot", "abs", "sign",
    ]
    .iter()
    .any(|candidate| name.eq_ignore_ascii_case(candidate))
}

fn serialize_horizontal_position(value: &HorizontalPosition) -> Result<String, EngineError> {
    serialize_position_component(value)
}

fn serialize_vertical_position(value: &VerticalPosition) -> Result<String, EngineError> {
    serialize_position_component(value)
}

fn serialize_position_component<S: ToCss>(
    value: &PositionComponent<S>,
) -> Result<String, EngineError> {
    serialize_typed(value)
}

fn serialize_direct_length(value: &Length) -> Result<String, EngineError> {
    let Length::Value(value) = value else {
        return serialize_typed(value);
    };
    let (number, unit) = value.to_unit_value();
    if number == 0.0 {
        return Ok(format!("0{unit}"));
    }
    serialize_typed(value)
}

fn serialize_direct_length_percentage(value: &LengthPercentage) -> Result<String, EngineError> {
    if let DimensionPercentage::Dimension(value) = value {
        let (number, unit) = value.to_unit_value();
        if number == 0.0 {
            return Ok(format!("0{unit}"));
        }
    }
    serialize_typed(value)
}

fn serialize_number(number: CSSNumber) -> Result<String, EngineError> {
    if number.is_nan() {
        return Ok("NaN".to_owned());
    }
    if number == CSSNumber::INFINITY {
        return Ok("infinity".to_owned());
    }
    if number == CSSNumber::NEG_INFINITY {
        return Ok("-infinity".to_owned());
    }
    let serialized = serialize_typed(&number)?;
    if let Some(fraction) = serialized.strip_prefix("-.") {
        return Ok(format!("-0.{fraction}"));
    }
    if let Some(fraction) = serialized.strip_prefix('.') {
        return Ok(format!("0.{fraction}"));
    }
    Ok(serialized)
}

fn serialize_calculation<T>(value: &Calc<T>) -> Result<String, EngineError>
where
    Calc<T>: ToCss,
{
    let serialized = serialize_typed(value)?;
    if leading_math_function(&serialized).is_some() {
        return Ok(serialized);
    }
    Ok(format!("calc({serialized})"))
}

fn serialize_typed<T: ToCss>(value: &T) -> Result<String, EngineError> {
    value
        .to_css_string(PrinterOptions::default())
        .map_err(|error| EngineError::Serialize(error.to_string()))
}

fn parse_entire<'i, T, F>(source: &'i str, parser: F) -> Result<T, EngineError>
where
    F: for<'t> FnOnce(
        &mut Parser<'i, 't>,
    )
        -> Result<T, cssparser::ParseError<'i, lightningcss::error::ParserError<'i>>>,
{
    let mut input = ParserInput::new(source);
    let mut css = Parser::new(&mut input);
    css.parse_entirely(parser)
        .map_err(|_| EngineError::Parse("invalid SheetOM extension value".to_owned()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(
        extension: PropertyGrammarExtension,
        property: &str,
        source: &str,
    ) -> Result<SemanticExtensionValue, EngineError> {
        parse_extension_value(&[extension], property, source)?.ok_or_else(|| {
            EngineError::Parse("extension did not return a semantic value".to_owned())
        })
    }

    #[test]
    fn parses_integer_calculations() {
        for (source, expected) in [
            ("calc(1 + 1)", "calc(2)"),
            ("min(1, 2)", "calc(1)"),
            ("calc(1.5)", "calc(1.5)"),
            ("calc(-1.5)", "calc(-1.5)"),
            ("calc(infinity)", "calc(infinity)"),
            ("calc(NaN)", "calc(NaN)"),
        ] {
            let value = parse(
                PropertyGrammarExtension::IntegerCalculation,
                "z-index",
                source,
            )
            .unwrap();
            assert_eq!(value.canonical_value().unwrap(), expected, "{source}");
        }
        assert!(parse(
            PropertyGrammarExtension::IntegerCalculation,
            "z-index",
            "calc(1px)"
        )
        .is_err());
    }

    #[test]
    fn parses_context_dependent_compound_values() {
        let ratio = parse(
            PropertyGrammarExtension::AspectRatio,
            "aspect-ratio",
            "sign(1em) / 1",
        )
        .unwrap();
        assert_eq!(ratio.canonical_value().unwrap(), "sign(1em) / 1");

        let image = parse(
            PropertyGrammarExtension::WebkitBorderImage,
            "-webkit-border-image",
            "sign(1em) fill",
        )
        .unwrap();
        assert_eq!(image.canonical_value().unwrap(), "sign(1em) fill");
    }

    #[test]
    fn parses_static_content_from_typed_component_values() {
        for (source, expected) in [
            ("normal", "normal"),
            ("none", "none"),
            ("open-quote", "open-quote"),
            ("\"a\" \"b\"", "\"a\" \"b\""),
            ("url(a.png)", "url(\"a.png\")"),
            ("linear-gradient(red,blue)", "linear-gradient(red, #00f)"),
            ("counter(chapter, decimal)", "counter(chapter)"),
            (
                "counters(chapter, \".\", upper-roman)",
                "counters(chapter, \".\", upper-roman)",
            ),
            (
                "url(a.png) / counter(label)",
                "url(\"a.png\") / counter(label)",
            ),
        ] {
            let value = parse(PropertyGrammarExtension::Content, "content", source).unwrap();
            assert_eq!(value.canonical_value().unwrap(), expected, "{source}");
        }
        for source in [
            "contents",
            "normal \"text\"",
            "leader(\".\")",
            "target-text(url(#target))",
            "counter()",
            "counters(name)",
            "\"text\" /",
            "url(a.png) / url(alt.png)",
        ] {
            assert!(
                parse(PropertyGrammarExtension::Content, "content", source).is_err(),
                "{source}"
            );
        }
    }

    #[test]
    fn parses_webkit_box_reflect_without_erasing_mask_presence() {
        for (source, expected) in [
            ("below", "below 0px"),
            ("above 10px", "above 10px"),
            ("left 5%", "left 5%"),
            ("right calc(1px + 2%)", "right calc(2% + 1px)"),
            ("below 0 url(mask.png)", "below 0px url(\"mask.png\")"),
            (
                "below 0 url(mask.png) 30 / 10 / 0 stretch",
                "below 0px url(\"mask.png\") 30 fill / 10 / 0 stretch",
            ),
            ("below 0 30", "below 0px 30 fill"),
            ("below 0 stretch", "below 0px stretch"),
            ("below 0 30 / / 2", "below 0px 30 fill / 2"),
        ] {
            let value = parse(
                PropertyGrammarExtension::WebkitBoxReflect,
                "-webkit-box-reflect",
                source,
            )
            .unwrap();
            assert_eq!(value.canonical_value().unwrap(), expected, "{source}");
        }
        for source in [
            "none",
            "below url(mask.png)",
            "below 0 fill",
            "below 0px junk",
        ] {
            assert!(
                parse(
                    PropertyGrammarExtension::WebkitBoxReflect,
                    "-webkit-box-reflect",
                    source,
                )
                .is_err(),
                "{source}"
            );
        }
    }

    #[test]
    fn owns_number_results_with_relative_dimension_arguments() {
        for (extension, source, expected) in [
            (
                PropertyGrammarExtension::LengthNumberCalculation,
                "sign(1em)",
                "sign(1em)",
            ),
            (
                PropertyGrammarExtension::LengthNumberCalculation,
                "sign(calc(1px - 2em))",
                "sign(-2em + 1px)",
            ),
            (
                PropertyGrammarExtension::LengthNumberCalculation,
                "calc(sign(1em) * 2)",
                "calc(2 * sign(1em))",
            ),
            (
                PropertyGrammarExtension::LengthPercentageNumberCalculation,
                "sign(1%)",
                "sign(1%)",
            ),
            (
                PropertyGrammarExtension::LengthPercentageNumberCalculation,
                "sign(calc(1px + 2%))",
                "sign(2% + 1px)",
            ),
            (
                PropertyGrammarExtension::LengthPercentageNumberCalculation,
                "calc(sign(1em) / sign(1rem))",
                "calc(sign(1em) * (1 / sign(1rem)))",
            ),
        ] {
            let value = parse(extension, "opacity", source).unwrap();
            assert_eq!(value.canonical_value().unwrap(), expected, "{source}");
        }
    }

    #[test]
    fn owns_number_or_dimension_results_without_erasing_the_result_type() {
        for (source, expected) in [
            ("sign(1em)", "sign(1em)"),
            ("calc(sign(1em) * 1px)", "calc(1px * sign(1em))"),
            ("calc(1px / sign(1em))", "calc(1px * (1 / sign(1em)))"),
        ] {
            let value = parse(
                PropertyGrammarExtension::LengthPercentageOrNumberCalculation,
                "line-height",
                source,
            )
            .unwrap();
            assert_eq!(value.canonical_value().unwrap(), expected, "{source}");
        }
    }

    #[test]
    fn rejects_dimension_results_and_incompatible_math() {
        for (extension, source) in [
            (
                PropertyGrammarExtension::LengthNumberCalculation,
                "calc(sign(1em) * 1px)",
            ),
            (
                PropertyGrammarExtension::LengthPercentageNumberCalculation,
                "calc(sign(1em) * 1px)",
            ),
            (
                PropertyGrammarExtension::LengthNumberCalculation,
                "sin(1em)",
            ),
            (
                PropertyGrammarExtension::LengthNumberCalculation,
                "sign(1%)",
            ),
        ] {
            assert!(parse(extension, "opacity", source).is_err(), "{source}");
        }
    }

    #[test]
    fn validates_direct_number_percentage_profiles() {
        let extension = PropertyGrammarExtension::LengthPercentageNumberCalculation;
        for (property, source, expected) in [
            ("flood-opacity", "-1", "-1"),
            ("flood-opacity", "10%", "0.1"),
            ("zoom", "10%", "10%"),
            ("font-weight", "1.5", "1.5"),
            ("grid-column-start", "-1", "-1"),
            ("math-depth", "0", "0"),
            ("-webkit-box-ordinal-group", "1", "1"),
        ] {
            let value = parse(extension, property, source).unwrap();
            assert_eq!(
                value.canonical_value().unwrap(),
                expected,
                "{property}: {source}"
            );
        }
        for (property, source) in [
            ("zoom", "-1"),
            ("font-weight", "0"),
            ("grid-column-start", "0"),
            ("math-depth", "1.5"),
            ("-webkit-box-ordinal-group", "1.5"),
            ("flood-opacity", "10px"),
        ] {
            assert!(
                parse(extension, property, source).is_err(),
                "{property}: {source}"
            );
        }
    }

    #[test]
    fn validates_direct_dimension_number_profiles_and_lists() {
        let extension = PropertyGrammarExtension::LengthPercentageOrNumberCalculation;
        for (property, source, expected) in [
            ("x", "0", "0px"),
            ("x", "-1", "-1"),
            ("x", "-10px", "-10px"),
            ("r", "0", "0px"),
            ("r", "10%", "10%"),
            ("stroke-width", "0px", "0px"),
            ("stroke-dashoffset", "-1", "-1"),
            ("tab-size", "10px", "10px"),
            ("stroke-dasharray", "1 2px, 3%", "1, 2px, 3%"),
            ("stroke-dasharray", "calc(1 + 1) 2", "calc(2), 2"),
            ("border-image-outset", "1px 2 3px 4", "1px 2 3px 4"),
            ("border-image-width", "1px auto 2 3px", "1px auto 2 3px"),
        ] {
            let value = parse(extension, property, source).unwrap();
            assert_eq!(
                value.canonical_value().unwrap(),
                expected,
                "{property}: {source}"
            );
        }
        for (property, source) in [
            ("r", "-1"),
            ("stroke-width", "-1px"),
            ("stroke-dasharray", "1 -2"),
            ("stroke-dasharray", "1,,2"),
            ("tab-size", "10%"),
            ("border-image-outset", "1px 2px 3px 4px 5px"),
            ("border-image-outset", "1px auto"),
            ("border-image-width", "1px -2px"),
        ] {
            assert!(
                parse(extension, property, source).is_err(),
                "{property}: {source}"
            );
        }
    }

    #[test]
    fn owns_legacy_webkit_perspective_unitless_lengths() {
        let extension = PropertyGrammarExtension::WebkitPerspective;
        for (source, expected) in [
            ("none", "none"),
            ("0", "0px"),
            ("1.5", "1.5px"),
            ("10px", "10px"),
            ("calc(1 + 1)", "2px"),
            ("min(1px, 2px)", "1px"),
        ] {
            let value = parse(extension, "-webkit-perspective", source).unwrap();
            assert_eq!(value.canonical_value().unwrap(), expected, "{source}");
        }
        for source in ["-1", "-10px", "10%", "calc(10%)"] {
            assert!(
                parse(extension, "-webkit-perspective", source).is_err(),
                "{source}"
            );
        }
    }

    #[test]
    fn parses_and_orders_offset_positions() {
        for (property, source, expected) in [
            ("offset-position", "normal", "normal"),
            ("offset-position", "auto", "auto"),
            ("offset-anchor", "auto", "auto"),
            ("offset-position", "center", "center center"),
            ("offset-position", "left", "left center"),
            ("offset-position", "top", "center top"),
            (
                "offset-position",
                "top 20px left 10px",
                "left 10px top 20px",
            ),
        ] {
            let value = parse(PropertyGrammarExtension::OffsetPosition, property, source).unwrap();
            assert_eq!(value.canonical_value().unwrap(), expected, "{source}");
        }
        assert!(parse(
            PropertyGrammarExtension::OffsetPosition,
            "offset-anchor",
            "normal"
        )
        .is_err());
    }

    #[test]
    fn parses_unordered_offset_rotation_components() {
        for (source, expected) in [
            ("auto", "auto"),
            ("reverse", "reverse"),
            ("0deg", "0deg"),
            ("10deg auto", "auto 10deg"),
            ("10deg reverse", "reverse 10deg"),
        ] {
            let value = parse(
                PropertyGrammarExtension::OffsetRotate,
                "offset-rotate",
                source,
            )
            .unwrap();
            assert_eq!(value.canonical_value().unwrap(), expected, "{source}");
        }
        for source in ["0", "auto reverse", "10deg 20deg"] {
            assert!(
                parse(
                    PropertyGrammarExtension::OffsetRotate,
                    "offset-rotate",
                    source
                )
                .is_err(),
                "{source}"
            );
        }
    }

    #[test]
    fn parses_page_size_without_width_property_proxying() {
        for (source, expected) in [
            ("auto", "auto"),
            ("A4", "a4"),
            ("landscape", "landscape"),
            ("landscape A4", "a4 landscape"),
            ("0", "0px"),
            ("-0", "0px"),
            ("10cm 20cm", "10cm 20cm"),
            ("calc(1cm + 2mm)", "calc(45.3543px)"),
            ("calc(-1px)", "calc(-1px)"),
        ] {
            let value = parse(PropertyGrammarExtension::PageSize, "size", source);
            assert!(value.is_ok(), "{source}: {value:?}");
            let value = value.unwrap();
            assert_eq!(value.canonical_value().unwrap(), expected, "{source}");
        }
        for source in ["1", "-1px", "50%", "auto landscape"] {
            assert!(
                parse(PropertyGrammarExtension::PageSize, "size", source).is_err(),
                "{source}"
            );
        }
    }
}
