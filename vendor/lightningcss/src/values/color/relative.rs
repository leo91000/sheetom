use super::{
  ColorSpace, ColorSpaceName, ComponentParser, CssColor, FloatColor, HueInterpolationMethod, LABColor, ParserError,
  PredefinedColor, RGB,
};
use crate::{
  printer::Printer,
  targets::Features,
  traits::{Parse, ToCss},
  values::{angle::Angle, calc::RoundingStrategy},
};
use cssparser::{Parser, Token};

use crate::error::PrinterError;

const MAX_RELATIVE_COLOR_NESTING_DEPTH: usize = 500;

/// A relative color whose origin or channel expressions must be resolved at computed-value time.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "into_owned", derive(static_self::IntoOwned))]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "jsonschema", derive(schemars::JsonSchema))]
pub struct RelativeColor {
  function: RelativeColorFunction,
  origin: RelativeColorOrigin,
  components: [RelativeColorComponent; 3],
  alpha: Option<RelativeColorComponent>,
}

impl RelativeColor {
  pub(super) fn origin_features(&self) -> Features {
    self.origin.features()
  }
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "into_owned", derive(static_self::IntoOwned))]
#[cfg_attr(
  feature = "serde",
  derive(serde::Serialize, serde::Deserialize),
  serde(tag = "type", content = "value", rename_all = "kebab-case")
)]
#[cfg_attr(feature = "jsonschema", derive(schemars::JsonSchema))]
enum RelativeColorOrigin {
  Keyword { name: String, color: CssColor },
  Value(CssColor),
  Contrast(Box<RelativeColorOrigin>),
  LightDark(Box<RelativeColorOrigin>, Box<RelativeColorOrigin>),
  ColorMix(Box<RelativeColorMix>),
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "into_owned", derive(static_self::IntoOwned))]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "jsonschema", derive(schemars::JsonSchema))]
struct RelativeColorMix {
  color_space: ColorSpaceName,
  hue_interpolation_method: HueInterpolationMethod,
  first: RelativeColorOrigin,
  first_percentage: Option<f32>,
  second: RelativeColorOrigin,
  second_percentage: Option<f32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "into_owned", derive(static_self::IntoOwned))]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize), serde(rename_all = "kebab-case"))]
#[cfg_attr(feature = "jsonschema", derive(schemars::JsonSchema))]
enum RelativeColorFunction {
  RGB,
  HSL,
  HWB,
  LAB,
  LCH,
  OKLAB,
  OKLCH,
  Color(RelativeColorSpace),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "into_owned", derive(static_self::IntoOwned))]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize), serde(rename_all = "kebab-case"))]
#[cfg_attr(feature = "jsonschema", derive(schemars::JsonSchema))]
enum RelativeColorSpace {
  SRGB,
  SRGBLinear,
  DisplayP3,
  DisplayP3Linear,
  A98RGB,
  ProPhotoRGB,
  Rec2020,
  XYZd50,
  XYZd65,
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "into_owned", derive(static_self::IntoOwned))]
#[cfg_attr(
  feature = "serde",
  derive(serde::Serialize, serde::Deserialize),
  serde(tag = "type", content = "value", rename_all = "kebab-case")
)]
#[cfg_attr(feature = "jsonschema", derive(schemars::JsonSchema))]
enum RelativeColorComponent {
  None,
  Value(RelativeColorExpression),
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "into_owned", derive(static_self::IntoOwned))]
#[cfg_attr(
  feature = "serde",
  derive(serde::Serialize, serde::Deserialize),
  serde(tag = "type", content = "value", rename_all = "kebab-case")
)]
#[cfg_attr(feature = "jsonschema", derive(schemars::JsonSchema))]
enum RelativeColorExpression {
  Number(f32),
  Percentage(f32),
  Angle(Angle),
  Channel(RelativeColorChannel),
  Constant(RelativeColorConstant),
  Group(Box<RelativeColorExpression>),
  Add(Box<RelativeColorExpression>, Box<RelativeColorExpression>),
  Subtract(Box<RelativeColorExpression>, Box<RelativeColorExpression>),
  Multiply(Box<RelativeColorExpression>, Box<RelativeColorExpression>),
  Divide(Box<RelativeColorExpression>, Box<RelativeColorExpression>),
  Function(Box<RelativeMathFunction>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "into_owned", derive(static_self::IntoOwned))]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize), serde(rename_all = "lowercase"))]
#[cfg_attr(feature = "jsonschema", derive(schemars::JsonSchema))]
enum RelativeColorChannel {
  R,
  G,
  B,
  H,
  S,
  L,
  W,
  A,
  C,
  X,
  Y,
  Z,
  Alpha,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "into_owned", derive(static_self::IntoOwned))]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize), serde(rename_all = "kebab-case"))]
#[cfg_attr(feature = "jsonschema", derive(schemars::JsonSchema))]
enum RelativeColorConstant {
  E,
  Pi,
  Infinity,
  NegativeInfinity,
  NaN,
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "into_owned", derive(static_self::IntoOwned))]
#[cfg_attr(
  feature = "serde",
  derive(serde::Serialize, serde::Deserialize),
  serde(tag = "type", rename_all = "kebab-case")
)]
#[cfg_attr(feature = "jsonschema", derive(schemars::JsonSchema))]
enum RelativeMathFunction {
  Calc {
    value: RelativeColorExpression,
  },
  Min {
    values: Vec<RelativeColorExpression>,
  },
  Max {
    values: Vec<RelativeColorExpression>,
  },
  Clamp {
    min: RelativeColorExpression,
    center: RelativeColorExpression,
    max: RelativeColorExpression,
  },
  Round {
    strategy: RoundingStrategy,
    value: RelativeColorExpression,
    step: Option<RelativeColorExpression>,
  },
  Rem {
    dividend: RelativeColorExpression,
    divisor: RelativeColorExpression,
  },
  Mod {
    dividend: RelativeColorExpression,
    divisor: RelativeColorExpression,
  },
  Abs {
    value: RelativeColorExpression,
  },
  Sign {
    value: RelativeColorExpression,
  },
  Hypot {
    values: Vec<RelativeColorExpression>,
  },
  Sin {
    value: RelativeColorExpression,
  },
  Cos {
    value: RelativeColorExpression,
  },
  Tan {
    value: RelativeColorExpression,
  },
  Asin {
    value: RelativeColorExpression,
  },
  Acos {
    value: RelativeColorExpression,
  },
  Atan {
    value: RelativeColorExpression,
  },
  Atan2 {
    y: RelativeColorExpression,
    x: RelativeColorExpression,
  },
  Pow {
    base: RelativeColorExpression,
    exponent: RelativeColorExpression,
  },
  Log {
    value: RelativeColorExpression,
    base: Option<RelativeColorExpression>,
  },
  Sqrt {
    value: RelativeColorExpression,
  },
  Exp {
    value: RelativeColorExpression,
  },
  SiblingIndex,
  SiblingCount,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum NumericType {
  Number,
  Percentage,
  Angle,
}

#[derive(Clone, Copy)]
struct RelativeColorParseContext {
  function: RelativeColorFunction,
  depth: usize,
}

impl RelativeColorParseContext {
  fn new(function: RelativeColorFunction) -> Self {
    Self { function, depth: 0 }
  }

