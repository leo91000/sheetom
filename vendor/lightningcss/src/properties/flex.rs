//! CSS properties related to flexbox layout.

use super::align::{
  AlignContent, AlignItems, AlignSelf, ContentDistribution, ContentPosition, JustifyContent, SelfPosition,
};
use super::{Property, PropertyId};
use crate::context::PropertyHandlerContext;
use crate::declaration::{DeclarationBlock, DeclarationList};
use crate::error::{ParserError, PrinterError};
use crate::macros::*;
use crate::prefixes::{is_flex_2009, Feature};
use crate::printer::Printer;
use crate::targets::Browsers;
use crate::traits::{
  private::TryAdd, FromStandard, IsCompatible, Parse, PropertyHandler, Shorthand, ToCss, TryMap, TryOp,
  TrySign, Zero,
};
use crate::values::angle::Angle;
use crate::values::calc::{Calc, MathFunction};
use crate::values::number::{CSSInteger, CSSNumber};
use crate::values::{
  length::{LengthPercentage, LengthPercentageOrAuto, LengthValue},
  percentage::{DimensionPercentage, Percentage},
};
use crate::vendor_prefix::VendorPrefix;
#[cfg(feature = "visitor")]
use crate::visitor::Visit;
use cssparser::*;

enum_property! {
  /// A value for the [flex-direction](https://www.w3.org/TR/2018/CR-css-flexbox-1-20181119/#propdef-flex-direction) property.
  pub enum FlexDirection {
    /// Flex items are laid out in a row.
    Row,
    /// Flex items are laid out in a row, and reversed.
    RowReverse,
    /// Flex items are laid out in a column.
    Column,
    /// Flex items are laid out in a column, and reversed.
    ColumnReverse,
  }
}

impl Default for FlexDirection {
  fn default() -> FlexDirection {
    FlexDirection::Row
  }
}

/// A value for the [flex-wrap](https://www.w3.org/TR/css-flexbox-2/#flex-wrap-property) property.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "visitor", derive(Visit))]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize), serde(rename_all = "kebab-case"))]
#[cfg_attr(feature = "jsonschema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "into_owned", derive(static_self::IntoOwned))]
pub enum FlexWrap {
  /// The flex items do not wrap.
  NoWrap,
  /// The flex items wrap.
  Wrap,
  /// The flex items wrap, in reverse.
  WrapReverse,
  /// Wrapped lines are balanced. An authored `wrap` is omitted when serialized.
  Balance,
  /// Reverse wrapped lines are balanced.
  WrapReverseBalance,
}

impl<'i> Parse<'i> for FlexWrap {
  fn parse<'t>(input: &mut Parser<'i, 't>) -> Result<Self, ParseError<'i, ParserError<'i>>> {
    let location = input.current_source_location();
    let ident = input.expect_ident()?;
    let mut value = cssparser::match_ignore_ascii_case! { &*ident,
      "nowrap" => Self::NoWrap,
      "wrap" => Self::Wrap,
      "wrap-reverse" => Self::WrapReverse,
      "balance" => Self::Balance,
      _ => return Err(location.new_unexpected_token_error(Token::Ident(ident.clone()))),
    };
    if matches!(value, Self::Wrap | Self::WrapReverse)
      && input.try_parse(|input| input.expect_ident_matching("balance")).is_ok()
    {
      value = match value {
        Self::Wrap => Self::Balance,
        Self::WrapReverse => Self::WrapReverseBalance,
        _ => unreachable!(),
      };
    } else if value == Self::Balance {
      if input.try_parse(|input| input.expect_ident_matching("wrap")).is_ok() {
        value = Self::Balance;
      } else if input
        .try_parse(|input| input.expect_ident_matching("wrap-reverse"))
        .is_ok()
      {
        value = Self::WrapReverseBalance;
      }
    }
    Ok(value)
  }
}

impl FlexWrap {
  /// Returns the canonical serialized value.
  pub fn as_str(&self) -> &str {
    match self {
      Self::NoWrap => "nowrap",
      Self::Wrap => "wrap",
      Self::WrapReverse => "wrap-reverse",
      Self::Balance => "balance",
      Self::WrapReverseBalance => "wrap-reverse balance",
    }
  }
}

impl ToCss for FlexWrap {
  fn to_css<W>(&self, dest: &mut Printer<W>) -> Result<(), PrinterError>
  where
    W: std::fmt::Write,
  {
    dest.write_str(self.as_str())
  }
}

impl Default for FlexWrap {
  fn default() -> FlexWrap {
    FlexWrap::NoWrap
  }
}

impl FromStandard<FlexWrap> for FlexWrap {
  fn from_standard(wrap: &FlexWrap) -> Option<FlexWrap> {
    Some(wrap.clone())
  }
}

define_shorthand! {
  /// A value for the [flex-flow](https://www.w3.org/TR/2018/CR-css-flexbox-1-20181119/#flex-flow-property) shorthand property.
  pub struct FlexFlow(VendorPrefix) {
    /// The direction that flex items flow.
    direction: FlexDirection(FlexDirection, VendorPrefix),
    /// How the flex items wrap.
    wrap: FlexWrap(FlexWrap, VendorPrefix),
  }
}

impl<'i> Parse<'i> for FlexFlow {
  fn parse<'t>(input: &mut Parser<'i, 't>) -> Result<Self, ParseError<'i, ParserError<'i>>> {
    let mut direction = None;
    let mut wrap = None;
    loop {
      if direction.is_none() {
        if let Ok(value) = input.try_parse(FlexDirection::parse) {
          direction = Some(value);
          continue;
        }
      }
      if wrap.is_none() {
        if let Ok(value) = input.try_parse(FlexWrap::parse) {
          wrap = Some(value);
          continue;
        }
      }
      break;
    }

    Ok(FlexFlow {
      direction: direction.unwrap_or_default(),
      wrap: wrap.unwrap_or_default(),
    })
  }
}

impl ToCss for FlexFlow {
  fn to_css<W>(&self, dest: &mut Printer<W>) -> Result<(), PrinterError>
  where
    W: std::fmt::Write,
  {
    let mut needs_space = false;
    if self.direction != FlexDirection::default() || self.wrap == FlexWrap::default() {
      self.direction.to_css(dest)?;
      needs_space = true;
    }

    if self.wrap != FlexWrap::default() {
      if needs_space {
        dest.write_str(" ")?;
      }
      self.wrap.to_css(dest)?;
    }

    Ok(())
  }
}

/// A value for the `flex-basis` property.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "visitor", derive(Visit))]
#[cfg_attr(
  feature = "serde",
  derive(serde::Serialize, serde::Deserialize),
  serde(tag = "type", content = "value", rename_all = "kebab-case")
)]
#[cfg_attr(feature = "jsonschema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "into_owned", derive(static_self::IntoOwned))]
pub enum FlexBasis {
  /// The `auto` keyword.
  Auto,
  /// The `content` keyword.
  Content,
  /// The `min-content` keyword.
  MinContent(VendorPrefix),
  /// The `max-content` keyword.
  MaxContent(VendorPrefix),
  /// The `fit-content` keyword.
  FitContent(VendorPrefix),
  /// The `stretch` keyword or a legacy equivalent.
  Stretch(VendorPrefix),
  /// An explicit length or percentage.
  LengthPercentage(LengthPercentage),
  /// A `calc-size()` function.
  CalcSize(Box<FlexCalcSize>),
}

