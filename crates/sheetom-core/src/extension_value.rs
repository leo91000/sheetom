use cssparser::{Parser, ParserInput};
use lightningcss::{
    stylesheet::PrinterOptions,
    traits::{Parse, Sign, ToCss},
    values::{
        angle::Angle,
        calc::Calc,
        length::{Length, LengthValue, PreservedLengthPercentage},
        number::CSSNumber,
        position::{HorizontalPosition, Position, PositionComponent, VerticalPosition},
    },
};

use crate::{catalog::PropertyGrammarExtension, EngineError};

#[derive(Clone, Debug, PartialEq)]
pub enum SemanticExtensionValue {
    IntegerCalculation(IntegerCalculationValue),
    CrossDimensionCalculation(CrossDimensionCalculationValue),
    OffsetPosition(OffsetPositionValue),
    OffsetRotate(OffsetRotateValue),
    PageSize(PageSizeValue),
}

impl SemanticExtensionValue {
    pub fn canonical_value(&self) -> Result<String, EngineError> {
        match self {
            SemanticExtensionValue::IntegerCalculation(value) => value.canonical_value(),
            SemanticExtensionValue::CrossDimensionCalculation(value) => value.canonical_value(),
            SemanticExtensionValue::OffsetPosition(value) => value.canonical_value(),
            SemanticExtensionValue::OffsetRotate(value) => value.canonical_value(),
            SemanticExtensionValue::PageSize(value) => value.canonical_value(),
        }
    }

    pub(crate) fn retains_context_dependent_math(&self) -> bool {
        match self {
            SemanticExtensionValue::CrossDimensionCalculation(value) => {
                value.retains_context_dependent_math()
            }
            _ => false,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum CrossDimensionCalculationValue {
    LengthNumber(Calc<Length>),
    LengthPercentageNumber(Calc<PreservedLengthPercentage>),
    LengthPercentageOrNumber(Calc<PreservedLengthPercentage>),
}

impl CrossDimensionCalculationValue {
    fn canonical_value(&self) -> Result<String, EngineError> {
        match self {
            CrossDimensionCalculationValue::LengthNumber(value) => serialize_typed(value),
            CrossDimensionCalculationValue::LengthPercentageNumber(value)
            | CrossDimensionCalculationValue::LengthPercentageOrNumber(value) => {
                serialize_typed(value)
            }
        }
    }

    fn retains_context_dependent_math(&self) -> bool {
        match self {
            CrossDimensionCalculationValue::LengthNumber(value) => value.contains_unresolved_sign(),
            CrossDimensionCalculationValue::LengthPercentageNumber(value)
            | CrossDimensionCalculationValue::LengthPercentageOrNumber(value) => {
                value.contains_unresolved_sign()
            }
        }
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
            PropertyGrammarExtension::IntegerCalculation => Some(parse_integer_calculation(source)),
            PropertyGrammarExtension::LengthNumberCalculation => {
                Some(parse_length_number_calculation(source))
            }
            PropertyGrammarExtension::LengthPercentageNumberCalculation => {
                Some(parse_length_percentage_number_calculation(source))
            }
            PropertyGrammarExtension::LengthPercentageOrNumberCalculation => {
                Some(parse_length_percentage_or_number_calculation(source))
            }
            PropertyGrammarExtension::OffsetPosition => {
                Some(parse_offset_position(property_name, source))
            }
            PropertyGrammarExtension::OffsetRotate => Some(parse_offset_rotate(source)),
            PropertyGrammarExtension::PageSize => Some(parse_page_size(source)),
            _ => None,
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

pub(crate) fn parse_preferred_extension_value(
    extensions: &[PropertyGrammarExtension],
    property_name: &str,
    source: &str,
) -> Option<SemanticExtensionValue> {
    let preferred = extensions.iter().copied().filter(|extension| {
        matches!(
            extension,
            PropertyGrammarExtension::LengthNumberCalculation
                | PropertyGrammarExtension::LengthPercentageNumberCalculation
                | PropertyGrammarExtension::LengthPercentageOrNumberCalculation
        )
    });
    for extension in preferred {
        let Ok(Some(value)) = parse_extension_value(&[extension], property_name, source) else {
            continue;
        };
        if value.retains_context_dependent_math() {
            return Some(value);
        }
    }
    None
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

fn parse_length_percentage_number_calculation(
    source: &str,
) -> Result<SemanticExtensionValue, EngineError> {
    let value = parse_entire(source, Calc::<PreservedLengthPercentage>::parse)?;
    if !value.resolves_to_number() {
        return Err(EngineError::Parse(
            "calculation does not resolve to a number".to_owned(),
        ));
    }
    Ok(SemanticExtensionValue::CrossDimensionCalculation(
        CrossDimensionCalculationValue::LengthPercentageNumber(value),
    ))
}

fn parse_length_percentage_or_number_calculation(
    source: &str,
) -> Result<SemanticExtensionValue, EngineError> {
    let value = parse_entire(source, Calc::<PreservedLengthPercentage>::parse)?;
    Ok(SemanticExtensionValue::CrossDimensionCalculation(
        CrossDimensionCalculationValue::LengthPercentageOrNumber(value),
    ))
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
    serialize_typed(&number)
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