  fn descend<'i, 't>(
    self,
    input: &mut Parser<'i, 't>,
  ) -> Result<Self, cssparser::ParseError<'i, ParserError<'i>>> {
    if self.depth >= MAX_RELATIVE_COLOR_NESTING_DEPTH {
      return Err(input.new_custom_error(ParserError::MaximumNestingDepth));
    }
    Ok(Self {
      function: self.function,
      depth: self.depth + 1,
    })
  }
}

impl RelativeColorFunction {
  fn from_name(name: &str) -> Option<Self> {
    if name.eq_ignore_ascii_case("rgb") || name.eq_ignore_ascii_case("rgba") {
      return Some(Self::RGB);
    }
    if name.eq_ignore_ascii_case("hsl") || name.eq_ignore_ascii_case("hsla") {
      return Some(Self::HSL);
    }
    if name.eq_ignore_ascii_case("hwb") {
      return Some(Self::HWB);
    }
    if name.eq_ignore_ascii_case("lab") {
      return Some(Self::LAB);
    }
    if name.eq_ignore_ascii_case("lch") {
      return Some(Self::LCH);
    }
    if name.eq_ignore_ascii_case("oklab") {
      return Some(Self::OKLAB);
    }
    if name.eq_ignore_ascii_case("oklch") {
      return Some(Self::OKLCH);
    }
    if name.eq_ignore_ascii_case("color") {
      return Some(Self::Color(RelativeColorSpace::SRGB));
    }
    None
  }

  fn name(self) -> &'static str {
    match self {
      Self::RGB => "rgb",
      Self::HSL => "hsl",
      Self::HWB => "hwb",
      Self::LAB => "lab",
      Self::LCH => "lch",
      Self::OKLAB => "oklab",
      Self::OKLCH => "oklch",
      Self::Color(_) => "color",
    }
  }

  fn allows_channel(self, channel: RelativeColorChannel) -> bool {
    use RelativeColorChannel::*;
    match self {
      Self::RGB => matches!(channel, R | G | B | Alpha),
      Self::HSL => matches!(channel, H | S | L | Alpha),
      Self::HWB => matches!(channel, H | W | B | Alpha),
      Self::LAB | Self::OKLAB => matches!(channel, L | A | B | Alpha),
      Self::LCH | Self::OKLCH => matches!(channel, L | C | H | Alpha),
      Self::Color(space) if space.is_xyz() => matches!(channel, X | Y | Z | Alpha),
      Self::Color(_) => matches!(channel, R | G | B | Alpha),
    }
  }

  fn allows_component_type(self, index: usize, numeric_type: NumericType) -> bool {
    if numeric_type == NumericType::Number {
      return true;
    }
    if matches!(self, Self::HSL | Self::HWB) && index == 0 {
      return numeric_type == NumericType::Angle;
    }
    if matches!(self, Self::LCH | Self::OKLCH) && index == 2 {
      return numeric_type == NumericType::Angle;
    }
    numeric_type == NumericType::Percentage
  }
}

impl RelativeColorSpace {
  fn parse<'i, 't>(input: &mut Parser<'i, 't>) -> Result<Self, cssparser::ParseError<'i, ParserError<'i>>> {
    let location = input.current_source_location();
    let identifier = input.expect_ident_cloned()?;
    let space = if identifier.eq_ignore_ascii_case("srgb") {
      Self::SRGB
    } else if identifier.eq_ignore_ascii_case("srgb-linear") {
      Self::SRGBLinear
    } else if identifier.eq_ignore_ascii_case("display-p3") {
      Self::DisplayP3
    } else if identifier.eq_ignore_ascii_case("display-p3-linear") {
      Self::DisplayP3Linear
    } else if identifier.eq_ignore_ascii_case("a98-rgb") {
      Self::A98RGB
    } else if identifier.eq_ignore_ascii_case("prophoto-rgb") {
      Self::ProPhotoRGB
    } else if identifier.eq_ignore_ascii_case("rec2020") {
      Self::Rec2020
    } else if identifier.eq_ignore_ascii_case("xyz-d50") {
      Self::XYZd50
    } else if identifier.eq_ignore_ascii_case("xyz") || identifier.eq_ignore_ascii_case("xyz-d65") {
      Self::XYZd65
    } else {
      return Err(location.new_unexpected_token_error(Token::Ident(identifier)));
    };
    Ok(space)
  }

  fn name(self) -> &'static str {
    match self {
      Self::SRGB => "srgb",
      Self::SRGBLinear => "srgb-linear",
      Self::DisplayP3 => "display-p3",
      Self::DisplayP3Linear => "display-p3-linear",
      Self::A98RGB => "a98-rgb",
      Self::ProPhotoRGB => "prophoto-rgb",
      Self::Rec2020 => "rec2020",
      Self::XYZd50 => "xyz-d50",
      Self::XYZd65 => "xyz-d65",
    }
  }

  fn is_xyz(self) -> bool {
    matches!(self, Self::XYZd50 | Self::XYZd65)
  }
}

impl RelativeColorChannel {
  fn parse(name: &str) -> Option<Self> {
    if name.eq_ignore_ascii_case("r") {
      return Some(Self::R);
    }
    if name.eq_ignore_ascii_case("g") {
      return Some(Self::G);
    }
    if name.eq_ignore_ascii_case("b") {
      return Some(Self::B);
    }
    if name.eq_ignore_ascii_case("h") {
      return Some(Self::H);
    }
    if name.eq_ignore_ascii_case("s") {
      return Some(Self::S);
    }
    if name.eq_ignore_ascii_case("l") {
      return Some(Self::L);
    }
    if name.eq_ignore_ascii_case("w") {
      return Some(Self::W);
    }
    if name.eq_ignore_ascii_case("a") {
      return Some(Self::A);
    }
    if name.eq_ignore_ascii_case("c") {
      return Some(Self::C);
    }
    if name.eq_ignore_ascii_case("x") {
      return Some(Self::X);
    }
    if name.eq_ignore_ascii_case("y") {
      return Some(Self::Y);
    }
    if name.eq_ignore_ascii_case("z") {
      return Some(Self::Z);
    }
    if name.eq_ignore_ascii_case("alpha") {
      return Some(Self::Alpha);
    }
    None
  }

  fn name(self) -> &'static str {
    match self {
      Self::R => "r",
      Self::G => "g",
      Self::B => "b",
      Self::H => "h",
      Self::S => "s",
      Self::L => "l",
      Self::W => "w",
      Self::A => "a",
      Self::C => "c",
      Self::X => "x",
      Self::Y => "y",
      Self::Z => "z",
      Self::Alpha => "alpha",
    }
  }
}

impl RelativeColorConstant {
  fn parse(name: &str) -> Option<Self> {
    if name.eq_ignore_ascii_case("e") {
      return Some(Self::E);
    }
    if name.eq_ignore_ascii_case("pi") {
      return Some(Self::Pi);
    }
    if name.eq_ignore_ascii_case("infinity") {
      return Some(Self::Infinity);
    }
    if name.eq_ignore_ascii_case("-infinity") {
      return Some(Self::NegativeInfinity);
    }
    if name.eq_ignore_ascii_case("nan") {
      return Some(Self::NaN);
    }
    None
  }

  fn name(self) -> &'static str {
    match self {
      Self::E => "e",
      Self::Pi => "pi",
      Self::Infinity => "infinity",
      Self::NegativeInfinity => "-infinity",
      Self::NaN => "NaN",
    }
  }
}

