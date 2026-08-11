//! CSS properties related to box sizing.

use crate::compat::Feature;
use crate::context::PropertyHandlerContext;
use crate::declaration::DeclarationList;
use crate::error::{ParserError, PrinterError};
use crate::logical::PropertyCategory;
use crate::macros::{enum_property, property_bitflags};
use crate::printer::Printer;
use crate::properties::{Property, PropertyId};
use crate::traits::{
  private::TryAdd, IsCompatible, Parse, PropertyHandler, ToCss, TryMap, TryOp, TrySign, Zero,
};
use crate::values::angle::Angle;
use crate::values::ident::DashedIdent;
use crate::values::length::{LengthPercentage, LengthValue};
use crate::values::number::CSSNumber;
use crate::values::percentage::{DimensionPercentage, Percentage};
use crate::values::ratio::Ratio;
use crate::vendor_prefix::VendorPrefix;
#[cfg(feature = "visitor")]
use crate::visitor::Visit;
use cssparser::*;

#[cfg(feature = "serde")]
use crate::serialization::*;

/// A value returned by the [`anchor-size()`](https://drafts.csswg.org/css-anchor-position/#anchor-size)
/// function.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "visitor", derive(Visit))]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "jsonschema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "into_owned", derive(static_self::IntoOwned))]
pub struct AnchorSize {
  /// The optional anchor name.
  pub anchor_name: Option<String>,
  /// The optional dimension of the anchor to query.
  pub dimension: Option<AnchorSizeDimension>,
  /// The optional fallback used when the anchor size cannot be resolved.
  pub fallback: Option<LengthPercentage>,
  multiplier: CSSNumber,
}

/// A dimension accepted by the `anchor-size()` function.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Parse, ToCss)]
#[cfg_attr(feature = "visitor", derive(Visit))]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize), serde(rename_all = "kebab-case"))]
#[cfg_attr(feature = "jsonschema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "into_owned", derive(static_self::IntoOwned))]
pub enum AnchorSizeDimension {
  /// The physical width of the anchor.
  Width,
  /// The physical height of the anchor.
  Height,
  /// The logical block size of the anchor in the containing block's writing mode.
  Block,
  /// The logical inline size of the anchor in the containing block's writing mode.
  Inline,
  /// The logical block size of the anchor in the anchored element's writing mode.
  SelfBlock,
  /// The logical inline size of the anchor in the anchored element's writing mode.
  SelfInline,
}

/// A length component accepted by sizing properties, including `anchor-size()`.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "visitor", derive(Visit))]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize), serde(tag = "type", content = "value", rename_all = "kebab-case"))]
#[cfg_attr(feature = "jsonschema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "into_owned", derive(static_self::IntoOwned))]
pub enum SizeLength {
  /// A regular CSS length.
  Length(LengthValue),
  /// An anchor-relative length.
  AnchorSize(AnchorSize),
}

/// A `<length-percentage>` accepted by sizing properties, including `anchor-size()`
/// within math expressions.
pub type SizeLengthPercentage = DimensionPercentage<SizeLength>;

/// Either an anchor-aware `<length-percentage>`, or the `auto` keyword.
///
/// This is used by inset and margin properties, which accept `anchor-size()`
/// in addition to the ordinary `<length-percentage>` grammar.
#[derive(Debug, Clone, PartialEq, Parse, ToCss)]
#[cfg_attr(feature = "visitor", derive(Visit))]
#[cfg_attr(
  feature = "serde",
  derive(serde::Serialize, serde::Deserialize),
  serde(tag = "type", content = "value", rename_all = "kebab-case")
)]
#[cfg_attr(feature = "jsonschema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "into_owned", derive(static_self::IntoOwned))]
pub enum AnchorSizeOrAuto {
  /// The `auto` keyword.
  Auto,
  /// An anchor-aware `<length-percentage>` value.
  LengthPercentage(SizeLengthPercentage),
}

/// A value returned by the [`anchor()`](https://drafts.csswg.org/css-anchor-position/#anchor-pos)
/// function.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "visitor", derive(Visit))]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "jsonschema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "into_owned", derive(static_self::IntoOwned))]
pub struct AnchorPosition {
  /// The optional anchor name.
  pub anchor_name: Option<String>,
  /// The side of the anchor to query.
  pub side: AnchorSide,
  /// The optional fallback used when the anchor cannot be resolved.
  pub fallback: Option<Box<InsetLengthPercentage>>,
  multiplier: CSSNumber,
}

/// A side accepted by the `anchor()` function.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "visitor", derive(Visit))]
#[cfg_attr(
  feature = "serde",
  derive(serde::Serialize, serde::Deserialize),
  serde(tag = "type", content = "value", rename_all = "kebab-case")
)]
#[cfg_attr(feature = "jsonschema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "into_owned", derive(static_self::IntoOwned))]
pub enum AnchorSide {
  /// A named physical, logical, or relative side.
  Keyword(AnchorSideKeyword),
  /// A percentage between the start and end sides.
  Percentage(Percentage),
}

/// A keyword accepted as an `anchor()` side.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Parse, ToCss)]
#[cfg_attr(feature = "visitor", derive(Visit))]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize), serde(rename_all = "kebab-case"))]
#[cfg_attr(feature = "jsonschema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "into_owned", derive(static_self::IntoOwned))]
pub enum AnchorSideKeyword {
  /// The side facing the positioned element.
  Inside,
  /// The side facing away from the positioned element.
  Outside,
  /// The physical top side.
  Top,
  /// The physical left side.
  Left,
  /// The physical right side.
  Right,
  /// The physical bottom side.
  Bottom,
  /// The logical start side.
  Start,
  /// The logical end side.
  End,
  /// The positioned element's logical start side.
  SelfStart,
  /// The positioned element's logical end side.
  SelfEnd,
  /// The center of the anchor.
  Center,
}

/// A length component accepted by inset properties, including anchor
/// positioning functions.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "visitor", derive(Visit))]
#[cfg_attr(
  feature = "serde",
  derive(serde::Serialize, serde::Deserialize),
  serde(tag = "type", content = "value", rename_all = "kebab-case")
)]
#[cfg_attr(feature = "jsonschema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "into_owned", derive(static_self::IntoOwned))]
pub enum InsetLength {
  /// An ordinary length value.
  Length(LengthValue),
  /// An `anchor-size()` function.
  AnchorSize(AnchorSize),
  /// An `anchor()` function.
  Anchor(AnchorPosition),
}