/// A typed `calc-size()` function in the `flex-basis` calculation context.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "visitor", derive(Visit))]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "jsonschema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "into_owned", derive(static_self::IntoOwned))]
pub struct FlexCalcSize {
  /// The intrinsic or numeric basis.
  pub basis: FlexCalcSizeBasis,
  /// The calculation applied to the basis.
  pub calculation: Calc<FlexCalcSizeLengthPercentage>,
}

/// A basis accepted by `calc-size()` in `flex-basis`.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "visitor", derive(Visit))]
#[cfg_attr(
  feature = "serde",
  derive(serde::Serialize, serde::Deserialize),
  serde(tag = "type", content = "value", rename_all = "kebab-case")
)]
#[cfg_attr(feature = "jsonschema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "into_owned", derive(static_self::IntoOwned))]
pub enum FlexCalcSizeBasis {
  /// A basis that can interpolate with every size.
  Any,
  /// A complete `flex-basis` value, including a nested `calc-size()`.
  Value(Box<FlexBasis>),
  /// A bare `<calc-sum>` basis.
  Calculation(Calc<LengthPercentage>),
}

/// A dimension in the calculation argument of `calc-size()`.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "visitor", derive(Visit))]
#[cfg_attr(
  feature = "serde",
  derive(serde::Serialize, serde::Deserialize),
  serde(tag = "type", content = "value", rename_all = "kebab-case")
)]
#[cfg_attr(feature = "jsonschema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "into_owned", derive(static_self::IntoOwned))]
pub enum FlexCalcSizeLength {
  /// An ordinary length.
  Length(LengthValue),
  /// The `size` placeholder, including an arithmetic multiplier.
  Size(CSSNumber),
}

/// A length-percentage calculation that can refer to the `size` placeholder.
pub type FlexCalcSizeLengthPercentage = DimensionPercentage<FlexCalcSizeLength>;

impl Default for FlexBasis {
  fn default() -> Self {
    Self::Auto
  }
}

impl<'i> Parse<'i> for FlexBasis {
  fn parse<'t>(input: &mut Parser<'i, 't>) -> Result<Self, ParseError<'i, ParserError<'i>>> {
    if let Ok(value) = input.try_parse(FlexCalcSize::parse) {
      return Ok(Self::CalcSize(Box::new(value)));
    }

    if let Ok(value) = input.try_parse(parse_flex_basis_keyword) {
      return Ok(value);
    }

    LengthPercentage::parse(input).map(Self::LengthPercentage)
  }
}

impl FlexBasis {
  fn parse_shorthand<'i, 't>(
    input: &mut Parser<'i, 't>,
  ) -> Result<Self, ParseError<'i, ParserError<'i>>> {
    let location = input.current_source_location();
    let value = Self::parse(input)?;
    if matches!(value, Self::CalcSize(_)) {
      return Err(location.new_custom_error(ParserError::InvalidValue));
    }
    Ok(value)
  }
}

fn parse_flex_basis_keyword<'i, 't>(
  input: &mut Parser<'i, 't>,
) -> Result<FlexBasis, ParseError<'i, ParserError<'i>>> {
  let location = input.current_source_location();
  let ident = input.expect_ident_cloned()?;
  Ok(match_ignore_ascii_case! { &ident,
    "auto" => FlexBasis::Auto,
    "content" => FlexBasis::Content,
    "min-content" => FlexBasis::MinContent(VendorPrefix::None),
    "-webkit-min-content" => FlexBasis::MinContent(VendorPrefix::WebKit),
    "-moz-min-content" => FlexBasis::MinContent(VendorPrefix::Moz),
    "max-content" => FlexBasis::MaxContent(VendorPrefix::None),
    "-webkit-max-content" => FlexBasis::MaxContent(VendorPrefix::WebKit),
    "-moz-max-content" => FlexBasis::MaxContent(VendorPrefix::Moz),
    "fit-content" => FlexBasis::FitContent(VendorPrefix::None),
    "-webkit-fit-content" => FlexBasis::FitContent(VendorPrefix::WebKit),
    "-moz-fit-content" => FlexBasis::FitContent(VendorPrefix::Moz),
    "stretch" => FlexBasis::Stretch(VendorPrefix::None),
    "-webkit-fill-available" => FlexBasis::Stretch(VendorPrefix::WebKit),
    "-moz-available" => FlexBasis::Stretch(VendorPrefix::Moz),
    _ => return Err(location.new_unexpected_token_error(Token::Ident(ident))),
  })
}

impl ToCss for FlexBasis {
  fn to_css<W>(&self, dest: &mut Printer<W>) -> Result<(), PrinterError>
  where
    W: std::fmt::Write,
  {
    match self {
      Self::Auto => dest.write_str("auto"),
      Self::Content => dest.write_str("content"),
      Self::MinContent(prefix) => {
        prefix.to_css(dest)?;
        dest.write_str("min-content")
      }
      Self::MaxContent(prefix) => {
        prefix.to_css(dest)?;
        dest.write_str("max-content")
      }
      Self::FitContent(prefix) => {
        prefix.to_css(dest)?;
        dest.write_str("fit-content")
      }
      Self::Stretch(prefix) => match *prefix {
        VendorPrefix::None => dest.write_str("stretch"),
        VendorPrefix::WebKit => dest.write_str("-webkit-fill-available"),
        VendorPrefix::Moz => dest.write_str("-moz-available"),
        _ => unreachable!(),
      },
      Self::LengthPercentage(value) => value.to_css(dest),
      Self::CalcSize(value) => value.to_css(dest),
    }
  }
}

impl IsCompatible for FlexBasis {
  fn is_compatible(&self, browsers: Browsers) -> bool {
    match self {
      Self::Auto => true,
      Self::Content | Self::CalcSize(_) => false,
      Self::MinContent(_) => crate::compat::Feature::MinContentSize.is_compatible(browsers),
      Self::MaxContent(_) => crate::compat::Feature::MaxContentSize.is_compatible(browsers),
      Self::FitContent(_) => crate::compat::Feature::FitContentSize.is_compatible(browsers),
      Self::Stretch(prefix) => match *prefix {
        VendorPrefix::None => crate::compat::Feature::StretchSize,
        VendorPrefix::WebKit | VendorPrefix::Moz => crate::compat::Feature::WebkitFillAvailableSize,
        _ => return false,
      }
      .is_compatible(browsers),
      Self::LengthPercentage(value) => value.is_compatible(browsers),
    }
  }
}

impl FromStandard<FlexBasis> for LengthPercentageOrAuto {
  fn from_standard(value: &FlexBasis) -> Option<Self> {
    match value {
      FlexBasis::Auto => Some(Self::Auto),
      FlexBasis::LengthPercentage(value) => Some(Self::LengthPercentage(value.clone())),
      _ => None,
    }
  }
}