impl RelativeColorOrigin {
  fn parse<'i, 't>(input: &mut Parser<'i, 't>) -> Result<Self, cssparser::ParseError<'i, ParserError<'i>>> {
    let state = input.state();
    let location = input.current_source_location();
    let token = input.next()?.clone();
    match token {
      Token::Ident(identifier) => {
        input.reset(&state);
        let color = CssColor::parse(input)?;
        Ok(Self::Keyword {
          name: identifier.to_ascii_lowercase(),
          color,
        })
      }
      Token::Function(name) if has_relative_color_prefix(&name, input) => {
        input.reset(&state);
        CssColor::parse(input).map(Self::Value)
      }
      Token::Function(name) if name.eq_ignore_ascii_case("rgb") || name.eq_ignore_ascii_case("rgba") => {
        parse_origin_rgb(input).map(Self::Value)
      }
      Token::Function(name) if name.eq_ignore_ascii_case("hsl") || name.eq_ignore_ascii_case("hsla") => {
        parse_origin_hsl(input).map(Self::Value)
      }
      Token::Function(name) if name.eq_ignore_ascii_case("hwb") => parse_origin_hwb(input).map(Self::Value),
      Token::Function(name) if name.eq_ignore_ascii_case("color-mix") => input
        .parse_nested_block(RelativeColorMix::parse)
        .map(|value| Self::ColorMix(Box::new(value))),
      Token::Function(name) if name.eq_ignore_ascii_case("contrast-color") => input.parse_nested_block(|input| {
        let color = Self::parse(input)?;
        input.expect_exhausted()?;
        Ok(Self::Contrast(Box::new(color)))
      }),
      Token::Function(name) if name.eq_ignore_ascii_case("light-dark") => input.parse_nested_block(|input| {
        let light = Self::parse(input)?;
        input.expect_comma()?;
        let dark = Self::parse(input)?;
        input.expect_exhausted()?;
        Ok(Self::LightDark(Box::new(light), Box::new(dark)))
      }),
      _ => {
        input.reset(&state);
        CssColor::parse(input)
          .map(Self::Value)
          .map_err(|_| location.new_unexpected_token_error(token))
      }
    }
  }

  fn features(&self) -> Features {
    match self {
      Self::Keyword { color, .. } | Self::Value(color) => color.get_features(),
      Self::Contrast(color) => color.features(),
      Self::LightDark(light, dark) => light.features() | dark.features(),
      Self::ColorMix(mix) => mix.first.features() | mix.second.features(),
    }
  }
}

impl RelativeColorMix {
  fn parse<'i, 't>(input: &mut Parser<'i, 't>) -> Result<Self, cssparser::ParseError<'i, ParserError<'i>>> {
    let (color_space, hue_interpolation_method) = if input
      .try_parse(|input| input.expect_ident_matching("in"))
      .is_ok()
    {
      let color_space = ColorSpaceName::parse(input)?;
      let hue_interpolation_method = if matches!(
        color_space,
        ColorSpaceName::Hsl | ColorSpaceName::Hwb | ColorSpaceName::LCH | ColorSpaceName::OKLCH
      ) {
        input
          .try_parse(|input| -> Result<HueInterpolationMethod, cssparser::ParseError<'i, ParserError<'i>>> {
            let method = HueInterpolationMethod::parse(input)?;
            input.expect_ident_matching("hue")?;
            Ok(method)
          })
          .unwrap_or(HueInterpolationMethod::Shorter)
      } else {
        HueInterpolationMethod::Shorter
      };
      input.expect_comma()?;
      (color_space, hue_interpolation_method)
    } else {
      (ColorSpaceName::OKLAB, HueInterpolationMethod::Shorter)
    };

    let leading_first_percentage = input.try_parse(|input| input.expect_percentage()).ok();
    let first = RelativeColorOrigin::parse(input)?;
    let first_percentage = leading_first_percentage.or_else(|| input.try_parse(|input| input.expect_percentage()).ok());
    input.expect_comma()?;

    let leading_second_percentage = input.try_parse(|input| input.expect_percentage()).ok();
    let second = RelativeColorOrigin::parse(input)?;
    let second_percentage =
      leading_second_percentage.or_else(|| input.try_parse(|input| input.expect_percentage()).ok());

    let first_weight = first_percentage.unwrap_or_else(|| 1.0 - second_percentage.unwrap_or(0.5));
    let second_weight = second_percentage.unwrap_or_else(|| 1.0 - first_percentage.unwrap_or(0.5));
    if first_weight + second_weight == 0.0 {
      return Err(input.new_custom_error(ParserError::InvalidValue));
    }

    Ok(Self {
      color_space,
      hue_interpolation_method,
      first,
      first_percentage,
      second,
      second_percentage,
    })
  }
}

fn parse_origin_rgb<'i, 't>(
  input: &mut Parser<'i, 't>,
) -> Result<CssColor, cssparser::ParseError<'i, ParserError<'i>>> {
  input.parse_nested_block(|input| {
    let mut parser = ComponentParser::new(true);
    let (r, g, b, legacy) = super::parse_rgb_components(input, &mut parser)?;
    let alpha = if legacy {
      super::parse_legacy_alpha(input, &parser)?
    } else {
      super::parse_alpha(input, &parser)?
    };
    Ok(CssColor::Float(Box::new(FloatColor::RGB(RGB { r, g, b, alpha }))))
  })
}

fn parse_origin_hsl<'i, 't>(
  input: &mut Parser<'i, 't>,
) -> Result<CssColor, cssparser::ParseError<'i, ParserError<'i>>> {
  input.parse_nested_block(|input| {
    let mut parser = ComponentParser::new(true);
    let (h, s, l, legacy) = super::parse_hsl_hwb_components::<super::HSL>(input, &mut parser, true)?;
    let alpha = if legacy {
      super::parse_legacy_alpha(input, &parser)?
    } else {
      super::parse_alpha(input, &parser)?
    };
    Ok(CssColor::Float(Box::new(FloatColor::HSL(super::HSL { h, s, l, alpha }))))
  })
}

fn parse_origin_hwb<'i, 't>(
  input: &mut Parser<'i, 't>,
) -> Result<CssColor, cssparser::ParseError<'i, ParserError<'i>>> {
  input.parse_nested_block(|input| {
    let mut parser = ComponentParser::new(true);
    let (h, w, b, _) = super::parse_hsl_hwb_components::<super::HWB>(input, &mut parser, false)?;
    let alpha = super::parse_alpha(input, &parser)?;
    Ok(CssColor::Float(Box::new(FloatColor::HWB(super::HWB { h, w, b, alpha }))))
  })
}

pub(super) fn parse_relative_color<'i, 't>(
  name: &str,
  input: &mut Parser<'i, 't>,
) -> Result<RelativeColor, cssparser::ParseError<'i, ParserError<'i>>> {
  let Some(mut function) = RelativeColorFunction::from_name(name) else {
    return Err(input.new_custom_error(ParserError::InvalidValue));
  };

  input.parse_nested_block(|input| {
    input.expect_ident_matching("from")?;
    let origin = RelativeColorOrigin::parse(input)?;
    if matches!(function, RelativeColorFunction::Color(_)) {
      function = RelativeColorFunction::Color(RelativeColorSpace::parse(input)?);
    }

    let context = RelativeColorParseContext::new(function);

    let components = [
      parse_component(input, context, 0)?,
      parse_component(input, context, 1)?,
      parse_component(input, context, 2)?,
    ];
    let alpha = if input.try_parse(|input| input.expect_delim('/')).is_ok() {
      Some(parse_alpha_component(input, context)?)
    } else {
      None
    };
    input.expect_exhausted()?;
    Ok(RelativeColor {
      function,
      origin,
      components,
      alpha,
    })
  })
}

pub(super) fn has_relative_color_prefix<'i, 't>(name: &str, input: &mut Parser<'i, 't>) -> bool {
  if RelativeColorFunction::from_name(name).is_none() {
    return false;
  }

  let state = input.state();
  let mut has_prefix = false;
  let _: Result<(), cssparser::ParseError<'i, ParserError<'i>>> = input.parse_nested_block(|input| {
    has_prefix = input
      .try_parse(|input| input.expect_ident_matching("from"))
      .is_ok();
    Err(input.new_custom_error(ParserError::InvalidValue))
  });
  input.reset(&state);
  has_prefix
}