/// An inset-aware `<length-percentage>` that supports `anchor()` and
/// `anchor-size()` within math expressions.
pub type InsetLengthPercentage = DimensionPercentage<InsetLength>;

/// Either an anchor-aware inset value or the `auto` keyword.
#[derive(Debug, Clone, PartialEq, Parse, ToCss)]
#[cfg_attr(feature = "visitor", derive(Visit))]
#[cfg_attr(
  feature = "serde",
  derive(serde::Serialize, serde::Deserialize),
  serde(tag = "type", content = "value", rename_all = "kebab-case")
)]
#[cfg_attr(feature = "jsonschema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "into_owned", derive(static_self::IntoOwned))]
pub enum AnchorPositionOrAuto {
  /// The `auto` keyword.
  Auto,
  /// An inset-aware length-percentage value.
  LengthPercentage(InsetLengthPercentage),
}

impl IsCompatible for AnchorSizeOrAuto {
  fn is_compatible(&self, browsers: crate::targets::Browsers) -> bool {
    match self {
      AnchorSizeOrAuto::Auto => true,
      AnchorSizeOrAuto::LengthPercentage(value) => value.is_compatible(browsers),
    }
  }
}

impl IsCompatible for AnchorPositionOrAuto {
  fn is_compatible(&self, browsers: crate::targets::Browsers) -> bool {
    match self {
      AnchorPositionOrAuto::Auto => true,
      AnchorPositionOrAuto::LengthPercentage(value) => value.is_compatible(browsers),
    }
  }
}

impl<'i> Parse<'i> for AnchorPosition {
  fn parse<'t>(input: &mut Parser<'i, 't>) -> Result<Self, ParseError<'i, ParserError<'i>>> {
    input.expect_function_matching("anchor")?;
    input.parse_nested_block(|input| {
      let mut anchor_name = None;
      let mut side = None;
      for _ in 0..2 {
        if anchor_name.is_none() {
          if let Ok(name) = input.try_parse(parse_anchor_name) {
            anchor_name = Some(name);
            continue;
          }
        }
        if side.is_none() {
          if let Ok(value) = input.try_parse(AnchorSide::parse) {
            side = Some(value);
            continue;
          }
        }
        break;
      }
      let Some(side) = side else {
        return Err(input.new_custom_error(ParserError::InvalidValue));
      };
      let fallback = if input.try_parse(|input| input.expect_comma()).is_ok() {
        Some(Box::new(InsetLengthPercentage::parse(input)?))
      } else {
        None
      };
      Ok(AnchorPosition {
        anchor_name,
        side,
        fallback,
        multiplier: 1.0,
      })
    })
  }
}

impl ToCss for AnchorPosition {
  fn to_css<W>(&self, dest: &mut Printer<W>) -> Result<(), PrinterError>
  where
    W: std::fmt::Write,
  {
    let wrap_calc = self.multiplier != 1.0 && !dest.in_calc;
    if wrap_calc {
      dest.write_str("calc(")?;
      dest.in_calc = true;
    }
    let result = (|| {
      if self.multiplier != 1.0 {
        self.multiplier.to_css(dest)?;
        dest.write_str(" * ")?;
      }
      dest.write_str("anchor(")?;
      if let Some(name) = &self.anchor_name {
        dest.write_dashed_ident(name, true)?;
        dest.write_char(' ')?;
      }
      self.side.to_css(dest)?;
      if let Some(fallback) = &self.fallback {
        dest.delim(',', false)?;
        fallback.to_css(dest)?;
      }
      dest.write_char(')')
    })();
    if wrap_calc {
      dest.in_calc = false;
      result?;
      return dest.write_char(')');
    }
    result
  }
}

impl<'i> Parse<'i> for AnchorSide {
  fn parse<'t>(input: &mut Parser<'i, 't>) -> Result<Self, ParseError<'i, ParserError<'i>>> {
    if let Ok(value) = input.try_parse(AnchorSideKeyword::parse) {
      return Ok(AnchorSide::Keyword(value));
    }
    Percentage::parse(input).map(AnchorSide::Percentage)
  }
}

impl ToCss for AnchorSide {
  fn to_css<W>(&self, dest: &mut Printer<W>) -> Result<(), PrinterError>
  where
    W: std::fmt::Write,
  {
    match self {
      AnchorSide::Keyword(value) => value.to_css(dest),
      AnchorSide::Percentage(value) => value.to_css(dest),
    }
  }
}

impl<'i> Parse<'i> for AnchorSize {
  fn parse<'t>(input: &mut Parser<'i, 't>) -> Result<Self, ParseError<'i, ParserError<'i>>> {
    input.expect_function_matching("anchor-size")?;
    input.parse_nested_block(|input| {
      let mut anchor_name = None;
      let mut dimension = None;

      if input.is_exhausted() {
        return Ok(AnchorSize {
          anchor_name,
          dimension,
          fallback: None,
          multiplier: 1.0,
        });
      }

      if let Ok(fallback) = input.try_parse(LengthPercentage::parse) {
        return Ok(AnchorSize {
          anchor_name,
          dimension,
          fallback: Some(fallback),
          multiplier: 1.0,
        });
      }

      for _ in 0..2 {
        if anchor_name.is_none() {
          if let Ok(name) = input.try_parse(parse_anchor_name) {
            anchor_name = Some(name);
            continue;
          }
        }
        if dimension.is_none() {
          if let Ok(value) = input.try_parse(AnchorSizeDimension::parse) {
            dimension = Some(value);
            continue;
          }
        }
        break;
      }

      if anchor_name.is_none() && dimension.is_none() {
        return Err(input.new_custom_error(ParserError::InvalidValue));
      }

      let fallback = if input.try_parse(|input| input.expect_comma()).is_ok() {
        Some(LengthPercentage::parse(input)?)
      } else {
        None
      };

      Ok(AnchorSize {
        anchor_name,
        dimension,
        fallback,
        multiplier: 1.0,
      })
    })
  }
}