impl<'i> Parse<'i> for FlexCalcSize {
  fn parse<'t>(input: &mut Parser<'i, 't>) -> Result<Self, ParseError<'i, ParserError<'i>>> {
    input.expect_function_matching("calc-size")?;
    input.parse_nested_block(|input| {
      let basis = if input
        .try_parse(|input| {
          input.expect_ident_matching("any")?;
          input.expect_comma()
        })
        .is_ok()
      {
        FlexCalcSizeBasis::Any
      } else if let Ok(value) = input.try_parse(|input| {
        let value = FlexBasis::parse(input)?;
        input.expect_comma()?;
        Ok::<_, ParseError<'i, ParserError<'i>>>(value)
      }) {
        FlexCalcSizeBasis::Value(Box::new(value))
      } else {
        let calculation = Calc::<LengthPercentage>::parse_sum_with(input, |_| None)?;
        if !calculation.resolves_to_dimension() {
          return Err(input.new_custom_error(ParserError::InvalidValue));
        }
        input.expect_comma()?;
        FlexCalcSizeBasis::Calculation(calculation)
      };

      let calculation = Calc::<FlexCalcSizeLengthPercentage>::parse_sum_with(input, |_| None)?;
      if !calculation.resolves_to_dimension() {
        return Err(input.new_custom_error(ParserError::InvalidValue));
      }
      if matches!(basis, FlexCalcSizeBasis::Any) && calc_size_contains_size(&calculation) {
        return Err(input.new_custom_error(ParserError::InvalidValue));
      }
      Ok(Self { basis, calculation })
    })
  }
}

fn calc_size_contains_size(value: &Calc<FlexCalcSizeLengthPercentage>) -> bool {
  match value {
    Calc::Value(value) => match value.as_ref() {
      DimensionPercentage::Dimension(FlexCalcSizeLength::Size(_)) => true,
      DimensionPercentage::Dimension(FlexCalcSizeLength::Length(_))
      | DimensionPercentage::Percentage(_) => false,
      DimensionPercentage::Calc(value) => calc_size_contains_size(value),
    },
    Calc::Number(_) => false,
    Calc::Sum(left, right)
    | Calc::ProductExpression(left, right)
    | Calc::QuotientExpression(left, right) => {
      calc_size_contains_size(left) || calc_size_contains_size(right)
    }
    Calc::Product(_, value) => calc_size_contains_size(value),
    Calc::Function(function) => match function.as_ref() {
      MathFunction::Calc(value) | MathFunction::Abs(value) | MathFunction::Sign(value) => {
        calc_size_contains_size(value)
      }
      MathFunction::Min(values) | MathFunction::Max(values) | MathFunction::Hypot(values) => {
        values.iter().any(calc_size_contains_size)
      }
      MathFunction::Clamp(min, center, max) => {
        calc_size_contains_size(min)
          || calc_size_contains_size(center)
          || calc_size_contains_size(max)
      }
      MathFunction::Round(_, value, step)
      | MathFunction::Rem(value, step)
      | MathFunction::Mod(value, step) => {
        calc_size_contains_size(value) || calc_size_contains_size(step)
      }
    },
  }
}

impl ToCss for FlexCalcSize {
  fn to_css<W>(&self, dest: &mut Printer<W>) -> Result<(), PrinterError>
  where
    W: std::fmt::Write,
  {
    dest.write_str("calc-size(")?;
    self.basis.to_css(dest)?;
    dest.delim(',', false)?;
    serialize_flex_calc_size_calculation(&self.calculation, dest)?;
    dest.write_char(')')
  }
}

fn serialize_flex_calc_size_calculation<W>(
  value: &Calc<FlexCalcSizeLengthPercentage>,
  dest: &mut Printer<W>,
) -> Result<(), PrinterError>
where
  W: std::fmt::Write,
{
  let Calc::Sum(left, right) = value else {
    return value.to_css(dest);
  };

  serialize_flex_calc_size_term(left, dest)?;
  if flex_calc_size_term_is_negative(right) {
    dest.write_str(" - ")?;
    serialize_flex_calc_size_term(&(right.as_ref().clone() * -1.0), dest)
  } else {
    dest.write_str(" + ")?;
    serialize_flex_calc_size_term(right, dest)
  }
}

fn flex_calc_size_term_is_negative(value: &Calc<FlexCalcSizeLengthPercentage>) -> bool {
  match value {
    Calc::Value(value) => matches!(
      value.as_ref(),
      DimensionPercentage::Dimension(FlexCalcSizeLength::Size(multiplier))
        if multiplier.is_sign_negative()
    ),
    _ => value.is_sign_negative(),
  }
}

fn serialize_flex_calc_size_term<W>(
  value: &Calc<FlexCalcSizeLengthPercentage>,
  dest: &mut Printer<W>,
) -> Result<(), PrinterError>
where
  W: std::fmt::Write,
{
  if let Calc::Sum(_, _) = value {
    return serialize_flex_calc_size_calculation(value, dest);
  }

  let scaled_size = matches!(
    value,
    Calc::Value(value)
      if matches!(
        value.as_ref(),
        DimensionPercentage::Dimension(FlexCalcSizeLength::Size(multiplier))
          if *multiplier != 1.0
      )
  );
  if scaled_size {
    dest.write_char('(')?;
  }
  value.to_css(dest)?;
  if scaled_size {
    dest.write_char(')')?;
  }
  Ok(())
}

impl ToCss for FlexCalcSizeBasis {
  fn to_css<W>(&self, dest: &mut Printer<W>) -> Result<(), PrinterError>
  where
    W: std::fmt::Write,
  {
    match self {
      Self::Any => dest.write_str("any"),
      Self::Value(value) => value.to_css(dest),
      Self::Calculation(value) => value.to_css(dest),
    }
  }
}

impl<'i> Parse<'i> for FlexCalcSizeLength {
  fn parse<'t>(input: &mut Parser<'i, 't>) -> Result<Self, ParseError<'i, ParserError<'i>>> {
    if input
      .try_parse(|input| input.expect_ident_matching("size"))
      .is_ok()
    {
      return Ok(Self::Size(1.0));
    }
    LengthValue::parse(input).map(Self::Length)
  }
}

impl ToCss for FlexCalcSizeLength {
  fn to_css<W>(&self, dest: &mut Printer<W>) -> Result<(), PrinterError>
  where
    W: std::fmt::Write,
  {
    match self {
      Self::Length(value) => value.to_css(dest),
      Self::Size(multiplier) if *multiplier == 1.0 => dest.write_str("size"),
      Self::Size(multiplier) => {
        multiplier.to_css(dest)?;
        dest.write_str(" * size")
      }
    }
  }
}

impl std::ops::Mul<CSSNumber> for FlexCalcSizeLength {
  type Output = Self;

  fn mul(self, multiplier: CSSNumber) -> Self {
    match self {
      Self::Length(value) => Self::Length(value * multiplier),
      Self::Size(value) => Self::Size(value * multiplier),
    }
  }
}