fn parse_component<'i, 't>(
  input: &mut Parser<'i, 't>,
  context: RelativeColorParseContext,
  index: usize,
) -> Result<RelativeColorComponent, cssparser::ParseError<'i, ParserError<'i>>> {
  if input.try_parse(|input| input.expect_ident_matching("none")).is_ok() {
    return Ok(RelativeColorComponent::None);
  }
  let expression = parse_expression_value(input, context, false)?;
  let Some(numeric_type) = expression.numeric_type() else {
    return Err(input.new_custom_error(ParserError::InvalidValue));
  };
  if !context.function.allows_component_type(index, numeric_type) {
    return Err(input.new_custom_error(ParserError::InvalidValue));
  }
  Ok(RelativeColorComponent::Value(expression))
}

fn parse_alpha_component<'i, 't>(
  input: &mut Parser<'i, 't>,
  context: RelativeColorParseContext,
) -> Result<RelativeColorComponent, cssparser::ParseError<'i, ParserError<'i>>> {
  if input.try_parse(|input| input.expect_ident_matching("none")).is_ok() {
    return Ok(RelativeColorComponent::None);
  }
  let expression = parse_expression_value(input, context, false)?;
  if !matches!(
    expression.numeric_type(),
    Some(NumericType::Number | NumericType::Percentage)
  ) {
    return Err(input.new_custom_error(ParserError::InvalidValue));
  }
  Ok(RelativeColorComponent::Value(expression))
}

fn parse_expression_value<'i, 't>(
  input: &mut Parser<'i, 't>,
  context: RelativeColorParseContext,
  allow_constants: bool,
) -> Result<RelativeColorExpression, cssparser::ParseError<'i, ParserError<'i>>> {
  let location = input.current_source_location();
  let token = input.next()?.clone();
  match token {
    Token::Number { value, .. } => Ok(RelativeColorExpression::Number(value)),
    Token::Percentage { unit_value, .. } => Ok(RelativeColorExpression::Percentage(unit_value)),
    token @ Token::Dimension { .. } => Angle::try_from(&token)
      .map(RelativeColorExpression::Angle)
      .map_err(|_| location.new_unexpected_token_error(token)),
    Token::Ident(identifier) => {
      if allow_constants {
        if let Some(constant) = RelativeColorConstant::parse(&identifier) {
          return Ok(RelativeColorExpression::Constant(constant));
        }
      }
      let Some(channel) = RelativeColorChannel::parse(&identifier) else {
        return Err(location.new_unexpected_token_error(Token::Ident(identifier)));
      };
      if !context.function.allows_channel(channel) {
        return Err(input.new_custom_error(ParserError::InvalidValue));
      }
      Ok(RelativeColorExpression::Channel(channel))
    }
    Token::Function(name) => {
      let nested_context = context.descend(input)?;
      parse_math_function(&name, input, nested_context)
    }
    Token::ParenthesisBlock => {
      let nested_context = context.descend(input)?;
      input.parse_nested_block(|input| {
        let value = parse_sum(input, nested_context)?;
        input.expect_exhausted()?;
        Ok(RelativeColorExpression::Group(Box::new(value)))
      })
    }
    token => Err(location.new_unexpected_token_error(token)),
  }
}

fn parse_sum<'i, 't>(
  input: &mut Parser<'i, 't>,
  context: RelativeColorParseContext,
) -> Result<RelativeColorExpression, cssparser::ParseError<'i, ParserError<'i>>> {
  let mut expression = parse_product(input, context)?;
  loop {
    let state = input.state();
    if !consume_whitespace(input) {
      input.reset(&state);
      break;
    }
    let operator = match input.next_including_whitespace_and_comments() {
      Ok(Token::Delim(operator @ ('+' | '-'))) => *operator,
      _ => {
        input.reset(&state);
        break;
      }
    };
    if !consume_whitespace(input) {
      return Err(input.new_custom_error(ParserError::InvalidValue));
    }
    let right = parse_product(input, context)?;
    let Some(numeric_type) = expression.numeric_type() else {
      return Err(input.new_custom_error(ParserError::InvalidValue));
    };
    if right.numeric_type() != Some(numeric_type) {
      return Err(input.new_custom_error(ParserError::InvalidValue));
    }
    expression = if operator == '+' {
      canonical_add(expression, right)
    } else {
      canonical_subtract(expression, right)
    };
  }
  Ok(expression)
}

fn consume_whitespace<'i, 't>(input: &mut Parser<'i, 't>) -> bool {
  let mut found_whitespace = false;
  loop {
    let state = input.state();
    match input.next_including_whitespace_and_comments() {
      Ok(Token::WhiteSpace(_)) => found_whitespace = true,
      Ok(Token::Comment(_)) => {}
      _ => {
        input.reset(&state);
        return found_whitespace;
      }
    }
  }
}

fn parse_product<'i, 't>(
  input: &mut Parser<'i, 't>,
  context: RelativeColorParseContext,
) -> Result<RelativeColorExpression, cssparser::ParseError<'i, ParserError<'i>>> {
  let mut expression = parse_expression_value(input, context, true)?;
  loop {
    let state = input.state();
    let operator = match input.next() {
      Ok(Token::Delim(operator @ ('*' | '/'))) => *operator,
      _ => {
        input.reset(&state);
        break;
      }
    };
    let right = parse_expression_value(input, context, true)?;
    let Some(left_type) = expression.numeric_type() else {
      return Err(input.new_custom_error(ParserError::InvalidValue));
    };
    let Some(right_type) = right.numeric_type() else {
      return Err(input.new_custom_error(ParserError::InvalidValue));
    };
    let result_type = if operator == '*' {
      multiply_type(left_type, right_type)
    } else {
      divide_type(left_type, right_type)
    };
    if result_type.is_none() {
      return Err(input.new_custom_error(ParserError::InvalidValue));
    }
    expression = if operator == '*' {
      canonical_multiply(expression, right)
    } else {
      canonical_divide(expression, right)
    };
  }
  Ok(expression)
}

fn multiply_type(left: NumericType, right: NumericType) -> Option<NumericType> {
  match (left, right) {
    (NumericType::Number, numeric_type) | (numeric_type, NumericType::Number) => Some(numeric_type),
    _ => None,
  }
}

fn divide_type(left: NumericType, right: NumericType) -> Option<NumericType> {
  if right == NumericType::Number {
    return Some(left);
  }
  if left == right {
    return Some(NumericType::Number);
  }
  None
}

fn canonical_add(
  left: RelativeColorExpression,
  right: RelativeColorExpression,
) -> RelativeColorExpression {
  if matches!(right, RelativeColorExpression::Number(_))
    && !matches!(left, RelativeColorExpression::Number(_))
  {
    return RelativeColorExpression::Add(Box::new(right), Box::new(left));
  }
  RelativeColorExpression::Add(Box::new(left), Box::new(right))
}

fn canonical_subtract(
  left: RelativeColorExpression,
  right: RelativeColorExpression,
) -> RelativeColorExpression {
  if let RelativeColorExpression::Number(value) = right {
    return canonical_add(RelativeColorExpression::Number(-value), left);
  }
  RelativeColorExpression::Subtract(Box::new(left), Box::new(right))
}

fn canonical_multiply(
  left: RelativeColorExpression,
  right: RelativeColorExpression,
) -> RelativeColorExpression {
  if matches!(right, RelativeColorExpression::Number(_))
    && !matches!(left, RelativeColorExpression::Number(_))
  {
    return RelativeColorExpression::Multiply(Box::new(right), Box::new(left));
  }
  RelativeColorExpression::Multiply(Box::new(left), Box::new(right))
}

fn canonical_divide(
  left: RelativeColorExpression,
  right: RelativeColorExpression,
) -> RelativeColorExpression {
  if let RelativeColorExpression::Number(value) = right {
    if value != 0.0 && value.is_finite() {
      return canonical_multiply(RelativeColorExpression::Number(value.recip()), left);
    }
    return RelativeColorExpression::Divide(
      Box::new(left),
      Box::new(RelativeColorExpression::Number(value)),
    );
  }
  RelativeColorExpression::Divide(Box::new(left), Box::new(right))
}