impl ToCss for AnchorSize {
  fn to_css<W>(&self, dest: &mut Printer<W>) -> Result<(), PrinterError>
  where
    W: std::fmt::Write,
  {
    let wrap_calc = self.multiplier != 1.0 && !dest.in_calc;
    if wrap_calc {
      dest.write_str("calc(")?;
      dest.in_calc = true;
    }
    let result = (|| {
      if self.multiplier != 1.0 {
        self.multiplier.to_css(dest)?;
        dest.write_str(" * ")?;
      }
      dest.write_str("anchor-size(")?;
      let mut has_component = false;
      if let Some(name) = &self.anchor_name {
        dest.write_dashed_ident(name, true)?;
        has_component = true;
      }
      if let Some(dimension) = self.dimension {
        if has_component {
          dest.write_char(' ')?;
        }
        dimension.to_css(dest)?;
        has_component = true;
      }
      if let Some(fallback) = &self.fallback {
        if has_component {
          dest.delim(',', false)?;
        }
        fallback.to_css(dest)?;
      }
      dest.write_char(')')
    })();
    if wrap_calc {
      dest.in_calc = false;
      result?;
      dest.write_char(')')?;
      return Ok(());
    }
    result
  }
}

impl<'i> Parse<'i> for SizeLength {
  fn parse<'t>(input: &mut Parser<'i, 't>) -> Result<Self, ParseError<'i, ParserError<'i>>> {
    if let Ok(value) = input.try_parse(AnchorSize::parse) {
      return Ok(SizeLength::AnchorSize(value));
    }
    LengthValue::parse(input).map(SizeLength::Length)
  }
}

impl ToCss for SizeLength {
  fn to_css<W>(&self, dest: &mut Printer<W>) -> Result<(), PrinterError>
  where
    W: std::fmt::Write,
  {
    match self {
      SizeLength::Length(value) => value.to_css(dest),
      SizeLength::AnchorSize(value) => value.to_css(dest),
    }
  }
}

impl std::ops::Mul<CSSNumber> for SizeLength {
  type Output = Self;

  fn mul(self, multiplier: CSSNumber) -> Self {
    match self {
      SizeLength::Length(value) => SizeLength::Length(value * multiplier),
      SizeLength::AnchorSize(mut value) => {
        value.multiplier *= multiplier;
        SizeLength::AnchorSize(value)
      }
    }
  }
}

impl TryAdd<SizeLength> for SizeLength {
  fn try_add(&self, other: &SizeLength) -> Option<SizeLength> {
    match (self, other) {
      (SizeLength::Length(a), SizeLength::Length(b)) => a.try_add(b).map(SizeLength::Length),
      _ => None,
    }
  }


  fn canonical_order(&self, other: &SizeLength) -> Option<std::cmp::Ordering> {
    match (self, other) {
      (SizeLength::Length(_), SizeLength::AnchorSize(_)) => Some(std::cmp::Ordering::Less),
      (SizeLength::AnchorSize(_), SizeLength::Length(_)) => Some(std::cmp::Ordering::Greater),
      _ => None,
    }
  }
}

impl PartialOrd<SizeLength> for SizeLength {
  fn partial_cmp(&self, other: &SizeLength) -> Option<std::cmp::Ordering> {
    match (self, other) {
      (SizeLength::Length(a), SizeLength::Length(b)) => a.partial_cmp(b),
      _ => None,
    }
  }
}

impl TryOp for SizeLength {
  fn try_op<F: FnOnce(f32, f32) -> f32>(&self, rhs: &Self, op: F) -> Option<Self> {
    match (self, rhs) {
      (SizeLength::Length(a), SizeLength::Length(b)) => a.try_op(b, op).map(SizeLength::Length),
      _ => None,
    }
  }

  fn try_op_to<T, F: FnOnce(f32, f32) -> T>(&self, rhs: &Self, op: F) -> Option<T> {
    match (self, rhs) {
      (SizeLength::Length(a), SizeLength::Length(b)) => a.try_op_to(b, op),
      _ => None,
    }
  }
}

impl TryMap for SizeLength {
  fn try_map<F: FnOnce(f32) -> f32>(&self, op: F) -> Option<Self> {
    match self {
      SizeLength::Length(value) => Some(SizeLength::Length(crate::traits::Map::map(value, op))),
      SizeLength::AnchorSize(_) => None,
    }
  }
}

impl Zero for SizeLength {
  fn zero() -> Self {
    SizeLength::Length(LengthValue::Px(0.0))
  }

  fn is_zero(&self) -> bool {
    matches!(self, SizeLength::Length(value) if value.is_zero())
  }
}

impl TrySign for SizeLength {
  fn try_sign(&self) -> Option<f32> {
    match self {
      SizeLength::Length(value) => Some(crate::traits::Sign::sign(value)),
      SizeLength::AnchorSize(_) => None,
    }
  }
}

impl TryFrom<Angle> for SizeLength {
  type Error = ();

  fn try_from(value: Angle) -> Result<Self, Self::Error> {
    LengthValue::try_from(value).map(SizeLength::Length)
  }
}

impl TryInto<Angle> for SizeLength {
  type Error = ();

  fn try_into(self) -> Result<Angle, Self::Error> {
    match self {
      SizeLength::Length(value) => value.try_into(),
      SizeLength::AnchorSize(_) => Err(()),
    }
  }
}

impl IsCompatible for AnchorSize {
  fn is_compatible(&self, browsers: crate::targets::Browsers) -> bool {
    Feature::AnchorSizeSize.is_compatible(browsers)
      && self
        .fallback
        .as_ref()
        .is_none_or(|fallback| fallback.is_compatible(browsers))
  }
}

impl IsCompatible for SizeLength {
  fn is_compatible(&self, browsers: crate::targets::Browsers) -> bool {
    match self {
      SizeLength::Length(value) => value.is_compatible(browsers),
      SizeLength::AnchorSize(value) => value.is_compatible(browsers),
    }
  }
}

impl IsCompatible for DimensionPercentage<SizeLength> {
  fn is_compatible(&self, browsers: crate::targets::Browsers) -> bool {
    match self {
      DimensionPercentage::Dimension(value) => value.is_compatible(browsers),
      DimensionPercentage::Percentage(_) => true,
      DimensionPercentage::Calc(value) => value.is_compatible(browsers),
    }
  }
}

impl<'i> Parse<'i> for InsetLength {
  fn parse<'t>(input: &mut Parser<'i, 't>) -> Result<Self, ParseError<'i, ParserError<'i>>> {
    if let Ok(value) = input.try_parse(AnchorPosition::parse) {
      return Ok(InsetLength::Anchor(value));
    }
    if let Ok(value) = input.try_parse(AnchorSize::parse) {
      return Ok(InsetLength::AnchorSize(value));
    }
    LengthValue::parse(input).map(InsetLength::Length)
  }
}