impl TryAdd<FlexCalcSizeLength> for FlexCalcSizeLength {
  fn try_add(&self, other: &FlexCalcSizeLength) -> Option<FlexCalcSizeLength> {
    match (self, other) {
      (Self::Length(left), Self::Length(right)) => left.try_add(right).map(Self::Length),
      _ => None,
    }
  }

  fn canonical_order(&self, other: &FlexCalcSizeLength) -> Option<std::cmp::Ordering> {
    match (self, other) {
      (Self::Length(_), Self::Size(_)) => Some(std::cmp::Ordering::Less),
      (Self::Size(_), Self::Length(_)) => Some(std::cmp::Ordering::Greater),
      _ => None,
    }
  }
}

impl PartialOrd<FlexCalcSizeLength> for FlexCalcSizeLength {
  fn partial_cmp(&self, other: &FlexCalcSizeLength) -> Option<std::cmp::Ordering> {
    match (self, other) {
      (Self::Length(left), Self::Length(right)) => left.partial_cmp(right),
      _ => None,
    }
  }
}

impl TryOp for FlexCalcSizeLength {
  fn try_op<F: FnOnce(f32, f32) -> f32>(&self, rhs: &Self, op: F) -> Option<Self> {
    match (self, rhs) {
      (Self::Length(left), Self::Length(right)) => left.try_op(right, op).map(Self::Length),
      _ => None,
    }
  }

  fn try_op_to<T, F: FnOnce(f32, f32) -> T>(&self, rhs: &Self, op: F) -> Option<T> {
    match (self, rhs) {
      (Self::Length(left), Self::Length(right)) => left.try_op_to(right, op),
      _ => None,
    }
  }
}

impl TryMap for FlexCalcSizeLength {
  fn try_map<F: FnOnce(f32) -> f32>(&self, op: F) -> Option<Self> {
    match self {
      Self::Length(value) => Some(Self::Length(crate::traits::Map::map(value, op))),
      Self::Size(_) => None,
    }
  }
}

impl Zero for FlexCalcSizeLength {
  fn zero() -> Self {
    Self::Length(LengthValue::Px(0.0))
  }

  fn is_zero(&self) -> bool {
    matches!(self, Self::Length(value) if value.is_zero())
  }
}

impl TrySign for FlexCalcSizeLength {
  fn try_sign(&self) -> Option<f32> {
    match self {
      Self::Length(value) => Some(crate::traits::Sign::sign(value)),
      Self::Size(_) => None,
    }
  }
}

impl TryFrom<Angle> for FlexCalcSizeLength {
  type Error = ();

  fn try_from(value: Angle) -> Result<Self, Self::Error> {
    LengthValue::try_from(value).map(Self::Length)
  }
}

impl TryInto<Angle> for FlexCalcSizeLength {
  type Error = ();

  fn try_into(self) -> Result<Angle, Self::Error> {
    match self {
      Self::Length(value) => value.try_into(),
      Self::Size(_) => Err(()),
    }
  }
}

impl IsCompatible for FlexCalcSizeLength {
  fn is_compatible(&self, browsers: Browsers) -> bool {
    match self {
      Self::Length(value) => value.is_compatible(browsers),
      Self::Size(_) => false,
    }
  }
}

impl IsCompatible for DimensionPercentage<FlexCalcSizeLength> {
  fn is_compatible(&self, browsers: Browsers) -> bool {
    match self {
      DimensionPercentage::Dimension(value) => value.is_compatible(browsers),
      DimensionPercentage::Percentage(_) => true,
      DimensionPercentage::Calc(value) => value.is_compatible(browsers),
    }
  }
}

define_shorthand! {
/// A value for the [flex](https://www.w3.org/TR/2018/CR-css-flexbox-1-20181119/#flex-property) shorthand property.
  pub struct Flex(VendorPrefix) {
    /// The flex grow factor.
    grow: FlexGrow(CSSNumber, VendorPrefix),
    /// The flex shrink factor.
    shrink: FlexShrink(CSSNumber, VendorPrefix),
    /// The flex basis.
    basis: FlexBasis(FlexBasis, VendorPrefix),
  }
}

impl<'i> Parse<'i> for Flex {
  fn parse<'t>(input: &mut Parser<'i, 't>) -> Result<Self, ParseError<'i, ParserError<'i>>> {
    if input.try_parse(|input| input.expect_ident_matching("none")).is_ok() {
      return Ok(Flex {
        grow: 0.0,
        shrink: 0.0,
        basis: FlexBasis::Auto,
      });
    }

    let mut grow = None;
    let mut shrink = None;
    let mut basis = None;

    loop {
      if grow.is_none() {
        if let Ok(val) = input.try_parse(CSSNumber::parse) {
          grow = Some(val);
          shrink = input.try_parse(CSSNumber::parse).ok();
          continue;
        }
      }

      if basis.is_none() {
        if let Ok(val) = input.try_parse(FlexBasis::parse_shorthand) {
          basis = Some(val);
          continue;
        }
      }

      break;
    }

    Ok(Flex {
      grow: grow.unwrap_or(1.0),
      shrink: shrink.unwrap_or(1.0),
      basis: basis.unwrap_or(FlexBasis::LengthPercentage(LengthPercentage::Percentage(Percentage(
        0.0,
      )))),
    })
  }
}

impl ToCss for Flex {
  fn to_css<W>(&self, dest: &mut Printer<W>) -> Result<(), PrinterError>
  where
    W: std::fmt::Write,
  {
    if self.grow == 0.0 && self.shrink == 0.0 && self.basis == FlexBasis::Auto {
      dest.write_str("none")?;
      return Ok(());
    }

    #[derive(PartialEq)]
    enum ZeroKind {
      NonZero,
      Length,
      Percentage,
    }

    // If the basis is unitless 0, we must write all three components to disambiguate.
    // If the basis is 0%, we can omit the basis.
    let basis_kind = match &self.basis {
      FlexBasis::LengthPercentage(lp) => match lp {
        LengthPercentage::Dimension(l) if l.is_zero() => ZeroKind::Length,
        LengthPercentage::Percentage(p) if p.is_zero() => ZeroKind::Percentage,
        _ => ZeroKind::NonZero,
      },
      _ => ZeroKind::NonZero,
    };

    if self.grow != 1.0 || self.shrink != 1.0 || basis_kind != ZeroKind::NonZero {
      self.grow.to_css(dest)?;
      if self.shrink != 1.0 || basis_kind == ZeroKind::Length {
        dest.write_str(" ")?;
        self.shrink.to_css(dest)?;
      }
    }

    if basis_kind != ZeroKind::Percentage {
      if self.grow != 1.0 || self.shrink != 1.0 || basis_kind == ZeroKind::Length {
        dest.write_str(" ")?;
      }
      self.basis.to_css(dest)?;
    }

    Ok(())
  }
}

// Old flex (2009): https://www.w3.org/TR/2009/WD-css3-flexbox-20090723/