fn parse_math_function<'i, 't>(
  name: &str,
  input: &mut Parser<'i, 't>,
  context: RelativeColorParseContext,
) -> Result<RelativeColorExpression, cssparser::ParseError<'i, ParserError<'i>>> {
  let math = input.parse_nested_block(|input| {
    let math = if name.eq_ignore_ascii_case("calc") {
      RelativeMathFunction::Calc {
        value: parse_sum(input, context)?,
      }
    } else if name.eq_ignore_ascii_case("min") {
      RelativeMathFunction::Min {
        values: parse_same_type_list(input, context)?,
      }
    } else if name.eq_ignore_ascii_case("max") {
      RelativeMathFunction::Max {
        values: parse_same_type_list(input, context)?,
      }
    } else if name.eq_ignore_ascii_case("clamp") {
      let min = parse_sum(input, context)?;
      input.expect_comma()?;
      let center = parse_sum(input, context)?;
      input.expect_comma()?;
      let max = parse_sum(input, context)?;
      require_same_types(input, [&min, &center, &max])?;
      RelativeMathFunction::Clamp { min, center, max }
    } else if name.eq_ignore_ascii_case("round") {
      parse_round(input, context)?
    } else if name.eq_ignore_ascii_case("rem") {
      let (dividend, divisor) = parse_same_type_pair(input, context)?;
      RelativeMathFunction::Rem { dividend, divisor }
    } else if name.eq_ignore_ascii_case("mod") {
      let (dividend, divisor) = parse_same_type_pair(input, context)?;
      RelativeMathFunction::Mod { dividend, divisor }
    } else if name.eq_ignore_ascii_case("abs") {
      RelativeMathFunction::Abs {
        value: parse_sum(input, context)?,
      }
    } else if name.eq_ignore_ascii_case("sign") {
      RelativeMathFunction::Sign {
        value: parse_sum(input, context)?,
      }
    } else if name.eq_ignore_ascii_case("hypot") {
      RelativeMathFunction::Hypot {
        values: parse_same_type_list(input, context)?,
      }
    } else if name.eq_ignore_ascii_case("sin") {
      parse_trigonometric(input, context, |value| RelativeMathFunction::Sin { value })?
    } else if name.eq_ignore_ascii_case("cos") {
      parse_trigonometric(input, context, |value| RelativeMathFunction::Cos { value })?
    } else if name.eq_ignore_ascii_case("tan") {
      parse_trigonometric(input, context, |value| RelativeMathFunction::Tan { value })?
    } else if name.eq_ignore_ascii_case("asin") {
      parse_inverse_trigonometric(input, context, |value| RelativeMathFunction::Asin { value })?
    } else if name.eq_ignore_ascii_case("acos") {
      parse_inverse_trigonometric(input, context, |value| RelativeMathFunction::Acos { value })?
    } else if name.eq_ignore_ascii_case("atan") {
      parse_inverse_trigonometric(input, context, |value| RelativeMathFunction::Atan { value })?
    } else if name.eq_ignore_ascii_case("atan2") {
      let (y, x) = parse_same_type_pair(input, context)?;
      RelativeMathFunction::Atan2 { y, x }
    } else if name.eq_ignore_ascii_case("pow") {
      let base = parse_number_argument(input, context)?;
      input.expect_comma()?;
      let exponent = parse_number_argument(input, context)?;
      RelativeMathFunction::Pow { base, exponent }
    } else if name.eq_ignore_ascii_case("log") {
      let value = parse_number_argument(input, context)?;
      let base = if input.try_parse(|input| input.expect_comma()).is_ok() {
        Some(parse_number_argument(input, context)?)
      } else {
        None
      };
      RelativeMathFunction::Log { value, base }
    } else if name.eq_ignore_ascii_case("sqrt") {
      RelativeMathFunction::Sqrt {
        value: parse_number_argument(input, context)?,
      }
    } else if name.eq_ignore_ascii_case("exp") {
      RelativeMathFunction::Exp {
        value: parse_number_argument(input, context)?,
      }
    } else if name.eq_ignore_ascii_case("sibling-index") {
      RelativeMathFunction::SiblingIndex
    } else if name.eq_ignore_ascii_case("sibling-count") {
      RelativeMathFunction::SiblingCount
    } else {
      return Err(input.new_custom_error(ParserError::InvalidValue));
    };
    input.expect_exhausted()?;
    Ok(math)
  })?;
  Ok(RelativeColorExpression::Function(Box::new(math)))
}

fn parse_same_type_list<'i, 't>(
  input: &mut Parser<'i, 't>,
  context: RelativeColorParseContext,
) -> Result<Vec<RelativeColorExpression>, cssparser::ParseError<'i, ParserError<'i>>> {
  let values = input.parse_comma_separated(|input| parse_sum(input, context))?;
  require_same_types(input, values.iter())?;
  Ok(values)
}

fn parse_same_type_pair<'i, 't>(
  input: &mut Parser<'i, 't>,
  context: RelativeColorParseContext,
) -> Result<(RelativeColorExpression, RelativeColorExpression), cssparser::ParseError<'i, ParserError<'i>>> {
  let left = parse_sum(input, context)?;
  input.expect_comma()?;
  let right = parse_sum(input, context)?;
  require_same_types(input, [&left, &right])?;
  Ok((left, right))
}

fn require_same_types<'a, 'i, 't>(
  input: &mut Parser<'i, 't>,
  values: impl IntoIterator<Item = &'a RelativeColorExpression>,
) -> Result<(), cssparser::ParseError<'i, ParserError<'i>>> {
  let mut values = values.into_iter();
  let Some(first) = values.next() else {
    return Err(input.new_custom_error(ParserError::InvalidValue));
  };
  let Some(expected) = first.numeric_type() else {
    return Err(input.new_custom_error(ParserError::InvalidValue));
  };
  if values.any(|value| value.numeric_type() != Some(expected)) {
    return Err(input.new_custom_error(ParserError::InvalidValue));
  }
  Ok(())
}

fn parse_round<'i, 't>(
  input: &mut Parser<'i, 't>,
  context: RelativeColorParseContext,
) -> Result<RelativeMathFunction, cssparser::ParseError<'i, ParserError<'i>>> {
  let strategy = input
    .try_parse(|input| -> Result<RoundingStrategy, cssparser::ParseError<'i, ParserError<'i>>> {
      let identifier = input.expect_ident_cloned()?;
      let strategy = if identifier.eq_ignore_ascii_case("nearest") {
        RoundingStrategy::Nearest
      } else if identifier.eq_ignore_ascii_case("up") {
        RoundingStrategy::Up
      } else if identifier.eq_ignore_ascii_case("down") {
        RoundingStrategy::Down
      } else if identifier.eq_ignore_ascii_case("to-zero") {
        RoundingStrategy::ToZero
      } else {
        return Err(input.new_custom_error(ParserError::InvalidValue));
      };
      input.expect_comma()?;
      Ok(strategy)
    })
    .unwrap_or(RoundingStrategy::Nearest);
  let value = parse_sum(input, context)?;
  let step = if input.try_parse(|input| input.expect_comma()).is_ok() {
    let step = parse_sum(input, context)?;
    let Some(value_type) = value.numeric_type() else {
      return Err(input.new_custom_error(ParserError::InvalidValue));
    };
    if step.numeric_type() != Some(value_type) {
      return Err(input.new_custom_error(ParserError::InvalidValue));
    }
    Some(step)
  } else {
    None
  };
  Ok(RelativeMathFunction::Round { strategy, value, step })
}