impl ToCss for InsetLength {
  fn to_css<W>(&self, dest: &mut Printer<W>) -> Result<(), PrinterError>
  where
    W: std::fmt::Write,
  {
    match self {
      InsetLength::Length(value) => value.to_css(dest),
      InsetLength::AnchorSize(value) => value.to_css(dest),
      InsetLength::Anchor(value) => value.to_css(dest),
    }
  }
}

impl std::ops::Mul<CSSNumber> for InsetLength {
  type Output = Self;

  fn mul(self, multiplier: CSSNumber) -> Self {
    match self {
      InsetLength::Length(value) => InsetLength::Length(value * multiplier),
      InsetLength::AnchorSize(mut value) => {
        value.multiplier *= multiplier;
        InsetLength::AnchorSize(value)
      }
      InsetLength::Anchor(mut value) => {
        value.multiplier *= multiplier;
        InsetLength::Anchor(value)
      }
    }
  }
}

impl TryAdd<InsetLength> for InsetLength {
  fn try_add(&self, other: &InsetLength) -> Option<InsetLength> {
    match (self, other) {
      (InsetLength::Length(a), InsetLength::Length(b)) => {
        a.try_add(b).map(InsetLength::Length)
      }
      _ => None,
    }
  }


  fn canonical_order(&self, other: &InsetLength) -> Option<std::cmp::Ordering> {
    match (self, other) {
      (InsetLength::Length(_), InsetLength::AnchorSize(_) | InsetLength::Anchor(_)) => {
        Some(std::cmp::Ordering::Less)
      }
      (InsetLength::AnchorSize(_) | InsetLength::Anchor(_), InsetLength::Length(_)) => {
        Some(std::cmp::Ordering::Greater)
      }
      _ => None,
    }
  }
}

impl PartialOrd<InsetLength> for InsetLength {
  fn partial_cmp(&self, other: &InsetLength) -> Option<std::cmp::Ordering> {
    match (self, other) {
      (InsetLength::Length(a), InsetLength::Length(b)) => a.partial_cmp(b),
      _ => None,
    }
  }
}

impl TryOp for InsetLength {
  fn try_op<F: FnOnce(f32, f32) -> f32>(&self, rhs: &Self, op: F) -> Option<Self> {
    match (self, rhs) {
      (InsetLength::Length(a), InsetLength::Length(b)) => {
        a.try_op(b, op).map(InsetLength::Length)
      }
      _ => None,
    }
  }

  fn try_op_to<T, F: FnOnce(f32, f32) -> T>(&self, rhs: &Self, op: F) -> Option<T> {
    match (self, rhs) {
      (InsetLength::Length(a), InsetLength::Length(b)) => a.try_op_to(b, op),
      _ => None,
    }
  }
}

impl TryMap for InsetLength {
  fn try_map<F: FnOnce(f32) -> f32>(&self, op: F) -> Option<Self> {
    match self {
      InsetLength::Length(value) => Some(InsetLength::Length(crate::traits::Map::map(value, op))),
      InsetLength::AnchorSize(_) | InsetLength::Anchor(_) => None,
    }
  }
}

impl Zero for InsetLength {
  fn zero() -> Self {
    InsetLength::Length(LengthValue::Px(0.0))
  }

  fn is_zero(&self) -> bool {
    matches!(self, InsetLength::Length(value) if value.is_zero())
  }
}

impl TrySign for InsetLength {
  fn try_sign(&self) -> Option<f32> {
    match self {
      InsetLength::Length(value) => Some(crate::traits::Sign::sign(value)),
      InsetLength::AnchorSize(_) | InsetLength::Anchor(_) => None,
    }
  }
}

impl TryFrom<Angle> for InsetLength {
  type Error = ();

  fn try_from(value: Angle) -> Result<Self, Self::Error> {
    LengthValue::try_from(value).map(InsetLength::Length)
  }
}

impl TryInto<Angle> for InsetLength {
  type Error = ();

  fn try_into(self) -> Result<Angle, Self::Error> {
    match self {
      InsetLength::Length(value) => value.try_into(),
      InsetLength::AnchorSize(_) | InsetLength::Anchor(_) => Err(()),
    }
  }
}

impl IsCompatible for AnchorPosition {
  fn is_compatible(&self, _browsers: crate::targets::Browsers) -> bool {
    false
  }
}

impl IsCompatible for InsetLength {
  fn is_compatible(&self, browsers: crate::targets::Browsers) -> bool {
    match self {
      InsetLength::Length(value) => value.is_compatible(browsers),
      InsetLength::AnchorSize(value) => value.is_compatible(browsers),
      InsetLength::Anchor(value) => value.is_compatible(browsers),
    }
  }
}

impl IsCompatible for DimensionPercentage<InsetLength> {
  fn is_compatible(&self, browsers: crate::targets::Browsers) -> bool {
    match self {
      DimensionPercentage::Dimension(value) => value.is_compatible(browsers),
      DimensionPercentage::Percentage(_) => true,
      DimensionPercentage::Calc(value) => value.is_compatible(browsers),
    }
  }
}

fn parse_anchor_name<'i, 't>(
  input: &mut Parser<'i, 't>,
) -> Result<String, ParseError<'i, ParserError<'i>>> {
  DashedIdent::parse(input).map(|name| name.0.to_string())
}

// https://drafts.csswg.org/css-sizing-3/#specifying-sizes
// https://www.w3.org/TR/css-sizing-4/#sizing-values

