//! Shared typed representation for the `calc-size()` function.

use crate::error::{ParserError, PrinterError};
use crate::printer::Printer;
use crate::targets::Browsers;
use crate::traits::{private::TryAdd, IsCompatible, Parse, ToCss, TryMap, TryOp, TrySign, Zero};
use crate::values::angle::Angle;
use crate::values::calc::{Calc, MathFunction};
use crate::values::number::CSSNumber;
use crate::values::percentage::DimensionPercentage;
use cssparser::*;

#[cfg(feature = "visitor")]
use crate::visitor::Visit;

#[cfg(feature = "serde")]
use crate::serialization::*;

/// A typed `calc-size()` function whose basis grammar and length dimension are
/// supplied by the property that contains it.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "visitor", derive(Visit))]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "jsonschema", derive(schemars::JsonSchema))]
pub struct CalcSize<V, D> {
  /// The intrinsic, numeric, nested, or `any` basis.
  pub basis: CalcSizeBasis<V, D>,
  /// The calculation applied to the basis.
  pub calculation: Calc<CalcSizeLengthPercentage<D>>,
}

/// A property-specific basis accepted by `calc-size()`.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "visitor", derive(Visit))]
#[cfg_attr(
  feature = "serde",
  derive(serde::Serialize, serde::Deserialize),
  serde(tag = "type", content = "value", rename_all = "kebab-case")
)]
#[cfg_attr(feature = "jsonschema", derive(schemars::JsonSchema))]
pub enum CalcSizeBasis<V, D> {
  /// A basis that can interpolate with every size.
  Any,
  /// A complete value from the containing property's sizing grammar.
  Value(Box<V>),
  /// A bare length-percentage calculation.
  Calculation(Calc<DimensionPercentage<D>>),
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
pub enum CalcSizeLength<D> {
  /// An ordinary property-specific length.
  Length(D),
  /// The `size` placeholder, including an arithmetic multiplier.
  Size(CSSNumber),
}

/// A length-percentage calculation that can refer to the `size` placeholder.
pub type CalcSizeLengthPercentage<D> = DimensionPercentage<CalcSizeLength<D>>;

#[cfg(feature = "into_owned")]
impl<'any, V, D> static_self::IntoOwned<'any> for CalcSize<V, D>
where
  V: static_self::IntoOwned<'any>,
  D: static_self::IntoOwned<'any>,
  CalcSizeBasis<V, D>: static_self::IntoOwned<'any, Owned = CalcSizeBasis<V::Owned, D::Owned>>,
  Calc<CalcSizeLengthPercentage<D>>:
    static_self::IntoOwned<'any, Owned = Calc<CalcSizeLengthPercentage<D::Owned>>>,
{
  type Owned = CalcSize<V::Owned, D::Owned>;

  fn into_owned(self) -> Self::Owned {
    CalcSize {
      basis: static_self::IntoOwned::into_owned(self.basis),
      calculation: static_self::IntoOwned::into_owned(self.calculation),
    }
  }
}

#[cfg(feature = "into_owned")]
impl<'any, V, D> static_self::IntoOwned<'any> for CalcSizeBasis<V, D>
where
  V: static_self::IntoOwned<'any>,
  D: static_self::IntoOwned<'any>,
  Calc<DimensionPercentage<D>>: static_self::IntoOwned<'any, Owned = Calc<DimensionPercentage<D::Owned>>>,
{
  type Owned = CalcSizeBasis<V::Owned, D::Owned>;

  fn into_owned(self) -> Self::Owned {
    match self {
      Self::Any => CalcSizeBasis::Any,
      Self::Value(value) => CalcSizeBasis::Value(static_self::IntoOwned::into_owned(value)),
      Self::Calculation(value) => CalcSizeBasis::Calculation(static_self::IntoOwned::into_owned(value)),
    }
  }
}

#[cfg(feature = "into_owned")]
impl<'any, D> static_self::IntoOwned<'any> for CalcSizeLength<D>
where
  D: static_self::IntoOwned<'any>,
{
  type Owned = CalcSizeLength<D::Owned>;

  fn into_owned(self) -> Self::Owned {
    match self {
      Self::Length(value) => CalcSizeLength::Length(static_self::IntoOwned::into_owned(value)),
      Self::Size(value) => CalcSizeLength::Size(value),
    }
  }
}

pub(crate) trait CalcSizeDimension:
  for<'i> Parse<'i>
  + ToCss
  + std::ops::Mul<CSSNumber, Output = Self>
  + TryAdd<Self>
  + Clone
  + TryOp
  + TryMap
  + Zero
  + TrySign
  + TryFrom<Angle, Error = ()>
  + TryInto<Angle, Error = ()>
  + PartialOrd<Self>
  + std::fmt::Debug
  + IsCompatible
{
}

impl<D> CalcSizeDimension for D where
  D: for<'i> Parse<'i>
    + ToCss
    + std::ops::Mul<CSSNumber, Output = D>
    + TryAdd<D>
    + Clone
    + TryOp
    + TryMap
    + Zero
    + TrySign
    + TryFrom<Angle, Error = ()>
    + TryInto<Angle, Error = ()>
    + PartialOrd<D>
    + std::fmt::Debug
    + IsCompatible
{
}

impl<V, D> CalcSize<V, D> {
  /// Parses a `calc-size()` using the containing property's value parser for
  /// keyword and nested bases.
  pub(crate) fn parse_with<'i, 't, F>(
    input: &mut Parser<'i, 't>,
    parse_value: F,
  ) -> Result<Self, ParseError<'i, ParserError<'i>>>
  where
    D: CalcSizeDimension,
    F: Fn(&mut Parser<'i, '_>) -> Result<V, ParseError<'i, ParserError<'i>>>,
  {
    input.expect_function_matching("calc-size")?;
    input.parse_nested_block(|input| {
      let basis = if input
        .try_parse(|input| {
          input.expect_ident_matching("any")?;
          input.expect_comma()
        })
        .is_ok()
      {
        CalcSizeBasis::Any
      } else if let Ok(value) = input.try_parse(|input| {
        let value = parse_value(input)?;
        input.expect_comma()?;
        Ok::<_, ParseError<'i, ParserError<'i>>>(value)
      }) {
        CalcSizeBasis::Value(Box::new(value))
      } else {
        let calculation = Calc::<DimensionPercentage<D>>::parse_sum_with(input, |_| None)?;
        if !calculation.resolves_to_dimension() {
          return Err(input.new_custom_error(ParserError::InvalidValue));
        }
        input.expect_comma()?;
        CalcSizeBasis::Calculation(calculation)
      };

      let calculation = Calc::<CalcSizeLengthPercentage<D>>::parse_sum_with(input, |_| None)?;
      if !calculation.resolves_to_dimension() {
        return Err(input.new_custom_error(ParserError::InvalidValue));
      }
      if matches!(basis, CalcSizeBasis::Any) && calc_size_contains_size(&calculation) {
        return Err(input.new_custom_error(ParserError::InvalidValue));
      }

      // Chromium recovers a completed calculation and ignores subsequent
      // component values inside calc-size(). This remains confined to the
      // function block and cannot terminate the surrounding declaration.
      while input.next_including_whitespace_and_comments().is_ok() {}

      Ok(Self { basis, calculation })
    })
  }
}

fn calc_size_contains_size<D>(value: &Calc<CalcSizeLengthPercentage<D>>) -> bool {
  match value {
    Calc::Value(value) => match value.as_ref() {
      DimensionPercentage::Dimension(CalcSizeLength::Size(_)) => true,
      DimensionPercentage::Dimension(CalcSizeLength::Length(_)) | DimensionPercentage::Percentage(_) => false,
      DimensionPercentage::Calc(value) => calc_size_contains_size(value),
    },
    Calc::Number(_) => false,
    Calc::Sum(left, right) | Calc::ProductExpression(left, right) | Calc::QuotientExpression(left, right) => {
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
        calc_size_contains_size(min) || calc_size_contains_size(center) || calc_size_contains_size(max)
      }
      MathFunction::Round(_, value, step) | MathFunction::Rem(value, step) | MathFunction::Mod(value, step) => {
        calc_size_contains_size(value) || calc_size_contains_size(step)
      }
    },
  }
}