fn parse_trigonometric<'i, 't, F>(
  input: &mut Parser<'i, 't>,
  context: RelativeColorParseContext,
  constructor: F,
) -> Result<RelativeMathFunction, cssparser::ParseError<'i, ParserError<'i>>>
where
  F: FnOnce(RelativeColorExpression) -> RelativeMathFunction,
{
  let value = parse_sum(input, context)?;
  if !matches!(value.numeric_type(), Some(NumericType::Number | NumericType::Angle)) {
    return Err(input.new_custom_error(ParserError::InvalidValue));
  }
  Ok(constructor(value))
}

fn parse_inverse_trigonometric<'i, 't, F>(
  input: &mut Parser<'i, 't>,
  context: RelativeColorParseContext,
  constructor: F,
) -> Result<RelativeMathFunction, cssparser::ParseError<'i, ParserError<'i>>>
where
  F: FnOnce(RelativeColorExpression) -> RelativeMathFunction,
{
  let value = parse_number_argument(input, context)?;
  Ok(constructor(value))
}

fn parse_number_argument<'i, 't>(
  input: &mut Parser<'i, 't>,
  context: RelativeColorParseContext,
) -> Result<RelativeColorExpression, cssparser::ParseError<'i, ParserError<'i>>> {
  let value = parse_sum(input, context)?;
  if value.numeric_type() != Some(NumericType::Number) {
    return Err(input.new_custom_error(ParserError::InvalidValue));
  }
  Ok(value)
}

impl RelativeColorExpression {
  fn numeric_type(&self) -> Option<NumericType> {
    match self {
      Self::Percentage(_) => Some(NumericType::Percentage),
      Self::Angle(_) => Some(NumericType::Angle),
      Self::Add(left, right) | Self::Subtract(left, right) => {
        let left = left.numeric_type()?;
        (right.numeric_type()? == left).then_some(left)
      }
      Self::Multiply(left, right) => multiply_type(left.numeric_type()?, right.numeric_type()?),
      Self::Divide(left, right) => divide_type(left.numeric_type()?, right.numeric_type()?),
      Self::Group(value) => value.numeric_type(),
      Self::Function(function) => function.numeric_type(),
      Self::Number(_) | Self::Channel(_) | Self::Constant(_) => Some(NumericType::Number),
    }
  }
}

impl RelativeMathFunction {
  fn numeric_type(&self) -> Option<NumericType> {
    match self {
      Self::Calc { value }
      | Self::Abs { value }
      | Self::Sqrt { value }
      | Self::Exp { value } => value.numeric_type(),
      Self::Min { values } | Self::Max { values } | Self::Hypot { values } => same_expression_type(values),
      Self::Clamp { min, center, max } => same_expression_type([min, center, max]),
      Self::Round { value, step, .. } => {
        let value_type = value.numeric_type()?;
        if let Some(step) = step {
          return (step.numeric_type()? == value_type).then_some(value_type);
        }
        Some(value_type)
      }
      Self::Rem { dividend, divisor } | Self::Mod { dividend, divisor } => {
        same_expression_type([dividend, divisor])
      }
      Self::Asin { value } | Self::Acos { value } | Self::Atan { value } => {
        (value.numeric_type()? == NumericType::Number).then_some(NumericType::Angle)
      }
      Self::Atan2 { y, x } => same_expression_type([y, x]).map(|_| NumericType::Angle),
      Self::Sign { value } => value.numeric_type().map(|_| NumericType::Number),
      Self::Sin { value } | Self::Cos { value } | Self::Tan { value } => matches!(
        value.numeric_type()?,
        NumericType::Number | NumericType::Angle
      )
      .then_some(NumericType::Number),
      Self::Pow { base, exponent } => {
        all_number_expressions([base, exponent]).then_some(NumericType::Number)
      }
      Self::Log { value, base } => {
        let value_is_number = value.numeric_type() == Some(NumericType::Number);
        let base_is_number = base
          .as_ref()
          .is_none_or(|base| base.numeric_type() == Some(NumericType::Number));
        (value_is_number && base_is_number).then_some(NumericType::Number)
      }
      Self::SiblingIndex | Self::SiblingCount => Some(NumericType::Number),
    }
  }
}

fn same_expression_type<'a>(values: impl IntoIterator<Item = &'a RelativeColorExpression>) -> Option<NumericType> {
  let mut values = values.into_iter();
  let first = values.next()?.numeric_type()?;
  values.all(|value| value.numeric_type() == Some(first)).then_some(first)
}

fn all_number_expressions<'a>(values: impl IntoIterator<Item = &'a RelativeColorExpression>) -> bool {
  values
    .into_iter()
    .all(|value| value.numeric_type() == Some(NumericType::Number))
}

impl ToCss for RelativeColorOrigin {
  fn to_css<W>(&self, dest: &mut Printer<W>) -> Result<(), PrinterError>
  where
    W: std::fmt::Write,
  {
    match self {
      Self::Keyword { name, .. } => dest.write_str(name),
      Self::Value(color) => write_cssom_origin_color(color, dest),
      Self::Contrast(color) => {
        dest.write_str("contrast-color(")?;
        color.to_css(dest)?;
        dest.write_char(')')
      }
      Self::LightDark(light, dark) => {
        dest.write_str("light-dark(")?;
        light.to_css(dest)?;
        dest.delim(',', false)?;
        dark.to_css(dest)?;
        dest.write_char(')')
      }
      Self::ColorMix(mix) => mix.to_css(dest),
    }
  }
}

impl ToCss for RelativeColorMix {
  fn to_css<W>(&self, dest: &mut Printer<W>) -> Result<(), PrinterError>
  where
    W: std::fmt::Write,
  {
    dest.write_str("color-mix(")?;
    if self.color_space != ColorSpaceName::OKLAB
      || self.hue_interpolation_method != HueInterpolationMethod::Shorter
    {
      dest.write_str("in ")?;
      write_cssom_color_space(self.color_space, dest)?;
      if self.hue_interpolation_method != HueInterpolationMethod::Shorter {
        dest.write_char(' ')?;
        self.hue_interpolation_method.to_css(dest)?;
        dest.write_str(" hue")?;
      }
      dest.delim(',', false)?;
    }
    self.first.to_css(dest)?;
    if let Some(percentage) = self.first_percentage {
      dest.write_char(' ')?;
      write_cssom_percentage(percentage, dest)?;
    }
    dest.delim(',', false)?;
    self.second.to_css(dest)?;
    if let Some(percentage) = self.second_percentage {
      dest.write_char(' ')?;
      write_cssom_percentage(percentage, dest)?;
    }
    dest.write_char(')')
  }
}

fn write_cssom_origin_color<W>(color: &CssColor, dest: &mut Printer<W>) -> Result<(), PrinterError>
where
  W: std::fmt::Write,
{
  match color {
    CssColor::CurrentColor => dest.write_str("currentcolor"),
    CssColor::RGBA(color) => write_cssom_rgb(
      f32::from(color.red),
      f32::from(color.green),
      f32::from(color.blue),
      color.alpha_f32(),
      dest,
    ),
    CssColor::Float(color) => {
      let color = RGB::from(**color).resolve_missing();
      write_cssom_rgb(color.r, color.g, color.b, color.alpha, dest)
    }
    CssColor::LAB(color) => write_cssom_lab_color(color, dest),
    CssColor::Predefined(color) => write_cssom_predefined_color(color, dest),
    CssColor::LightDark(light, dark) => {
      dest.write_str("light-dark(")?;
      write_cssom_origin_color(light, dest)?;
      dest.delim(',', false)?;
      write_cssom_origin_color(dark, dest)?;
      dest.write_char(')')
    }
    CssColor::ContrastColor(color) => {
      dest.write_str("contrast-color(")?;
      write_cssom_origin_color(color, dest)?;
      dest.write_char(')')
    }
    CssColor::ColorMix(color) => color.to_css(dest),
    CssColor::Relative(color) => color.to_css(dest),
    CssColor::System(color) => color.to_css(dest),
  }
}