/// A value for the [preferred size properties](https://drafts.csswg.org/css-sizing-3/#preferred-size-properties),
/// i.e. `width` and `height.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "visitor", derive(Visit))]
#[cfg_attr(
  feature = "serde",
  derive(serde::Serialize, serde::Deserialize),
  serde(tag = "type", rename_all = "kebab-case")
)]
#[cfg_attr(feature = "jsonschema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "into_owned", derive(static_self::IntoOwned))]
pub enum Size {
  /// The `auto` keyword.
  Auto,
  /// An explicit length or percentage.
  #[cfg_attr(feature = "serde", serde(with = "ValueWrapper::<SizeLengthPercentage>"))]
  LengthPercentage(SizeLengthPercentage),
  /// The `min-content` keyword.
  #[cfg_attr(feature = "serde", serde(with = "PrefixWrapper"))]
  MinContent(VendorPrefix),
  /// The `max-content` keyword.
  #[cfg_attr(feature = "serde", serde(with = "PrefixWrapper"))]
  MaxContent(VendorPrefix),
  /// The `fit-content` keyword.
  #[cfg_attr(feature = "serde", serde(with = "PrefixWrapper"))]
  FitContent(VendorPrefix),
  /// The `fit-content()` function.
  #[cfg_attr(feature = "serde", serde(with = "ValueWrapper::<LengthPercentage>"))]
  FitContentFunction(LengthPercentage),
  /// The `stretch` keyword, or the `-webkit-fill-available` or `-moz-available` prefixed keywords.
  #[cfg_attr(feature = "serde", serde(with = "PrefixWrapper"))]
  Stretch(VendorPrefix),
  /// The `contain` keyword.
  Contain,
}

impl<'i> Parse<'i> for Size {
  fn parse<'t>(input: &mut Parser<'i, 't>) -> Result<Self, ParseError<'i, ParserError<'i>>> {
    let res = input.try_parse(|input| {
      let ident = input.expect_ident()?;
      Ok(match_ignore_ascii_case! { &*ident,
        "auto" => Size::Auto,
        "min-content" => Size::MinContent(VendorPrefix::None),
        "-webkit-min-content" => Size::MinContent(VendorPrefix::WebKit),
        "-moz-min-content" => Size::MinContent(VendorPrefix::Moz),
        "max-content" => Size::MaxContent(VendorPrefix::None),
        "-webkit-max-content" => Size::MaxContent(VendorPrefix::WebKit),
        "-moz-max-content" => Size::MaxContent(VendorPrefix::Moz),
        "stretch" => Size::Stretch(VendorPrefix::None),
        "-webkit-fill-available" => Size::Stretch(VendorPrefix::WebKit),
        "-moz-available" => Size::Stretch(VendorPrefix::Moz),
        "fit-content" => Size::FitContent(VendorPrefix::None),
        "-webkit-fit-content" => Size::FitContent(VendorPrefix::WebKit),
        "-moz-fit-content" => Size::FitContent(VendorPrefix::Moz),
        "contain" => Size::Contain,
        _ => return Err(input.new_custom_error(ParserError::InvalidValue))
      })
    });

    if res.is_ok() {
      return res;
    }

    if let Ok(res) = input.try_parse(parse_fit_content) {
      return Ok(Size::FitContentFunction(res));
    }

    let lp = input.try_parse(SizeLengthPercentage::parse)?;
    Ok(Size::LengthPercentage(lp))
  }
}

impl ToCss for Size {
  fn to_css<W>(&self, dest: &mut Printer<W>) -> Result<(), PrinterError>
  where
    W: std::fmt::Write,
  {
    use Size::*;
    match self {
      Auto => dest.write_str("auto"),
      Contain => dest.write_str("contain"),
      MinContent(vp) => {
        vp.to_css(dest)?;
        dest.write_str("min-content")
      }
      MaxContent(vp) => {
        vp.to_css(dest)?;
        dest.write_str("max-content")
      }
      FitContent(vp) => {
        vp.to_css(dest)?;
        dest.write_str("fit-content")
      }
      Stretch(vp) => match *vp {
        VendorPrefix::None => dest.write_str("stretch"),
        VendorPrefix::WebKit => dest.write_str("-webkit-fill-available"),
        VendorPrefix::Moz => dest.write_str("-moz-available"),
        _ => unreachable!(),
      },
      FitContentFunction(l) => {
        dest.write_str("fit-content(")?;
        l.to_css(dest)?;
        dest.write_str(")")
      }
      LengthPercentage(l) => l.to_css(dest),
    }
  }
}

impl IsCompatible for Size {
  fn is_compatible(&self, browsers: crate::targets::Browsers) -> bool {
    use Size::*;
    match self {
      LengthPercentage(l) => l.is_compatible(browsers),
      MinContent(..) => Feature::MinContentSize.is_compatible(browsers),
      MaxContent(..) => Feature::MaxContentSize.is_compatible(browsers),
      FitContent(..) => Feature::FitContentSize.is_compatible(browsers),
      FitContentFunction(l) => {
        Feature::FitContentFunctionSize.is_compatible(browsers) && l.is_compatible(browsers)
      }
      Stretch(vp) => match *vp {
        VendorPrefix::None => Feature::StretchSize,
        VendorPrefix::WebKit | VendorPrefix::Moz => Feature::WebkitFillAvailableSize,
        _ => return false,
      }
      .is_compatible(browsers),
      Contain => false, // ??? no data in mdn
      Auto => true,
    }
  }
}

/// A value for the [minimum](https://drafts.csswg.org/css-sizing-3/#min-size-properties)
/// and [maximum](https://drafts.csswg.org/css-sizing-3/#max-size-properties) size properties,
/// e.g. `min-width` and `max-height`.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "visitor", derive(Visit))]
#[cfg_attr(
  feature = "serde",
  derive(serde::Serialize, serde::Deserialize),
  serde(tag = "type", rename_all = "kebab-case")
)]
#[cfg_attr(feature = "jsonschema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "into_owned", derive(static_self::IntoOwned))]
pub enum MaxSize {
  /// The `none` keyword.
  None,
  /// An explicit length or percentage.
  #[cfg_attr(feature = "serde", serde(with = "ValueWrapper::<SizeLengthPercentage>"))]
  LengthPercentage(SizeLengthPercentage),
  /// The `min-content` keyword.
  #[cfg_attr(feature = "serde", serde(with = "PrefixWrapper"))]
  MinContent(VendorPrefix),
  /// The `max-content` keyword.
  #[cfg_attr(feature = "serde", serde(with = "PrefixWrapper"))]
  MaxContent(VendorPrefix),
  /// The `fit-content` keyword.
  #[cfg_attr(feature = "serde", serde(with = "PrefixWrapper"))]
  FitContent(VendorPrefix),
  /// The `fit-content()` function.
  #[cfg_attr(feature = "serde", serde(with = "ValueWrapper::<LengthPercentage>"))]
  FitContentFunction(LengthPercentage),
  /// The `stretch` keyword, or the `-webkit-fill-available` or `-moz-available` prefixed keywords.
  #[cfg_attr(feature = "serde", serde(with = "PrefixWrapper"))]
  Stretch(VendorPrefix),
  /// The `contain` keyword.
  Contain,
}