enum_property! {
  /// A value for the legacy (prefixed) [box-orient](https://www.w3.org/TR/2009/WD-css3-flexbox-20090723/#orientation) property.
  /// Partially equivalent to `flex-direction` in the standard syntax.
  pub enum BoxOrient {
    /// Items are laid out horizontally.
    Horizontal,
    /// Items are laid out vertically.
    Vertical,
    /// Items are laid out along the inline axis, according to the writing direction.
    InlineAxis,
    /// Items are laid out along the block axis, according to the writing direction.
    BlockAxis,
  }
}

impl FlexDirection {
  fn to_2009(&self) -> (BoxOrient, BoxDirection) {
    match self {
      FlexDirection::Row => (BoxOrient::Horizontal, BoxDirection::Normal),
      FlexDirection::Column => (BoxOrient::Vertical, BoxDirection::Normal),
      FlexDirection::RowReverse => (BoxOrient::Horizontal, BoxDirection::Reverse),
      FlexDirection::ColumnReverse => (BoxOrient::Vertical, BoxDirection::Reverse),
    }
  }
}

enum_property! {
  /// A value for the legacy (prefixed) [box-direction](https://www.w3.org/TR/2009/WD-css3-flexbox-20090723/#displayorder) property.
  /// Partially equivalent to the `flex-direction` property in the standard syntax.
  pub enum BoxDirection {
    /// Items flow in the natural direction.
    Normal,
    /// Items flow in the reverse direction.
    Reverse,
  }
}

enum_property! {
  /// A value for the legacy (prefixed) [box-align](https://www.w3.org/TR/2009/WD-css3-flexbox-20090723/#alignment) property.
  /// Equivalent to the `align-items` property in the standard syntax.
  pub enum BoxAlign {
    /// Items are aligned to the start.
    Start,
    /// Items are aligned to the end.
    End,
    /// Items are centered.
    Center,
    /// Items are aligned to the baseline.
    Baseline,
    /// Items are stretched.
    Stretch,
  }
}

impl FromStandard<AlignItems> for BoxAlign {
  fn from_standard(align: &AlignItems) -> Option<BoxAlign> {
    match align {
      AlignItems::SelfPosition { overflow: None, value } => match value {
        SelfPosition::Start | SelfPosition::FlexStart => Some(BoxAlign::Start),
        SelfPosition::End | SelfPosition::FlexEnd => Some(BoxAlign::End),
        SelfPosition::Center => Some(BoxAlign::Center),
        _ => None,
      },
      AlignItems::Stretch => Some(BoxAlign::Stretch),
      _ => None,
    }
  }
}

enum_property! {
  /// A value for the legacy (prefixed) [box-pack](https://www.w3.org/TR/2009/WD-css3-flexbox-20090723/#packing) property.
  /// Equivalent to the `justify-content` property in the standard syntax.
  pub enum BoxPack {
    /// Items are justified to the start.
    Start,
    /// Items are justified to the end.
    End,
    /// Items are centered.
    Center,
    /// Items are justified to the start and end.
    Justify,
  }
}

impl FromStandard<JustifyContent> for BoxPack {
  fn from_standard(justify: &JustifyContent) -> Option<BoxPack> {
    match justify {
      JustifyContent::ContentDistribution(cd) => match cd {
        ContentDistribution::SpaceBetween => Some(BoxPack::Justify),
        _ => None,
      },
      JustifyContent::ContentPosition { overflow: None, value } => match value {
        ContentPosition::Start | ContentPosition::FlexStart => Some(BoxPack::Start),
        ContentPosition::End | ContentPosition::FlexEnd => Some(BoxPack::End),
        ContentPosition::Center => Some(BoxPack::Center),
      },
      _ => None,
    }
  }
}

enum_property! {
  /// A value for the legacy (prefixed) [box-lines](https://www.w3.org/TR/2009/WD-css3-flexbox-20090723/#multiple) property.
  /// Equivalent to the `flex-wrap` property in the standard syntax.
  pub enum BoxLines {
    /// Items are laid out in a single line.
    Single,
    /// Items may wrap into multiple lines.
    Multiple,
  }
}

impl FromStandard<FlexWrap> for BoxLines {
  fn from_standard(wrap: &FlexWrap) -> Option<BoxLines> {
    match wrap {
      FlexWrap::NoWrap => Some(BoxLines::Single),
      FlexWrap::Wrap => Some(BoxLines::Multiple),
      _ => None,
    }
  }
}

type BoxOrdinalGroup = CSSInteger;
impl FromStandard<CSSInteger> for BoxOrdinalGroup {
  fn from_standard(order: &CSSInteger) -> Option<BoxOrdinalGroup> {
    Some(*order)
  }
}

// Old flex (2012): https://www.w3.org/TR/2012/WD-css3-flexbox-20120322/

enum_property! {
  /// A value for the legacy (prefixed) [flex-pack](https://www.w3.org/TR/2012/WD-css3-flexbox-20120322/#flex-pack) property.
  /// Equivalent to the `justify-content` property in the standard syntax.
  pub enum FlexPack {
    /// Items are justified to the start.
    Start,
    /// Items are justified to the end.
    End,
    /// Items are centered.
    Center,
    /// Items are justified to the start and end.
    Justify,
    /// Items are distributed evenly, with half size spaces on either end.
    Distribute,
  }
}

impl FromStandard<JustifyContent> for FlexPack {
  fn from_standard(justify: &JustifyContent) -> Option<FlexPack> {
    match justify {
      JustifyContent::ContentDistribution(cd) => match cd {
        ContentDistribution::SpaceBetween => Some(FlexPack::Justify),
        ContentDistribution::SpaceAround => Some(FlexPack::Distribute),
        _ => None,
      },
      JustifyContent::ContentPosition { overflow: None, value } => match value {
        ContentPosition::Start | ContentPosition::FlexStart => Some(FlexPack::Start),
        ContentPosition::End | ContentPosition::FlexEnd => Some(FlexPack::End),
        ContentPosition::Center => Some(FlexPack::Center),
      },
      _ => None,
    }
  }
}

/// A value for the legacy (prefixed) [flex-align](https://www.w3.org/TR/2012/WD-css3-flexbox-20120322/#flex-align) property.
pub type FlexAlign = BoxAlign;

enum_property! {
  /// A value for the legacy (prefixed) [flex-item-align](https://www.w3.org/TR/2012/WD-css3-flexbox-20120322/#flex-align) property.
  /// Equivalent to the `align-self` property in the standard syntax.
  pub enum FlexItemAlign {
    /// Equivalent to the value of `flex-align`.
    Auto,
    /// The item is aligned to the start.
    Start,
    /// The item is aligned to the end.
    End,
    /// The item is centered.
    Center,
    /// The item is aligned to the baseline.
    Baseline,
    /// The item is stretched.
    Stretch,
  }
}