fn write_cssom_rgb<W>(r: f32, g: f32, b: f32, alpha: f32, dest: &mut Printer<W>) -> Result<(), PrinterError>
where
  W: std::fmt::Write,
{
  let alpha = if alpha.is_nan() { 0.0 } else { alpha.clamp(0.0, 1.0) };
  dest.write_str(if alpha == 1.0 { "rgb(" } else { "rgba(" })?;
  for (index, channel) in [r, g, b].into_iter().enumerate() {
    if index > 0 {
      dest.delim(',', false)?;
    }
    write_cssom_number(if channel.is_nan() {
      0.0
    } else {
      channel.round().clamp(0.0, 255.0)
    }, dest)?;
  }
  if alpha != 1.0 {
    dest.delim(',', false)?;
    write_cssom_number(alpha, dest)?;
  }
  dest.write_char(')')
}

fn write_cssom_lab_color<W>(color: &LABColor, dest: &mut Printer<W>) -> Result<(), PrinterError>
where
  W: std::fmt::Write,
{
  match color {
    LABColor::LAB(color) => write_cssom_components("lab", color.l.clamp(0.0, 100.0), color.a, color.b, color.alpha, dest),
    LABColor::OKLAB(color) => {
      write_cssom_components("oklab", color.l.clamp(0.0, 1.0), color.a, color.b, color.alpha, dest)
    }
    LABColor::LCH(color) => {
      write_cssom_components(
        "lch",
        color.l.clamp(0.0, 100.0),
        clamp_chroma(color.c),
        normalize_hue(color.h),
        color.alpha,
        dest,
      )
    }
    LABColor::OKLCH(color) => {
      write_cssom_components(
        "oklch",
        color.l.clamp(0.0, 1.0),
        clamp_chroma(color.c),
        normalize_hue(color.h),
        color.alpha,
        dest,
      )
    }
  }
}

fn write_cssom_predefined_color<W>(color: &PredefinedColor, dest: &mut Printer<W>) -> Result<(), PrinterError>
where
  W: std::fmt::Write,
{
  let (name, a, b, c, alpha) = match color {
    PredefinedColor::SRGB(color) => ("srgb", color.r, color.g, color.b, color.alpha),
    PredefinedColor::SRGBLinear(color) => ("srgb-linear", color.r, color.g, color.b, color.alpha),
    PredefinedColor::DisplayP3(color) => ("display-p3", color.r, color.g, color.b, color.alpha),
    PredefinedColor::DisplayP3Linear(color) => ("display-p3-linear", color.r, color.g, color.b, color.alpha),
    PredefinedColor::A98(color) => ("a98-rgb", color.r, color.g, color.b, color.alpha),
    PredefinedColor::ProPhoto(color) => ("prophoto-rgb", color.r, color.g, color.b, color.alpha),
    PredefinedColor::Rec2020(color) => ("rec2020", color.r, color.g, color.b, color.alpha),
    PredefinedColor::XYZd50(color) => ("xyz-d50", color.x, color.y, color.z, color.alpha),
    PredefinedColor::XYZd65(color) => ("xyz-d65", color.x, color.y, color.z, color.alpha),
  };
  dest.write_str("color(")?;
  dest.write_str(name)?;
  for component in [a, b, c] {
    dest.write_char(' ')?;
    write_cssom_component(component, dest)?;
  }
  if alpha.is_nan() || alpha != 1.0 {
    dest.delim('/', true)?;
    write_cssom_component(alpha.clamp(0.0, 1.0), dest)?;
  }
  dest.write_char(')')
}

fn write_cssom_components<W>(
  name: &str,
  a: f32,
  b: f32,
  c: f32,
  alpha: f32,
  dest: &mut Printer<W>,
) -> Result<(), PrinterError>
where
  W: std::fmt::Write,
{
  dest.write_str(name)?;
  dest.write_char('(')?;
  for (index, component) in [a, b, c].into_iter().enumerate() {
    if index > 0 {
      dest.write_char(' ')?;
    }
    write_cssom_component(component, dest)?;
  }
  if alpha.is_nan() || alpha != 1.0 {
    dest.delim('/', true)?;
    write_cssom_component(alpha.clamp(0.0, 1.0), dest)?;
  }
  dest.write_char(')')
}

fn write_cssom_color_space<W>(space: ColorSpaceName, dest: &mut Printer<W>) -> Result<(), PrinterError>
where
  W: std::fmt::Write,
{
  if matches!(space, ColorSpaceName::XYZ | ColorSpaceName::XYZd65) {
    return dest.write_str("xyz-d65");
  }
  space.to_css(dest)
}

fn write_cssom_percentage<W>(value: f32, dest: &mut Printer<W>) -> Result<(), PrinterError>
where
  W: std::fmt::Write,
{
  write_cssom_number(value * 100.0, dest)?;
  dest.write_char('%')
}

fn write_cssom_component<W>(value: f32, dest: &mut Printer<W>) -> Result<(), PrinterError>
where
  W: std::fmt::Write,
{
  if value.is_nan() {
    return dest.write_str("none");
  }
  write_cssom_number(value, dest)
}

fn write_cssom_number<W>(value: f32, dest: &mut Printer<W>) -> Result<(), PrinterError>
where
  W: std::fmt::Write,
{
  if value.is_nan() {
    return dest.write_str("NaN");
  }
  if value == f32::INFINITY {
    return dest.write_str("infinity");
  }
  if value == f32::NEG_INFINITY {
    return dest.write_str("-infinity");
  }

  // Blink's CSSOM number serializer limits the observable decimal tail. Keep
  // the calculation in f64 so rounding a finite f32 cannot overflow first.
  let value = ((f64::from(value) * 1_000_000.0).round() / 1_000_000.0) as f32;
  let value = if value == 0.0 { 0.0 } else { value };
  dest.write_str(&value.to_string())
}

fn clamp_chroma(value: f32) -> f32 {
  if value.is_nan() {
    return value;
  }
  value.max(0.0)
}

fn normalize_hue(value: f32) -> f32 {
  if value.is_nan() {
    return value;
  }
  value.rem_euclid(360.0)
}

impl ToCss for RelativeColor {
  fn to_css<W>(&self, dest: &mut Printer<W>) -> Result<(), PrinterError>
  where
    W: std::fmt::Write,
  {
    dest.write_str(self.function.name())?;
    dest.write_str("(from ")?;
    self.origin.to_css(dest)?;
    dest.write_char(' ')?;
    if let RelativeColorFunction::Color(space) = self.function {
      dest.write_str(space.name())?;
      dest.write_char(' ')?;
    }
    for (index, component) in self.components.iter().enumerate() {
      if index > 0 {
        dest.write_char(' ')?;
      }
      component.to_css(dest)?;
    }
    if let Some(alpha) = &self.alpha {
      dest.delim('/', true)?;
      alpha.to_css(dest)?;
    }
    dest.write_char(')')
  }
}

impl ToCss for RelativeColorComponent {
  fn to_css<W>(&self, dest: &mut Printer<W>) -> Result<(), PrinterError>
  where
    W: std::fmt::Write,
  {
    match self {
      Self::None => dest.write_str("none"),
      Self::Value(value) if value.requires_cssom_calc_wrapper() => write_calc_function(value, dest),
      Self::Value(value) => value.to_css(dest),
    }
  }
}

impl ToCss for RelativeColorExpression {
  fn to_css<W>(&self, dest: &mut Printer<W>) -> Result<(), PrinterError>
  where
    W: std::fmt::Write,
  {
    self.to_css_with_precedence(dest, 0)
  }
}