impl<'i> Parse<'i> for MaxSize {
  fn parse<'t>(input: &mut Parser<'i, 't>) -> Result<Self, ParseError<'i, ParserError<'i>>> {
    let res = input.try_parse(|input| {
      let ident = input.expect_ident()?;
      Ok(match_ignore_ascii_case! { &*ident,
        "none" => MaxSize::None,
        "min-content" => MaxSize::MinContent(VendorPrefix::None),
        "-webkit-min-content" => MaxSize::MinContent(VendorPrefix::WebKit),
        "-moz-min-content" => MaxSize::MinContent(VendorPrefix::Moz),
        "max-content" => MaxSize::MaxContent(VendorPrefix::None),
        "-webkit-max-content" => MaxSize::MaxContent(VendorPrefix::WebKit),
        "-moz-max-content" => MaxSize::MaxContent(VendorPrefix::Moz),
        "stretch" => MaxSize::Stretch(VendorPrefix::None),
        "-webkit-fill-available" => MaxSize::Stretch(VendorPrefix::WebKit),
        "-moz-available" => MaxSize::Stretch(VendorPrefix::Moz),
        "fit-content" => MaxSize::FitContent(VendorPrefix::None),
        "-webkit-fit-content" => MaxSize::FitContent(VendorPrefix::WebKit),
        "-moz-fit-content" => MaxSize::FitContent(VendorPrefix::Moz),
        "contain" => MaxSize::Contain,
        _ => return Err(input.new_custom_error(ParserError::InvalidValue))
      })
    });

    if res.is_ok() {
      return res;
    }

    if let Ok(res) = input.try_parse(parse_fit_content) {
      return Ok(MaxSize::FitContentFunction(res));
    }

    let lp = input.try_parse(SizeLengthPercentage::parse)?;
    Ok(MaxSize::LengthPercentage(lp))
  }
}

impl ToCss for MaxSize {
  fn to_css<W>(&self, dest: &mut Printer<W>) -> Result<(), PrinterError>
  where
    W: std::fmt::Write,
  {
    use MaxSize::*;
    match self {
      None => dest.write_str("none"),
      Contain => dest.write_str("contain"),
      MinContent(vp) => {
        vp.to_css(dest)?;
        dest.write_str("min-content")
      }
      MaxContent(vp) => {
        vp.to_css(dest)?;
        dest.write_str("max-content")
      }
      FitContent(vp) => {
        vp.to_css(dest)?;
        dest.write_str("fit-content")
      }
      Stretch(vp) => match *vp {
        VendorPrefix::None => dest.write_str("stretch"),
        VendorPrefix::WebKit => dest.write_str("-webkit-fill-available"),
        VendorPrefix::Moz => dest.write_str("-moz-available"),
        _ => unreachable!(),
      },
      FitContentFunction(l) => {
        dest.write_str("fit-content(")?;
        l.to_css(dest)?;
        dest.write_str(")")
      }
      LengthPercentage(l) => l.to_css(dest),
    }
  }
}

impl IsCompatible for MaxSize {
  fn is_compatible(&self, browsers: crate::targets::Browsers) -> bool {
    use MaxSize::*;
    match self {
      LengthPercentage(l) => l.is_compatible(browsers),
      MinContent(..) => Feature::MinContentSize.is_compatible(browsers),
      MaxContent(..) => Feature::MaxContentSize.is_compatible(browsers),
      FitContent(..) => Feature::FitContentSize.is_compatible(browsers),
      FitContentFunction(l) => {
        Feature::FitContentFunctionSize.is_compatible(browsers) && l.is_compatible(browsers)
      }
      Stretch(vp) => match *vp {
        VendorPrefix::None => Feature::StretchSize,
        VendorPrefix::WebKit | VendorPrefix::Moz => Feature::WebkitFillAvailableSize,
        _ => return false,
      }
      .is_compatible(browsers),
      Contain => false, // ??? no data in mdn
      None => true,
    }
  }
}

fn parse_fit_content<'i, 't>(
  input: &mut Parser<'i, 't>,
) -> Result<LengthPercentage, ParseError<'i, ParserError<'i>>> {
  input.expect_function_matching("fit-content")?;
  input.parse_nested_block(|input| LengthPercentage::parse(input))
}

enum_property! {
  /// A value for the [box-sizing](https://drafts.csswg.org/css-sizing-3/#box-sizing) property.
  pub enum BoxSizing {
    /// Exclude the margin/border/padding from the width and height.
    ContentBox,
    /// Include the padding and border (but not the margin) in the width and height.
    BorderBox,
  }
}

/// A value for the [aspect-ratio](https://drafts.csswg.org/css-sizing-4/#aspect-ratio) property.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "visitor", derive(Visit))]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "jsonschema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "into_owned", derive(static_self::IntoOwned))]
pub struct AspectRatio {
  /// The `auto` keyword.
  pub auto: bool,
  /// A preferred aspect ratio for the box, specified as width / height.
  pub ratio: Option<Ratio>,
}

impl<'i> Parse<'i> for AspectRatio {
  fn parse<'t>(input: &mut Parser<'i, 't>) -> Result<Self, ParseError<'i, ParserError<'i>>> {
    let location = input.current_source_location();
    let mut auto = input.try_parse(|i| i.expect_ident_matching("auto"));
    let ratio = input.try_parse(Ratio::parse);
    if auto.is_err() {
      auto = input.try_parse(|i| i.expect_ident_matching("auto"));
    }
    if auto.is_err() && ratio.is_err() {
      return Err(location.new_custom_error(ParserError::InvalidValue));
    }

    Ok(AspectRatio {
      auto: auto.is_ok(),
      ratio: ratio.ok(),
    })
  }
}

impl ToCss for AspectRatio {
  fn to_css<W>(&self, dest: &mut Printer<W>) -> Result<(), PrinterError>
  where
    W: std::fmt::Write,
  {
    if self.auto {
      dest.write_str("auto")?;
    }

    if let Some(ratio) = &self.ratio {
      if self.auto {
        dest.write_char(' ')?;
      }
      ratio.to_css(dest)?;
    }

    Ok(())
  }
}