impl FromStandard<AlignSelf> for FlexItemAlign {
  fn from_standard(justify: &AlignSelf) -> Option<FlexItemAlign> {
    match justify {
      AlignSelf::Auto => Some(FlexItemAlign::Auto),
      AlignSelf::Stretch => Some(FlexItemAlign::Stretch),
      AlignSelf::SelfPosition { overflow: None, value } => match value {
        SelfPosition::Start | SelfPosition::FlexStart => Some(FlexItemAlign::Start),
        SelfPosition::End | SelfPosition::FlexEnd => Some(FlexItemAlign::End),
        SelfPosition::Center => Some(FlexItemAlign::Center),
        _ => None,
      },
      _ => None,
    }
  }
}

enum_property! {
  /// A value for the legacy (prefixed) [flex-line-pack](https://www.w3.org/TR/2012/WD-css3-flexbox-20120322/#flex-line-pack) property.
  /// Equivalent to the `align-content` property in the standard syntax.
  pub enum FlexLinePack {
    /// Content is aligned to the start.
    Start,
    /// Content is aligned to the end.
    End,
    /// Content is centered.
    Center,
    /// Content is justified.
    Justify,
    /// Content is distributed evenly, with half size spaces on either end.
    Distribute,
    /// Content is stretched.
    Stretch,
  }
}

impl FromStandard<AlignContent> for FlexLinePack {
  fn from_standard(justify: &AlignContent) -> Option<FlexLinePack> {
    match justify {
      AlignContent::ContentDistribution(cd) => match cd {
        ContentDistribution::SpaceBetween => Some(FlexLinePack::Justify),
        ContentDistribution::SpaceAround => Some(FlexLinePack::Distribute),
        ContentDistribution::Stretch => Some(FlexLinePack::Stretch),
        _ => None,
      },
      AlignContent::ContentPosition { overflow: None, value } => match value {
        ContentPosition::Start | ContentPosition::FlexStart => Some(FlexLinePack::Start),
        ContentPosition::End | ContentPosition::FlexEnd => Some(FlexLinePack::End),
        ContentPosition::Center => Some(FlexLinePack::Center),
      },
      _ => None,
    }
  }
}

#[derive(Default, Debug)]
pub(crate) struct FlexHandler {
  direction: Option<(FlexDirection, VendorPrefix)>,
  box_orient: Option<(BoxOrient, VendorPrefix)>,
  box_direction: Option<(BoxDirection, VendorPrefix)>,
  wrap: Option<(FlexWrap, VendorPrefix)>,
  box_lines: Option<(BoxLines, VendorPrefix)>,
  grow: Option<(CSSNumber, VendorPrefix)>,
  box_flex: Option<(CSSNumber, VendorPrefix)>,
  flex_positive: Option<(CSSNumber, VendorPrefix)>,
  shrink: Option<(CSSNumber, VendorPrefix)>,
  flex_negative: Option<(CSSNumber, VendorPrefix)>,
  basis: Option<(FlexBasis, VendorPrefix)>,
  preferred_size: Option<(LengthPercentageOrAuto, VendorPrefix)>,
  order: Option<(CSSInteger, VendorPrefix)>,
  box_ordinal_group: Option<(BoxOrdinalGroup, VendorPrefix)>,
  flex_order: Option<(CSSInteger, VendorPrefix)>,
  has_any: bool,
}

impl<'i> PropertyHandler<'i> for FlexHandler {
  fn handle_property(
    &mut self,
    property: &Property<'i>,
    dest: &mut DeclarationList<'i>,
    context: &mut PropertyHandlerContext<'i, '_>,
  ) -> bool {
    use Property::*;

    macro_rules! maybe_flush {
      ($prop: ident, $val: expr, $vp: ident) => {{
        // If two vendor prefixes for the same property have different
        // values, we need to flush what we have immediately to preserve order.
        if let Some((val, prefixes)) = &self.$prop {
          if val != $val && !prefixes.contains(*$vp) {
            self.flush(dest, context);
          }
        }
      }};
    }

    macro_rules! property {
      ($prop: ident, $val: expr, $vp: ident) => {{
        maybe_flush!($prop, $val, $vp);

        // Otherwise, update the value and add the prefix.
        if let Some((val, prefixes)) = &mut self.$prop {
          *val = $val.clone();
          *prefixes |= *$vp;
        } else {
          self.$prop = Some(($val.clone(), *$vp));
          self.has_any = true;
        }
      }};
    }

    match property {
      FlexDirection(val, vp) => {
        if context.targets.browsers.is_some() {
          self.box_direction = None;
          self.box_orient = None;
        }
        property!(direction, val, vp);
      }
      BoxOrient(val, vp) => property!(box_orient, val, vp),
      BoxDirection(val, vp) => property!(box_direction, val, vp),
      FlexWrap(val, vp) => {
        if context.targets.browsers.is_some() {
          self.box_lines = None;
        }
        property!(wrap, val, vp);
      }
      BoxLines(val, vp) => property!(box_lines, val, vp),
      FlexFlow(val, vp) => {
        if context.targets.browsers.is_some() {
          self.box_direction = None;
          self.box_orient = None;
        }
        property!(direction, &val.direction, vp);
        property!(wrap, &val.wrap, vp);
      }
      FlexGrow(val, vp) => {
        if context.targets.browsers.is_some() {
          self.box_flex = None;
          self.flex_positive = None;
        }
        property!(grow, val, vp);
      }
      BoxFlex(val, vp) => property!(box_flex, val, vp),
      FlexPositive(val, vp) => property!(flex_positive, val, vp),
      FlexShrink(val, vp) => {
        if context.targets.browsers.is_some() {
          self.flex_negative = None;
        }
        property!(shrink, val, vp);
      }
      FlexNegative(val, vp) => property!(flex_negative, val, vp),
      FlexBasis(val, vp) => {
        if context.targets.browsers.is_some() {
          self.preferred_size = None;
        }
        property!(basis, val, vp);
      }
      FlexPreferredSize(val, vp) => property!(preferred_size, val, vp),
      Flex(val, vp) => {
        if context.targets.browsers.is_some() {
          self.box_flex = None;
          self.flex_positive = None;
          self.flex_negative = None;
          self.preferred_size = None;
        }
        maybe_flush!(grow, &val.grow, vp);
        maybe_flush!(shrink, &val.shrink, vp);
        maybe_flush!(basis, &val.basis, vp);
        property!(grow, &val.grow, vp);
        property!(shrink, &val.shrink, vp);
        property!(basis, &val.basis, vp);
      }
      Order(val, vp) => {
        if context.targets.browsers.is_some() {
          self.box_ordinal_group = None;
          self.flex_order = None;
        }
        property!(order, val, vp);
      }
      BoxOrdinalGroup(val, vp) => property!(box_ordinal_group, val, vp),
      FlexOrder(val, vp) => property!(flex_order, val, vp),
      Unparsed(val) if is_flex_property(&val.property_id) => {
        self.flush(dest, context);
        dest.push(property.clone()) // TODO: prefix?
      }
      _ => return false,
    }

    true
  }

  fn finalize(&mut self, dest: &mut DeclarationList<'i>, context: &mut PropertyHandlerContext<'i, '_>) {
    self.flush(dest, context);
  }
}