impl RelativeColorExpression {
  fn requires_cssom_calc_wrapper(&self) -> bool {
    matches!(
      self,
      Self::Function(function)
        if matches!(
          &**function,
          RelativeMathFunction::Sin { .. }
            | RelativeMathFunction::Cos { .. }
            | RelativeMathFunction::Tan { .. }
            | RelativeMathFunction::Asin { .. }
            | RelativeMathFunction::Acos { .. }
            | RelativeMathFunction::Atan { .. }
            | RelativeMathFunction::Atan2 { .. }
            | RelativeMathFunction::Pow { .. }
            | RelativeMathFunction::Log { .. }
            | RelativeMathFunction::Sqrt { .. }
            | RelativeMathFunction::Exp { .. }
        )
    )
  }

  fn to_css_with_precedence<W>(&self, dest: &mut Printer<W>, parent_precedence: u8) -> Result<(), PrinterError>
  where
    W: std::fmt::Write,
  {
    let precedence = match self {
      Self::Add(..) | Self::Subtract(..) => 1,
      Self::Multiply(..) | Self::Divide(..) => 2,
      _ => 3,
    };
    let parenthesize = precedence < parent_precedence;
    if parenthesize {
      dest.write_char('(')?;
    }
    match self {
      Self::Number(value) => write_cssom_number(*value, dest)?,
      Self::Percentage(value) => write_cssom_percentage(*value, dest)?,
      Self::Angle(value) => value.to_css(dest)?,
      Self::Channel(channel) => dest.write_str(channel.name())?,
      Self::Constant(constant) => dest.write_str(constant.name())?,
      Self::Group(value) => {
        dest.write_char('(')?;
        value.to_css_with_precedence(dest, 0)?;
        dest.write_char(')')?;
      }
      Self::Add(left, right) => {
        left.to_css_with_precedence(dest, precedence)?;
        dest.write_str(" + ")?;
        right.to_css_with_precedence(dest, precedence)?;
      }
      Self::Subtract(left, right) => {
        left.to_css_with_precedence(dest, precedence)?;
        dest.write_str(" - ")?;
        right.to_css_with_precedence(dest, precedence + 1)?;
      }
      Self::Multiply(left, right) => {
        left.to_css_with_precedence(dest, precedence)?;
        dest.write_str(" * ")?;
        right.to_css_with_precedence(dest, precedence)?;
      }
      Self::Divide(left, right) => {
        left.to_css_with_precedence(dest, precedence)?;
        dest.write_str(" / ")?;
        right.to_css_with_precedence(dest, precedence + 1)?;
      }
      Self::Function(function) => function.to_css(dest)?,
    }
    if parenthesize {
      dest.write_char(')')?;
    }
    Ok(())
  }
}

impl ToCss for RelativeMathFunction {
  fn to_css<W>(&self, dest: &mut Printer<W>) -> Result<(), PrinterError>
  where
    W: std::fmt::Write,
  {
    match self {
      Self::Calc { value } => write_calc_function(value, dest),
      Self::Min { values } => write_list_function("min", values, dest),
      Self::Max { values } => write_list_function("max", values, dest),
      Self::Clamp { min, center, max } => {
        dest.write_str("clamp(")?;
        min.to_css(dest)?;
        dest.delim(',', false)?;
        center.to_css(dest)?;
        dest.delim(',', false)?;
        max.to_css(dest)?;
        dest.write_char(')')
      }
      Self::Round { strategy, value, step } => {
        dest.write_str("round(")?;
        if *strategy != RoundingStrategy::Nearest {
          strategy.to_css(dest)?;
          dest.delim(',', false)?;
        }
        value.to_css(dest)?;
        if let Some(step) = step {
          dest.delim(',', false)?;
          step.to_css(dest)?;
        }
        dest.write_char(')')
      }
      Self::Rem { dividend, divisor } => write_binary_function("rem", dividend, divisor, dest),
      Self::Mod { dividend, divisor } => write_binary_function("mod", dividend, divisor, dest),
      Self::Abs { value } => write_unary_function("abs", value, dest),
      Self::Sign { value } => write_unary_function("sign", value, dest),
      Self::Hypot { values } => write_list_function("hypot", values, dest),
      Self::Sin { value } => write_unary_function("sin", value, dest),
      Self::Cos { value } => write_unary_function("cos", value, dest),
      Self::Tan { value } => write_unary_function("tan", value, dest),
      Self::Asin { value } => write_unary_function("asin", value, dest),
      Self::Acos { value } => write_unary_function("acos", value, dest),
      Self::Atan { value } => write_unary_function("atan", value, dest),
      Self::Atan2 { y, x } => write_binary_function("atan2", y, x, dest),
      Self::Pow { base, exponent } => write_binary_function("pow", base, exponent, dest),
      Self::Log { value, base } => {
        dest.write_str("log(")?;
        value.to_css(dest)?;
        if let Some(base) = base {
          dest.delim(',', false)?;
          base.to_css(dest)?;
        }
        dest.write_char(')')
      }
      Self::Sqrt { value } => write_unary_function("sqrt", value, dest),
      Self::Exp { value } => write_unary_function("exp", value, dest),
      Self::SiblingIndex => dest.write_str("sibling-index()"),
      Self::SiblingCount => dest.write_str("sibling-count()"),
    }
  }
}

fn write_calc_function<W>(
  value: &RelativeColorExpression,
  dest: &mut Printer<W>,
) -> Result<(), PrinterError>
where
  W: std::fmt::Write,
{
  dest.write_str("calc(")?;
  match value {
    RelativeColorExpression::Add(left, right) => {
      write_calc_sum_operand(left, dest)?;
      dest.write_str(" + ")?;
      write_calc_sum_operand(right, dest)?;
    }
    RelativeColorExpression::Subtract(left, right) => {
      write_calc_sum_operand(left, dest)?;
      dest.write_str(" - ")?;
      write_calc_sum_operand(right, dest)?;
    }
    _ => value.to_css(dest)?,
  }
  dest.write_char(')')
}

fn write_calc_sum_operand<W>(
  value: &RelativeColorExpression,
  dest: &mut Printer<W>,
) -> Result<(), PrinterError>
where
  W: std::fmt::Write,
{
  if matches!(
    value,
    RelativeColorExpression::Multiply(..) | RelativeColorExpression::Divide(..)
  ) {
    dest.write_char('(')?;
    value.to_css(dest)?;
    return dest.write_char(')');
  }
  value.to_css(dest)
}

fn write_unary_function<W>(
  name: &str,
  value: &RelativeColorExpression,
  dest: &mut Printer<W>,
) -> Result<(), PrinterError>
where
  W: std::fmt::Write,
{
  dest.write_str(name)?;
  dest.write_char('(')?;
  value.to_css(dest)?;
  dest.write_char(')')
}

fn write_binary_function<W>(
  name: &str,
  left: &RelativeColorExpression,
  right: &RelativeColorExpression,
  dest: &mut Printer<W>,
) -> Result<(), PrinterError>
where
  W: std::fmt::Write,
{
  dest.write_str(name)?;
  dest.write_char('(')?;
  left.to_css(dest)?;
  dest.delim(',', false)?;
  right.to_css(dest)?;
  dest.write_char(')')
}

fn write_list_function<W>(
  name: &str,
  values: &[RelativeColorExpression],
  dest: &mut Printer<W>,
) -> Result<(), PrinterError>
where
  W: std::fmt::Write,
{
  dest.write_str(name)?;
  dest.write_char('(')?;
  for (index, value) in values.iter().enumerate() {
    if index > 0 {
      dest.delim(',', false)?;
    }
    value.to_css(dest)?;
  }
  dest.write_char(')')
}