property_bitflags! {
  #[derive(Default)]
  struct SizeProperty: u16 {
    const Width = 1 << 0;
    const Height = 1 << 1;
    const MinWidth = 1 << 2;
    const MinHeight = 1 << 3;
    const MaxWidth = 1 << 4;
    const MaxHeight = 1 << 5;
    const BlockSize = 1 << 6;
    const InlineSize = 1 << 7;
    const MinBlockSize  = 1 << 8;
    const MinInlineSize = 1 << 9;
    const MaxBlockSize = 1 << 10;
    const MaxInlineSize = 1 << 11;
  }
}

#[derive(Default)]
pub(crate) struct SizeHandler {
  width: Option<Size>,
  height: Option<Size>,
  min_width: Option<Size>,
  min_height: Option<Size>,
  max_width: Option<MaxSize>,
  max_height: Option<MaxSize>,
  block_size: Option<Size>,
  inline_size: Option<Size>,
  min_block_size: Option<Size>,
  min_inline_size: Option<Size>,
  max_block_size: Option<MaxSize>,
  max_inline_size: Option<MaxSize>,
  has_any: bool,
  flushed_properties: SizeProperty,
  category: PropertyCategory,
}

impl<'i> PropertyHandler<'i> for SizeHandler {
  fn handle_property(
    &mut self,
    property: &Property<'i>,
    dest: &mut DeclarationList<'i>,
    context: &mut PropertyHandlerContext<'i, '_>,
  ) -> bool {
    let logical_supported = !context.should_compile_logical(Feature::LogicalSize);

    macro_rules! property {
      ($prop: ident, $val: ident, $category: ident) => {{
        // If the category changes betweet logical and physical,
        // or if the value contains syntax that isn't supported across all targets,
        // preserve the previous value as a fallback.
        if PropertyCategory::$category != self.category || (self.$prop.is_some() && matches!(context.targets.browsers, Some(targets) if !$val.is_compatible(targets))) {
          self.flush(dest, context);
        }

        self.$prop = Some($val.clone());
        self.category = PropertyCategory::$category;
        self.has_any = true;
      }};
    }

    match property {
      Property::Width(v) => property!(width, v, Physical),
      Property::Height(v) => property!(height, v, Physical),
      Property::MinWidth(v) => property!(min_width, v, Physical),
      Property::MinHeight(v) => property!(min_height, v, Physical),
      Property::MaxWidth(v) => property!(max_width, v, Physical),
      Property::MaxHeight(v) => property!(max_height, v, Physical),
      Property::BlockSize(size) => property!(block_size, size, Logical),
      Property::MinBlockSize(size) => property!(min_block_size, size, Logical),
      Property::MaxBlockSize(size) => property!(max_block_size, size, Logical),
      Property::InlineSize(size) => property!(inline_size, size, Logical),
      Property::MinInlineSize(size) => property!(min_inline_size, size, Logical),
      Property::MaxInlineSize(size) => property!(max_inline_size, size, Logical),
      Property::Unparsed(unparsed) => {
        self.flush(dest, context);
        macro_rules! logical_unparsed {
          ($physical: ident) => {
            if logical_supported {
              self
                .flushed_properties
                .insert(SizeProperty::try_from(&unparsed.property_id).unwrap());
              dest.push(property.clone());
            } else {
              dest.push(Property::Unparsed(
                unparsed.with_property_id(PropertyId::$physical),
              ));
              self.flushed_properties.insert(SizeProperty::$physical);
            }
          };
        }

        match &unparsed.property_id {
          PropertyId::Width
          | PropertyId::Height
          | PropertyId::MinWidth
          | PropertyId::MaxWidth
          | PropertyId::MinHeight
          | PropertyId::MaxHeight => {
            self
              .flushed_properties
              .insert(SizeProperty::try_from(&unparsed.property_id).unwrap());
            dest.push(property.clone());
          }
          PropertyId::BlockSize => logical_unparsed!(Height),
          PropertyId::MinBlockSize => logical_unparsed!(MinHeight),
          PropertyId::MaxBlockSize => logical_unparsed!(MaxHeight),
          PropertyId::InlineSize => logical_unparsed!(Width),
          PropertyId::MinInlineSize => logical_unparsed!(MinWidth),
          PropertyId::MaxInlineSize => logical_unparsed!(MaxWidth),
          _ => return false,
        }
      }
      _ => return false,
    }

    true
  }

  fn finalize(&mut self, dest: &mut DeclarationList, context: &mut PropertyHandlerContext<'i, '_>) {
    self.flush(dest, context);
    self.flushed_properties = SizeProperty::empty();
  }
}

impl SizeHandler {
  fn flush<'i>(&mut self, dest: &mut DeclarationList, context: &mut PropertyHandlerContext<'i, '_>) {
    if !self.has_any {
      return;
    }

    self.has_any = false;
    let logical_supported = !context.should_compile_logical(Feature::LogicalSize);

    macro_rules! prefix {
      ($prop: ident, $size: ident, $feature: ident) => {
        if !self.flushed_properties.contains(SizeProperty::$prop) {
          let prefixes =
            context.targets.prefixes(VendorPrefix::None, crate::prefixes::Feature::$feature) - VendorPrefix::None;
          for prefix in prefixes {
            dest.push(Property::$prop($size::$feature(prefix)));
          }
        }
      };
    }

    macro_rules! property {
      ($prop: ident, $val: ident, $size: ident) => {{
        if let Some(val) = std::mem::take(&mut self.$val) {
          match val {
            $size::Stretch(VendorPrefix::None) => prefix!($prop, $size, Stretch),
            $size::MinContent(VendorPrefix::None) => prefix!($prop, $size, MinContent),
            $size::MaxContent(VendorPrefix::None) => prefix!($prop, $size, MaxContent),
            $size::FitContent(VendorPrefix::None) => prefix!($prop, $size, FitContent),
            _ => {}
          }
          dest.push(Property::$prop(val.clone()));
          self.flushed_properties.insert(SizeProperty::$prop);
        }
      }};
    }

    macro_rules! logical {
      ($prop: ident, $val: ident, $physical: ident, $size: ident) => {
        if logical_supported {
          property!($prop, $val, $size);
        } else {
          property!($physical, $val, $size);
        }
      };
    }