impl FlexHandler {
  fn flush<'i>(&mut self, dest: &mut DeclarationList<'i>, context: &mut PropertyHandlerContext<'i, '_>) {
    if !self.has_any {
      return;
    }

    self.has_any = false;

    let mut direction = std::mem::take(&mut self.direction);
    let mut wrap = std::mem::take(&mut self.wrap);
    let mut grow = std::mem::take(&mut self.grow);
    let mut shrink = std::mem::take(&mut self.shrink);
    let mut basis = std::mem::take(&mut self.basis);
    let box_orient = std::mem::take(&mut self.box_orient);
    let box_direction = std::mem::take(&mut self.box_direction);
    let box_flex = std::mem::take(&mut self.box_flex);
    let box_ordinal_group = std::mem::take(&mut self.box_ordinal_group);
    let box_lines = std::mem::take(&mut self.box_lines);
    let flex_positive = std::mem::take(&mut self.flex_positive);
    let flex_negative = std::mem::take(&mut self.flex_negative);
    let preferred_size = std::mem::take(&mut self.preferred_size);
    let order = std::mem::take(&mut self.order);
    let flex_order = std::mem::take(&mut self.flex_order);

    macro_rules! single_property {
      ($prop: ident, $key: ident $(, 2012: $prop_2012: ident )? $(, 2009: $prop_2009: ident )?) => {
        if let Some((val, prefix)) = $key {
          if !prefix.is_empty() {
            let mut prefix = context.targets.prefixes(prefix, Feature::$prop);
            if prefix.contains(VendorPrefix::None) {
              $(
                // 2009 spec, implemented by webkit and firefox.
                if let Some(targets) = context.targets.browsers {
                  let mut prefixes_2009 = VendorPrefix::empty();
                  if is_flex_2009(targets) {
                    prefixes_2009 |= VendorPrefix::WebKit;
                  }
                  if prefix.contains(VendorPrefix::Moz) {
                    prefixes_2009 |= VendorPrefix::Moz;
                  }
                  if !prefixes_2009.is_empty() {
                    if let Some(v) = $prop_2009::from_standard(&val) {
                      dest.push(Property::$prop_2009(v, prefixes_2009));
                    }
                  }
                }
              )?
            }

            $(
              let mut ms = true;
              if prefix.contains(VendorPrefix::Ms) {
                dest.push(Property::$prop_2012(val.clone(), VendorPrefix::Ms));
                ms = false;
              }
              if !ms {
                prefix.remove(VendorPrefix::Ms);
              }
            )?

            // Firefox only implemented the 2009 spec prefixed.
            prefix.remove(VendorPrefix::Moz);
            dest.push(Property::$prop(val, prefix))
          }
        }
      };
    }

    macro_rules! legacy_property {
      ($prop: ident, $key: expr) => {
        if let Some((val, prefix)) = $key {
          if !prefix.is_empty() {
            dest.push(Property::$prop(val, prefix))
          }
        }
      };
    }

    // Legacy properties. These are only set if the final standard properties were unset.
    legacy_property!(BoxOrient, box_orient);
    legacy_property!(BoxDirection, box_direction);
    legacy_property!(BoxOrdinalGroup, box_ordinal_group);
    legacy_property!(BoxFlex, box_flex);
    legacy_property!(BoxLines, box_lines);
    legacy_property!(FlexPositive, flex_positive);
    legacy_property!(FlexNegative, flex_negative);
    legacy_property!(FlexPreferredSize, preferred_size.clone());
    legacy_property!(FlexOrder, flex_order.clone());

    if let Some((direction, _)) = direction {
      if let Some(targets) = context.targets.browsers {
        let prefixes = context.targets.prefixes(VendorPrefix::None, Feature::FlexDirection);
        let mut prefixes_2009 = VendorPrefix::empty();
        if is_flex_2009(targets) {
          prefixes_2009 |= VendorPrefix::WebKit;
        }
        if prefixes.contains(VendorPrefix::Moz) {
          prefixes_2009 |= VendorPrefix::Moz;
        }
        if !prefixes_2009.is_empty() {
          let (orient, dir) = direction.to_2009();
          dest.push(Property::BoxOrient(orient, prefixes_2009));
          dest.push(Property::BoxDirection(dir, prefixes_2009));
        }
      }
    }

    if let (Some((direction, dir_prefix)), Some((wrap, wrap_prefix))) = (&mut direction, &mut wrap) {
      let intersection = *dir_prefix & *wrap_prefix;
      if !intersection.is_empty() {
        let mut prefix = context.targets.prefixes(intersection, Feature::FlexFlow);
        // Firefox only implemented the 2009 spec prefixed.
        prefix.remove(VendorPrefix::Moz);
        dest.push(Property::FlexFlow(
          FlexFlow {
            direction: *direction,
            wrap: *wrap,
          },
          prefix,
        ));
        dir_prefix.remove(intersection);
        wrap_prefix.remove(intersection);
      }
    }

    single_property!(FlexDirection, direction);
    single_property!(FlexWrap, wrap, 2009: BoxLines);

    if let Some(targets) = context.targets.browsers {
      if let Some((grow, _)) = grow {
        let prefixes = context.targets.prefixes(VendorPrefix::None, Feature::FlexGrow);
        let mut prefixes_2009 = VendorPrefix::empty();
        if is_flex_2009(targets) {
          prefixes_2009 |= VendorPrefix::WebKit;
        }
        if prefixes.contains(VendorPrefix::Moz) {
          prefixes_2009 |= VendorPrefix::Moz;
        }
        if !prefixes_2009.is_empty() {
          dest.push(Property::BoxFlex(grow, prefixes_2009));
        }
      }
    }

    if let (Some((grow, grow_prefix)), Some((shrink, shrink_prefix)), Some((basis, basis_prefix))) =
      (&mut grow, &mut shrink, &mut basis)
    {
      let intersection = *grow_prefix & *shrink_prefix & *basis_prefix;
      if !intersection.is_empty() {
        let mut prefix = context.targets.prefixes(intersection, Feature::Flex);
        // Firefox only implemented the 2009 spec prefixed.
        prefix.remove(VendorPrefix::Moz);
        dest.push(Property::Flex(
          Flex {
            grow: *grow,
            shrink: *shrink,
            basis: basis.clone(),
          },
          prefix,
        ));
        grow_prefix.remove(intersection);
        shrink_prefix.remove(intersection);
        basis_prefix.remove(intersection);
      }
    }

    single_property!(FlexGrow, grow, 2012: FlexPositive);
    single_property!(FlexShrink, shrink, 2012: FlexNegative);
    if let Some((value, prefix)) = basis {
      if !prefix.is_empty() {
        let mut prefix = context.targets.prefixes(prefix, Feature::FlexBasis);
        if prefix.contains(VendorPrefix::Ms) {
          if let Some(value) = LengthPercentageOrAuto::from_standard(&value) {
            dest.push(Property::FlexPreferredSize(value, VendorPrefix::Ms));
          }
          prefix.remove(VendorPrefix::Ms);
        }
        // Firefox only implemented the 2009 flexbox syntax with a prefix.
        prefix.remove(VendorPrefix::Moz);
        if !prefix.is_empty() {
          dest.push(Property::FlexBasis(value, prefix));
        }
      }
    }
    single_property!(Order, order, 2012: FlexOrder, 2009: BoxOrdinalGroup);
  }
}