impl<V, D> ToCss for CalcSize<V, D>
where
  V: ToCss,
  D: CalcSizeDimension,
{
  fn to_css<W>(&self, dest: &mut Printer<W>) -> Result<(), PrinterError>
  where
    W: std::fmt::Write,
  {
    dest.write_str("calc-size(")?;
    self.basis.to_css(dest)?;
    dest.delim(',', false)?;
    serialize_calc_size_calculation(&self.calculation, dest)?;
    dest.write_char(')')
  }
}

impl<V, D> ToCss for CalcSizeBasis<V, D>
where
  V: ToCss,
  D: CalcSizeDimension,
{
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

fn serialize_calc_size_calculation<D, W>(
  value: &Calc<CalcSizeLengthPercentage<D>>,
  dest: &mut Printer<W>,
) -> Result<(), PrinterError>
where
  D: CalcSizeDimension,
  W: std::fmt::Write,
{
  let Calc::Sum(left, right) = value else {
    return value.to_css(dest);
  };

  serialize_calc_size_term(left, dest)?;
  if calc_size_term_is_negative(right) {
    dest.write_str(" - ")?;
    serialize_calc_size_term(&(right.as_ref().clone() * -1.0), dest)
  } else {
    dest.write_str(" + ")?;
    serialize_calc_size_term(right, dest)
  }
}

fn calc_size_term_is_negative<D>(value: &Calc<CalcSizeLengthPercentage<D>>) -> bool
where
  D: TrySign,
{
  match value {
    Calc::Value(value) => matches!(
      value.as_ref(),
      DimensionPercentage::Dimension(CalcSizeLength::Size(multiplier))
        if multiplier.is_sign_negative()
    ),
    _ => value.is_sign_negative(),
  }
}

fn serialize_calc_size_term<D, W>(
  value: &Calc<CalcSizeLengthPercentage<D>>,
  dest: &mut Printer<W>,
) -> Result<(), PrinterError>
where
  D: CalcSizeDimension,
  W: std::fmt::Write,
{
  if let Calc::Sum(_, _) = value {
    return serialize_calc_size_calculation(value, dest);
  }

  let scaled_size = matches!(
    value,
    Calc::Value(value)
      if matches!(
        value.as_ref(),
        DimensionPercentage::Dimension(CalcSizeLength::Size(multiplier))
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

impl<'i, D> Parse<'i> for CalcSizeLength<D>
where
  D: CalcSizeDimension,
{
  fn parse<'t>(input: &mut Parser<'i, 't>) -> Result<Self, ParseError<'i, ParserError<'i>>> {
    if input.try_parse(|input| input.expect_ident_matching("size")).is_ok() {
      return Ok(Self::Size(1.0));
    }
    D::parse(input).map(Self::Length)
  }
}

impl<D> ToCss for CalcSizeLength<D>
where
  D: CalcSizeDimension,
{
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

impl<D> std::ops::Mul<CSSNumber> for CalcSizeLength<D>
where
  D: std::ops::Mul<CSSNumber, Output = D>,
{
  type Output = Self;

  fn mul(self, multiplier: CSSNumber) -> Self {
    match self {
      Self::Length(value) => Self::Length(value * multiplier),
      Self::Size(value) => Self::Size(value * multiplier),
    }
  }
}

impl<D> TryAdd<CalcSizeLength<D>> for CalcSizeLength<D>
where
  D: TryAdd<D>,
{
  fn try_add(&self, other: &Self) -> Option<Self> {
    match (self, other) {
      (Self::Length(left), Self::Length(right)) => left.try_add(right).map(Self::Length),
      _ => None,
    }
  }

  fn canonical_order(&self, other: &Self) -> Option<std::cmp::Ordering> {
    match (self, other) {
      (Self::Length(_), Self::Size(_)) => Some(std::cmp::Ordering::Less),
      (Self::Size(_), Self::Length(_)) => Some(std::cmp::Ordering::Greater),
      (Self::Length(left), Self::Length(right)) => left.canonical_order(right),
      _ => None,
    }
  }
}

impl<D> PartialOrd<CalcSizeLength<D>> for CalcSizeLength<D>
where
  D: PartialOrd<D>,
{
  fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
    match (self, other) {
      (Self::Length(left), Self::Length(right)) => left.partial_cmp(right),
      _ => None,
    }
  }
}

impl<D> TryOp for CalcSizeLength<D>
where
  D: TryOp,
{
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

impl<D> TryMap for CalcSizeLength<D>
where
  D: TryMap,
{
  fn try_map<F: FnOnce(f32) -> f32>(&self, op: F) -> Option<Self> {
    match self {
      Self::Length(value) => value.try_map(op).map(Self::Length),
      Self::Size(_) => None,
    }
  }
}

impl<D> Zero for CalcSizeLength<D>
where
  D: Zero,
{
  fn zero() -> Self {
    Self::Length(D::zero())
  }

  fn is_zero(&self) -> bool {
    matches!(self, Self::Length(value) if value.is_zero())
  }
}

impl<D> TrySign for CalcSizeLength<D>
where
  D: TrySign,
{
  fn try_sign(&self) -> Option<f32> {
    match self {
      Self::Length(value) => value.try_sign(),
      Self::Size(_) => None,
    }
  }
}

impl<D> TryFrom<Angle> for CalcSizeLength<D>
where
  D: TryFrom<Angle, Error = ()>,
{
  type Error = ();

  fn try_from(value: Angle) -> Result<Self, Self::Error> {
    D::try_from(value).map(Self::Length)
  }
}

impl<D> TryInto<Angle> for CalcSizeLength<D>
where
  D: TryInto<Angle, Error = ()>,
{
  type Error = ();

  fn try_into(self) -> Result<Angle, Self::Error> {
    match self {
      Self::Length(value) => value.try_into(),
      Self::Size(_) => Err(()),
    }
  }
}

impl<D> IsCompatible for CalcSizeLength<D>
where
  D: IsCompatible,
{
  fn is_compatible(&self, browsers: Browsers) -> bool {
    match self {
      Self::Length(value) => value.is_compatible(browsers),
      Self::Size(_) => false,
    }
  }
}

impl<D> IsCompatible for DimensionPercentage<CalcSizeLength<D>>
where
  D: IsCompatible,
{
  fn is_compatible(&self, browsers: Browsers) -> bool {
    match self {
      DimensionPercentage::Dimension(value) => value.is_compatible(browsers),
      DimensionPercentage::Percentage(_) => true,
      DimensionPercentage::Calc(value) => value.is_compatible(browsers),
    }
  }
}