    property!(Width, width, Size);
    property!(MinWidth, min_width, Size);
    property!(MaxWidth, max_width, MaxSize);
    property!(Height, height, Size);
    property!(MinHeight, min_height, Size);
    property!(MaxHeight, max_height, MaxSize);
    logical!(BlockSize, block_size, Height, Size);
    logical!(MinBlockSize, min_block_size, MinHeight, Size);
    logical!(MaxBlockSize, max_block_size, MaxHeight, MaxSize);
    logical!(InlineSize, inline_size, Width, Size);
    logical!(MinInlineSize, min_inline_size, MinWidth, Size);
    logical!(MaxInlineSize, max_inline_size, MaxWidth, MaxSize);
  }
}

#[cfg(test)]
mod tests {
  use super::{AnchorPositionOrAuto, AnchorSizeOrAuto, MaxSize, Size};
  use crate::{printer::PrinterOptions, traits::{Parse, ToCss}};
  use cssparser::{Parser, ParserInput};

  fn parse_and_serialize<T>(source: &str) -> Result<String, ()>
  where
    for<'i> T: Parse<'i> + ToCss,
  {
    let mut input = ParserInput::new(source);
    let mut parser = Parser::new(&mut input);
    let value = parser.parse_entirely(T::parse).map_err(|_| ())?;
    value
      .to_css_string(PrinterOptions::default())
      .map_err(|_| ())
  }

  #[test]
  fn parses_anchor_size_across_sizing_properties() {
    for (source, expected) in [
      ("anchor-size()", "anchor-size()"),
      ("anchor-size(width)", "anchor-size(width)"),
      ("anchor-size(HEIGHT)", "anchor-size(height)"),
      ("anchor-size(block)", "anchor-size(block)"),
      ("anchor-size(inline)", "anchor-size(inline)"),
      ("anchor-size(self-block)", "anchor-size(self-block)"),
      ("anchor-size(self-inline)", "anchor-size(self-inline)"),
      ("anchor-size(--x)", "anchor-size(--x)"),
      ("anchor-size(width --x)", "anchor-size(--x width)"),
      ("anchor-size(\\2d\\2d x width)", "anchor-size(--x width)"),
      ("anchor-size(--x width, 10%)", "anchor-size(--x width, 10%)"),
      (
        "anchor-size(--x self-block, -1px)",
        "anchor-size(--x self-block, -1px)",
      ),
      ("anchor-size(10px)", "anchor-size(10px)"),
      (
        "anchor-size(width, calc(10px + 5%))",
        "anchor-size(width, calc(5% + 10px))",
      ),
      (
        "calc(anchor-size(width) * 2)",
        "calc(2 * anchor-size(width))",
      ),
      (
        "calc(anchor-size(width) / -2)",
        "calc(-.5 * anchor-size(width))",
      ),
      (
        "calc(anchor-size(width) + anchor-size(height))",
        "calc(anchor-size(width) + anchor-size(height))",
      ),
      (
        "calc(anchor-size(width) + 10px)",
        "calc(10px + anchor-size(width))",
      ),
      (
        "min(anchor-size(width), 10px)",
        "min(anchor-size(width), 10px)",
      ),
    ] {
      assert_eq!(parse_and_serialize::<Size>(source).unwrap(), expected, "{source}");
      assert_eq!(
        parse_and_serialize::<MaxSize>(source).unwrap(),
        expected,
        "{source}"
      );
    }
  }

  #[test]
  fn parses_anchor_positions_only_for_insets() {
    for (source, expected) in [
      ("anchor(inside)", "anchor(inside)"),
      ("anchor(--x inside)", "anchor(--x inside)"),
      ("anchor(inside --x)", "anchor(--x inside)"),
      ("anchor(-10%)", "anchor(-10%)"),
      ("anchor(--x self-end, 1px)", "anchor(--x self-end, 1px)"),
      (
        "anchor(--x inside, calc(1px + 5%))",
        "anchor(--x inside, calc(5% + 1px))",
      ),
      (
        "calc(anchor(inside) + 1px)",
        "calc(1px + anchor(inside))",
      ),
      (
        "calc(anchor(inside) * 2)",
        "calc(2 * anchor(inside))",
      ),
      ("min(anchor(inside), 10px)", "min(anchor(inside), 10px)"),
      (
        "anchor(inside, anchor-size(width))",
        "anchor(inside, anchor-size(width))",
      ),
      (
        "anchor(inside, anchor(outside, 1px))",
        "anchor(inside, anchor(outside, 1px))",
      ),
      (
        "anchor(inside, calc(anchor-size(width) + 1px))",
        "anchor(inside, calc(1px + anchor-size(width)))",
      ),
    ] {
      assert_eq!(
        parse_and_serialize::<AnchorPositionOrAuto>(source).unwrap(),
        expected,
        "{source}"
      );
      assert!(
        parse_and_serialize::<AnchorSizeOrAuto>(source).is_err(),
        "margin grammar accepted {source}"
      );
    }
  }

  #[test]
  fn rejects_invalid_anchor_position_neighbors() {
    for source in [
      "anchor()",
      "anchor(--x)",
      "anchor(inside,)",
      "anchor(inside, auto)",
      "anchor(inside outside)",
      "anchor(--x --y inside)",
      "anchor(inside, 1px --x)",
      "anchor(10px)",
    ] {
      assert!(
        parse_and_serialize::<AnchorPositionOrAuto>(source).is_err(),
        "{source}"
      );
    }
  }

  #[test]
  fn rejects_invalid_anchor_size_neighbors() {
    for source in [
      "anchor-size(, 10px)",
      "anchor-size(foo)",
      "anchor-size(width height)",
      "anchor-size(--x --y)",
      "anchor-size(width,)",
      "anchor-size(width, auto)",
      "anchor-size(10px, 20px)",
      "anchor-size(width 10px)",
      "anchor-size(--x 10px)",
      "anchor-size(none)",
      "anchor-size(default)",
      "fit-content(anchor-size(width))",
    ] {
      assert!(parse_and_serialize::<Size>(source).is_err(), "{source}");
      assert!(parse_and_serialize::<MaxSize>(source).is_err(), "{source}");
    }
  }

  #[test]
  fn preserves_existing_sizing_grammar() {
    for source in ["auto", "10px", "50%", "calc(10px + 5%)", "fit-content(20em)"] {
      assert!(parse_and_serialize::<Size>(source).is_ok(), "{source}");
    }
    for source in ["none", "10px", "50%", "calc(10px + 5%)", "fit-content(20em)"] {
      assert!(parse_and_serialize::<MaxSize>(source).is_ok(), "{source}");
    }
  }
}