#[inline]
fn is_flex_property(property_id: &PropertyId) -> bool {
  match property_id {
    PropertyId::FlexDirection(_)
    | PropertyId::BoxOrient(_)
    | PropertyId::BoxDirection(_)
    | PropertyId::FlexWrap(_)
    | PropertyId::BoxLines(_)
    | PropertyId::FlexFlow(_)
    | PropertyId::FlexGrow(_)
    | PropertyId::BoxFlex(_)
    | PropertyId::FlexPositive(_)
    | PropertyId::FlexShrink(_)
    | PropertyId::FlexNegative(_)
    | PropertyId::FlexBasis(_)
    | PropertyId::FlexPreferredSize(_)
    | PropertyId::Flex(_)
    | PropertyId::Order(_)
    | PropertyId::BoxOrdinalGroup(_)
    | PropertyId::FlexOrder(_) => true,
    _ => false,
  }
}

#[cfg(test)]
mod tests {
  use crate::printer::PrinterOptions;
  use crate::properties::{Property, PropertyId};
  use crate::stylesheet::ParserOptions;

  fn parse_typed<'i>(name: &'i str, source: &'i str) -> Property<'i> {
    let property = Property::parse_string(
      PropertyId::from(name),
      source,
      ParserOptions::default(),
    )
    .unwrap_or_else(|error| panic!("{name}: {source} should parse: {error:?}"));
    assert!(!matches!(property, Property::Unparsed(_)), "{name}: {source}");
    property
  }

  fn assert_typed_rejection(name: &str, source: &str) {
    let property = Property::parse_string(PropertyId::from(name), source, ParserOptions::default());
    assert!(
      property.is_err() || matches!(property, Ok(Property::Unparsed(_))),
      "{name}: {source} should not produce a typed property"
    );
  }

  #[test]
  fn parses_and_canonicalizes_balanced_flex_wrapping() {
    for (name, source, expected) in [
      ("flex-wrap", "balance", "balance"),
      ("flex-wrap", "wrap balance", "balance"),
      ("flex-wrap", "balance wrap", "balance"),
      ("flex-wrap", "wrap-reverse balance", "wrap-reverse balance"),
      ("flex-flow", "column balance", "column balance"),
      ("flex-flow", "wrap balance", "balance"),
    ] {
      let property = Property::parse_string(PropertyId::from(name), source, ParserOptions::default())
        .expect("balanced flex wrapping should parse");
      assert_eq!(
        property
          .value_to_css_string(PrinterOptions::default())
          .expect("balanced flex wrapping should serialize"),
        expected,
        "{name}: {source}"
      );
    }

    for source in ["nowrap balance", "balance nowrap", "balance balance", "wrap nowrap"] {
      let property = Property::parse_string(PropertyId::from("flex-wrap"), source, ParserOptions::default());
      assert!(
        property.is_err() || matches!(property, Ok(Property::Unparsed(_))),
        "flex-wrap: {source}"
      );
    }
  }

  #[test]
  fn parses_intrinsic_flex_basis_branches() {
    for source in [
      "auto",
      "content",
      "min-content",
      "max-content",
      "fit-content",
      "stretch",
      "10px",
      "10%",
    ] {
      let property = parse_typed("flex-basis", source);
      assert_eq!(
        property
          .value_to_css_string(PrinterOptions::default())
          .expect("flex-basis should serialize"),
        source
      );
    }

    for (source, expected) in [
      ("-webkit-min-content", "-webkit-min-content"),
      ("-moz-max-content", "-moz-max-content"),
      ("-webkit-fill-available", "-webkit-fill-available"),
      ("-moz-available", "-moz-available"),
    ] {
      let property = parse_typed("flex-basis", source);
      assert_eq!(
        property
          .value_to_css_string(PrinterOptions::default())
          .expect("legacy intrinsic flex-basis should serialize"),
        expected
      );
    }
  }

  #[test]
  fn parses_and_canonicalizes_flex_calc_size() {
    for (source, expected) in [
      ("calc-size(auto, size)", "calc-size(auto, size)"),
      (
        "calc-size(auto, size + 1px)",
        "calc-size(auto, 1px + size)",
      ),
      ("calc-size(auto, size * 2)", "calc-size(auto, 2 * size)"),
      ("calc-size(auto, size / 2)", "calc-size(auto, .5 * size)"),
      (
        "calc-size(auto, size / 2 + 1px)",
        "calc-size(auto, 1px + (.5 * size))",
      ),
      (
        "calc-size(auto, 1px - size / 2)",
        "calc-size(auto, 1px - (.5 * size))",
      ),
      (
        "calc-size(auto, min(size, 10px))",
        "calc-size(auto, min(size, 10px))",
      ),
      ("calc-size(any, 1px)", "calc-size(any, 1px)"),
      ("calc-size(any, 10%)", "calc-size(any, 10%)"),
      ("calc-size(10%, size)", "calc-size(10%, size)"),
      ("calc-size(1px + 2px, size)", "calc-size(3px, size)"),
      (
        "calc-size(calc-size(auto, size), size)",
        "calc-size(calc-size(auto, size), size)",
      ),
      (
        "calc-size(auto, round(size, 1px))",
        "calc-size(auto, round(size, 1px))",
      ),
      (
        "calc-size(auto, sign(size) * 1px)",
        "calc-size(auto, 1px * sign(size))",
      ),
    ] {
      let property = parse_typed("flex-basis", source);
      assert_eq!(
        property
          .value_to_css_string(PrinterOptions::default())
          .expect("calc-size should serialize"),
        expected,
        "flex-basis: {source}"
      );
    }
  }

  #[test]
  fn rejects_invalid_flex_calc_size_branches() {
    for source in [
      "calc-size(any, size)",
      "calc-size(any, min(size, 10px))",
      "calc-size(auto, 0)",
      "calc-size(auto, 1)",
      "calc-size(auto, 1deg)",
      "calc-size(auto size)",
      "calc-size(auto, size, 1px)",
      "calc-size(auto, calc-size(auto, size))",
    ] {
      assert_typed_rejection("flex-basis", source);
    }
  }

  #[test]
  fn parses_intrinsic_flex_shorthand_in_any_valid_order() {
    for source in [
      "content",
      "content 1 1",
      "1 content",
      "1 1 content",
      "min-content",
      "1 max-content",
      "2 3 fit-content",
      "stretch 2",
      "2 stretch",
      "2 3 stretch",
    ] {
      parse_typed("flex", source);
      parse_typed("-webkit-flex", source);
    }

    for source in [
      "calc-size(auto, size)",
      "contain",
      "fit-content(10px)",
      "content content",
      "1 2 3 content",
    ] {
      assert_typed_rejection("flex", source);
      assert_typed_rejection("-webkit-flex", source);
    }
  }
}
