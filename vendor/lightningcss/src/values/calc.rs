//! Mathematical calculation functions and expressions.

use crate::compat::Feature;
use crate::error::{ParserError, PrinterError};
use crate::macros::enum_property;
use crate::printer::Printer;
use crate::targets::{should_compile, Browsers};
use crate::traits::private::AddInternal;
use crate::traits::{IsCompatible, Parse, Sign, ToCss, TryMap, TryOp, TrySign};
#[cfg(feature = "visitor")]
use crate::visitor::Visit;
use cssparser::*;
use std::cmp::Ordering;

use super::angle::Angle;
use super::length::Length;
use super::number::CSSNumber;
use super::percentage::Percentage;
use super::time::Time;

/// A CSS [math function](https://www.w3.org/TR/css-values-4/#math-function).
///
/// Math functions may be used in most properties and values that accept numeric
/// values, including lengths, percentages, angles, times, etc.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "visitor", derive(Visit))]
#[cfg_attr(
  feature = "serde",
  derive(serde::Serialize, serde::Deserialize),
  serde(tag = "type", content = "value", rename_all = "kebab-case")
)]
#[cfg_attr(feature = "jsonschema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "into_owned", derive(static_self::IntoOwned))]
pub enum MathFunction<V> {
  /// The [`calc()`](https://www.w3.org/TR/css-values-4/#calc-func) function.
  Calc(Calc<V>),
  /// The [`min()`](https://www.w3.org/TR/css-values-4/#funcdef-min) function.
  Min(Vec<Calc<V>>),
  /// The [`max()`](https://www.w3.org/TR/css-values-4/#funcdef-max) function.
  Max(Vec<Calc<V>>),
  /// The [`clamp()`](https://www.w3.org/TR/css-values-4/#funcdef-clamp) function.
  Clamp(Calc<V>, Calc<V>, Calc<V>),
  /// The [`round()`](https://www.w3.org/TR/css-values-4/#funcdef-round) function.
  Round(RoundingStrategy, Calc<V>, Calc<V>),
  /// The [`rem()`](https://www.w3.org/TR/css-values-4/#funcdef-rem) function.
  Rem(Calc<V>, Calc<V>),
  /// The [`mod()`](https://www.w3.org/TR/css-values-4/#funcdef-mod) function.
  Mod(Calc<V>, Calc<V>),
  /// The [`abs()`](https://drafts.csswg.org/css-values-4/#funcdef-abs) function.
  Abs(Calc<V>),
  /// The [`sign()`](https://drafts.csswg.org/css-values-4/#funcdef-sign) function.
  Sign(Calc<V>),
  /// The [`hypot()`](https://drafts.csswg.org/css-values-4/#funcdef-hypot) function.
  Hypot(Vec<Calc<V>>),
}

impl<V: IsCompatible> IsCompatible for MathFunction<V> {
  fn is_compatible(&self, browsers: Browsers) -> bool {
    match self {
      MathFunction::Calc(v) => Feature::CalcFunction.is_compatible(browsers) && v.is_compatible(browsers),
      MathFunction::Min(v) => {
        Feature::MinFunction.is_compatible(browsers) && v.iter().all(|v| v.is_compatible(browsers))
      }
      MathFunction::Max(v) => {
        Feature::MaxFunction.is_compatible(browsers) && v.iter().all(|v| v.is_compatible(browsers))
      }
      MathFunction::Clamp(a, b, c) => {
        Feature::ClampFunction.is_compatible(browsers)
          && a.is_compatible(browsers)
          && b.is_compatible(browsers)
          && c.is_compatible(browsers)
      }
      MathFunction::Round(_, a, b) => {
        Feature::RoundFunction.is_compatible(browsers) && a.is_compatible(browsers) && b.is_compatible(browsers)
      }
      MathFunction::Rem(a, b) => {
        Feature::RemFunction.is_compatible(browsers) && a.is_compatible(browsers) && b.is_compatible(browsers)
      }
      MathFunction::Mod(a, b) => {
        Feature::ModFunction.is_compatible(browsers) && a.is_compatible(browsers) && b.is_compatible(browsers)
      }
      MathFunction::Abs(v) => Feature::AbsFunction.is_compatible(browsers) && v.is_compatible(browsers),
      MathFunction::Sign(v) => Feature::SignFunction.is_compatible(browsers) && v.is_compatible(browsers),
      MathFunction::Hypot(v) => {
        Feature::HypotFunction.is_compatible(browsers) && v.iter().all(|v| v.is_compatible(browsers))
      }
    }
  }
}

enum_property! {
  /// A [rounding strategy](https://www.w3.org/TR/css-values-4/#typedef-rounding-strategy),
  /// as used in the `round()` function.
  pub enum RoundingStrategy {
    /// Round to the nearest integer.
    Nearest,
    /// Round up (ceil).
    Up,
    /// Round down (floor).
    Down,
    /// Round toward zero (truncate).
    ToZero,
  }
}

impl Default for RoundingStrategy {
  fn default() -> Self {
    RoundingStrategy::Nearest
  }
}

fn round(value: f32, to: f32, strategy: RoundingStrategy) -> f32 {
  let v = value / to;
  match strategy {
    RoundingStrategy::Down => v.floor() * to,
    RoundingStrategy::Up => v.ceil() * to,
    RoundingStrategy::Nearest => v.round() * to,
    RoundingStrategy::ToZero => v.trunc() * to,
  }
}

fn modulo(a: f32, b: f32) -> f32 {
  ((a % b) + b) % b
}

impl<V: ToCss + std::ops::Mul<f32, Output = V> + TrySign + Clone + std::fmt::Debug> ToCss for MathFunction<V> {
  fn to_css<W>(&self, dest: &mut Printer<W>) -> Result<(), PrinterError>
  where
    W: std::fmt::Write,
  {
    match self {
      MathFunction::Calc(calc) => {
        dest.write_str("calc(")?;
        calc.to_css(dest)?;
        dest.write_char(')')
      }
      MathFunction::Min(args) => {
        dest.write_str("min(")?;
        let mut first = true;
        for arg in args {
          if first {
            first = false;
          } else {
            dest.delim(',', false)?;
          }
          arg.to_css(dest)?;
        }
        dest.write_char(')')
      }
      MathFunction::Max(args) => {
        dest.write_str("max(")?;
        let mut first = true;
        for arg in args {
          if first {
            first = false;
          } else {
            dest.delim(',', false)?;
          }
          arg.to_css(dest)?;
        }
        dest.write_char(')')
      }
      MathFunction::Clamp(a, b, c) => {
        // If clamp() is unsupported by targets, output min()/max()
        if should_compile!(dest.targets.current, ClampFunction) {
          dest.write_str("max(")?;
          a.to_css(dest)?;
          dest.delim(',', false)?;
          dest.write_str("min(")?;
          b.to_css(dest)?;
          dest.delim(',', false)?;
          c.to_css(dest)?;
          dest.write_str("))")?;
          return Ok(());
        }

        dest.write_str("clamp(")?;
        a.to_css(dest)?;
        dest.delim(',', false)?;
        b.to_css(dest)?;
        dest.delim(',', false)?;
        c.to_css(dest)?;
        dest.write_char(')')
      }
      MathFunction::Round(strategy, a, b) => {
        dest.write_str("round(")?;
        if *strategy != RoundingStrategy::default() {
          strategy.to_css(dest)?;
          dest.delim(',', false)?;
        }
        a.to_css(dest)?;
        dest.delim(',', false)?;
        b.to_css(dest)?;
        dest.write_char(')')
      }
      MathFunction::Rem(a, b) => {
        dest.write_str("rem(")?;
        a.to_css(dest)?;
        dest.delim(',', false)?;
        b.to_css(dest)?;
        dest.write_char(')')
      }
      MathFunction::Mod(a, b) => {
        dest.write_str("mod(")?;
        a.to_css(dest)?;
        dest.delim(',', false)?;
        b.to_css(dest)?;
        dest.write_char(')')
      }
      MathFunction::Abs(v) => {
        dest.write_str("abs(")?;
        v.to_css(dest)?;
        dest.write_char(')')
      }
      MathFunction::Sign(v) => {
        dest.write_str("sign(")?;
        v.to_css(dest)?;
        dest.write_char(')')
      }
      MathFunction::Hypot(args) => {
        dest.write_str("hypot(")?;
        let mut first = true;
        for arg in args {
          if first {
            first = false;
          } else {
            dest.delim(',', false)?;
          }
          arg.to_css(dest)?;
        }
        dest.write_char(')')
      }
    }
  }
}

/// A mathematical expression used within the [`calc()`](https://www.w3.org/TR/css-values-4/#calc-func) function.
///
/// This type supports generic value types. Values such as [Length](super::length::Length), [Percentage](super::percentage::Percentage),
/// [Time](super::time::Time), and [Angle](super::angle::Angle) support `calc()` expressions.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "visitor", derive(Visit))]
#[cfg_attr(
  feature = "serde",
  derive(serde::Serialize, serde::Deserialize),
  serde(tag = "type", content = "value", rename_all = "kebab-case")
)]
#[cfg_attr(feature = "jsonschema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "into_owned", derive(static_self::IntoOwned))]
pub enum Calc<V> {
  /// A literal value.
  Value(Box<V>),
  /// A literal number.
  Number(CSSNumber),
  /// A sum of two calc expressions.
  #[cfg_attr(feature = "visitor", skip_type)]
  Sum(Box<Calc<V>>, Box<Calc<V>>),
  /// A product of a number and another calc expression.
  #[cfg_attr(feature = "visitor", skip_type)]
  Product(CSSNumber, Box<Calc<V>>),
  /// A product containing a number-valued expression that cannot be reduced
  /// until computed-value time.
  #[cfg_attr(feature = "visitor", skip_type)]
  ProductExpression(Box<Calc<V>>, Box<Calc<V>>),
  /// A quotient whose divisor is a number-valued expression that cannot be
  /// reduced until computed-value time.
  #[cfg_attr(feature = "visitor", skip_type)]
  QuotientExpression(Box<Calc<V>>, Box<Calc<V>>),
  /// A math function, such as `calc()`, `min()`, or `max()`.
  #[cfg_attr(feature = "visitor", skip_type)]
  Function(Box<MathFunction<V>>),
}

// Erase the callback type before recursive parsing so each value type produces one parser body,
// rather than one body per call-site closure.
type CalcIdentifierParser<'a, V> = dyn Fn(&str) -> Option<Calc<V>> + 'a;

// Most of the math-function dispatcher is independent of V, but making it part of Calc<V>'s
// parser creates a large copy for every value type. Keep Calc<V> as the public representation and
// store typed nodes in a small arena, while the shared dispatcher operates on node handles through
// this adapter.
type CalcParserNode = usize;

#[derive(Clone, Copy)]
enum CalcParserNodeKind {
  Value,
  Number(CSSNumber),
  Function,
  Other,
}

enum CalcParserFunction {
  Min(Vec<CalcParserNode>),
  Max(Vec<CalcParserNode>),
  Clamp(CalcParserNode, CalcParserNode, CalcParserNode),
  Round(RoundingStrategy, CalcParserNode, CalcParserNode),
  Rem(CalcParserNode, CalcParserNode),
  Mod(CalcParserNode, CalcParserNode),
  Abs(CalcParserNode),
  Sign(CalcParserNode),
  Hypot(Vec<CalcParserNode>),
}

#[derive(Clone, Copy)]
enum CalcParserBinaryOp {
  Round(RoundingStrategy),
  Rem,
  Mod,
  Hypot,
  HypotSum,
}

#[derive(Clone, Copy)]
enum CalcParserMapOp {
  Abs,
  Square,
  Sqrt,
}

trait CalcParserOps<'i> {
  fn parse_sum<'t>(
    &mut self,
    input: &mut Parser<'i, 't>,
    preserve_math_functions: bool,
  ) -> Result<CalcParserNode, ParseError<'i, ParserError<'i>>>;
  fn number(&mut self, value: CSSNumber) -> CalcParserNode;
  fn node_kind(&self, node: CalcParserNode) -> CalcParserNodeKind;
  fn function(&mut self, function: CalcParserFunction) -> CalcParserNode;
  fn compare_values(&self, left: CalcParserNode, right: CalcParserNode) -> Option<Ordering>;
  fn apply_binary(
    &mut self,
    left: CalcParserNode,
    right: CalcParserNode,
    op: CalcParserBinaryOp,
  ) -> Option<CalcParserNode>;
  fn apply_map(&mut self, node: CalcParserNode, op: CalcParserMapOp) -> Option<CalcParserNode>;
  fn value_sign(&self, node: CalcParserNode) -> Option<CSSNumber>;
  fn clone_node(&mut self, node: CalcParserNode) -> CalcParserNode;
  fn parse_identifier_as_angle(&self, identifier: &str) -> Option<Calc<Angle>>;
  fn parse_identifier_as_number(&self, identifier: &str) -> Option<Calc<CSSNumber>>;
  fn from_angle(&mut self, angle: Angle) -> Option<CalcParserNode>;
}

struct TypedCalcParserOps<'a, V> {
  nodes: Vec<Option<Calc<V>>>,
  parse_ident: &'a CalcIdentifierParser<'a, V>,
}

impl<'a, V> TypedCalcParserOps<'a, V> {
  fn new(parse_ident: &'a CalcIdentifierParser<'a, V>) -> Self {
    Self {
      nodes: Vec::new(),
      parse_ident,
    }
  }

  fn insert(&mut self, value: Calc<V>) -> CalcParserNode {
    let node = self.nodes.len();
    self.nodes.push(Some(value));
    node
  }

  fn get(&self, node: CalcParserNode) -> &Calc<V> {
    self.nodes[node].as_ref().unwrap()
  }

  fn take(&mut self, node: CalcParserNode) -> Calc<V> {
    self.nodes[node].take().unwrap()
  }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CalcResolvedType {
  Number,
  Dimension,
}

impl<V> Calc<V> {
  /// Returns whether this calculation resolves to the dimension represented by
  /// `V`, rather than to a plain `<number>`.
  ///
  /// `Calc<V>` can contain numbers because CSS products use numeric
  /// coefficients. That does not make a standalone number, or a function such
  /// as `sign()`, valid where a dimension is required.
  pub(crate) fn resolves_to_dimension(&self) -> bool {
    self.resolved_type() == Some(CalcResolvedType::Dimension)
  }

  fn resolved_type(&self) -> Option<CalcResolvedType> {
    match self {
      Calc::Value(_) => Some(CalcResolvedType::Dimension),
      Calc::Number(_) => Some(CalcResolvedType::Number),
      Calc::Sum(a, b) => (a.resolved_type() == b.resolved_type())
        .then(|| a.resolved_type())
        .flatten(),
      Calc::Product(_, value) => value.resolved_type(),
      Calc::ProductExpression(left, right) => match (left.resolved_type()?, right.resolved_type()?) {
        (CalcResolvedType::Number, CalcResolvedType::Number) => Some(CalcResolvedType::Number),
        (CalcResolvedType::Number, CalcResolvedType::Dimension)
        | (CalcResolvedType::Dimension, CalcResolvedType::Number) => Some(CalcResolvedType::Dimension),
        (CalcResolvedType::Dimension, CalcResolvedType::Dimension) => None,
      },
      Calc::QuotientExpression(value, divisor) => match (value.resolved_type()?, divisor.resolved_type()?) {
        (CalcResolvedType::Number, CalcResolvedType::Number)
        | (CalcResolvedType::Dimension, CalcResolvedType::Dimension) => Some(CalcResolvedType::Number),
        (CalcResolvedType::Dimension, CalcResolvedType::Number) => Some(CalcResolvedType::Dimension),
        (CalcResolvedType::Number, CalcResolvedType::Dimension) => None,
      },
      Calc::Function(function) => function.resolved_type(),
    }
  }

  /// Returns whether this calculation resolves to a plain `<number>` rather
  /// than to the dimension represented by `V`.
  ///
  /// This is useful for number-valued CSS properties that accept functions
  /// such as `sign()` whose argument may have a different dimension. The
  /// dimensional argument still needs to be retained until computed-value
  /// time, even though the result of the outer calculation is a number.
  pub fn resolves_to_number(&self) -> bool {
    self.resolved_type() == Some(CalcResolvedType::Number)
  }

  /// Returns whether this calculation still contains a `sign()` function that
  /// depends on computed context.
  pub fn contains_unresolved_sign(&self) -> bool {
    match self {
      Calc::Value(_) | Calc::Number(_) => false,
      Calc::Sum(left, right)
      | Calc::ProductExpression(left, right)
      | Calc::QuotientExpression(left, right) => {
        left.contains_unresolved_sign() || right.contains_unresolved_sign()
      }
      Calc::Product(_, value) => value.contains_unresolved_sign(),
      Calc::Function(function) => function.contains_unresolved_sign(),
    }
  }
}

impl<V> MathFunction<V> {
  fn resolved_type(&self) -> Option<CalcResolvedType> {
    match self {
      MathFunction::Calc(value) | MathFunction::Abs(value) => value.resolved_type(),
      MathFunction::Min(values) | MathFunction::Max(values) | MathFunction::Hypot(values) => {
        common_resolved_type(values)
      }
      MathFunction::Clamp(min, center, max) => common_resolved_type([min, center, max]),
      MathFunction::Round(_, value, step) | MathFunction::Rem(value, step) | MathFunction::Mod(value, step) => {
        common_resolved_type([value, step])
      }
      MathFunction::Sign(_) => Some(CalcResolvedType::Number),
    }
  }

  fn contains_unresolved_sign(&self) -> bool {
    match self {
      MathFunction::Sign(_) => true,
      MathFunction::Calc(value) | MathFunction::Abs(value) => value.contains_unresolved_sign(),
      MathFunction::Min(values) | MathFunction::Max(values) | MathFunction::Hypot(values) => {
        values.iter().any(Calc::contains_unresolved_sign)
      }
      MathFunction::Clamp(min, center, max) => {
        min.contains_unresolved_sign()
          || center.contains_unresolved_sign()
          || max.contains_unresolved_sign()
      }
      MathFunction::Round(_, value, step)
      | MathFunction::Rem(value, step)
      | MathFunction::Mod(value, step) => {
        value.contains_unresolved_sign() || step.contains_unresolved_sign()
      }
    }
  }
}

fn common_resolved_type<'a, V>(values: impl IntoIterator<Item = &'a Calc<V>>) -> Option<CalcResolvedType>
where
  V: 'a,
{
  let mut values = values.into_iter();
  let first = values.next()?.resolved_type()?;
  values
    .all(|value| value.resolved_type() == Some(first))
    .then_some(first)
}

impl<V: IsCompatible> IsCompatible for Calc<V> {
  fn is_compatible(&self, browsers: Browsers) -> bool {
    match self {
      Calc::Sum(a, b) => a.is_compatible(browsers) && b.is_compatible(browsers),
      Calc::Product(_, v) => v.is_compatible(browsers),
      Calc::ProductExpression(left, right) | Calc::QuotientExpression(left, right) => {
        left.is_compatible(browsers) && right.is_compatible(browsers)
      }
      Calc::Function(f) => f.is_compatible(browsers),
      Calc::Value(v) => v.is_compatible(browsers),
      Calc::Number(..) => true,
    }
  }
}

enum_property! {
  /// A mathematical constant.
  pub enum Constant {
    /// The base of the natural logarithm
    "e": E,
    /// The ratio of a circle’s circumference to its diameter
    "pi": Pi,
    /// infinity
    "infinity": Infinity,
    /// -infinity
    "-infinity": NegativeInfinity,
    /// Not a number.
    "nan": Nan,
  }
}

impl Into<f32> for Constant {
  fn into(self) -> f32 {
    use std::f32::consts;
    use Constant::*;
    match self {
      E => consts::E,
      Pi => consts::PI,
      Infinity => f32::INFINITY,
      NegativeInfinity => -f32::INFINITY,
      Nan => f32::NAN,
    }
  }
}

impl<
    'a,
    'i,
    V: Parse<'i>
      + std::ops::Mul<f32, Output = V>
      + AddInternal
      + TryOp
      + TryMap
      + TrySign
      + std::cmp::PartialOrd<V>
      + Into<Calc<V>>
      + TryFrom<Calc<V>>
      + TryFrom<Angle>
      + TryInto<Angle>
      + Clone
      + std::fmt::Debug,
  > CalcParserOps<'i> for TypedCalcParserOps<'a, V>
{
  fn parse_sum<'t>(
    &mut self,
    input: &mut Parser<'i, 't>,
    preserve_math_functions: bool,
  ) -> Result<CalcParserNode, ParseError<'i, ParserError<'i>>> {
    let value = Calc::parse_sum(input, self.parse_ident, preserve_math_functions)?;
    Ok(self.insert(value))
  }

  fn number(&mut self, value: CSSNumber) -> CalcParserNode {
    self.insert(Calc::Number(value))
  }

  fn node_kind(&self, node: CalcParserNode) -> CalcParserNodeKind {
    match self.get(node) {
      Calc::Value(_) => CalcParserNodeKind::Value,
      Calc::Number(value) => CalcParserNodeKind::Number(*value),
      Calc::Function(_) => CalcParserNodeKind::Function,
      _ => CalcParserNodeKind::Other,
    }
  }

  fn function(&mut self, function: CalcParserFunction) -> CalcParserNode {
    let function = match function {
      CalcParserFunction::Min(values) => MathFunction::Min(values.into_iter().map(|value| self.take(value)).collect()),
      CalcParserFunction::Max(values) => MathFunction::Max(values.into_iter().map(|value| self.take(value)).collect()),
      CalcParserFunction::Clamp(min, center, max) => {
        MathFunction::Clamp(self.take(min), self.take(center), self.take(max))
      }
      CalcParserFunction::Round(strategy, value, step) => {
        MathFunction::Round(strategy, self.take(value), self.take(step))
      }
      CalcParserFunction::Rem(value, step) => MathFunction::Rem(self.take(value), self.take(step)),
      CalcParserFunction::Mod(value, step) => MathFunction::Mod(self.take(value), self.take(step)),
      CalcParserFunction::Abs(value) => MathFunction::Abs(self.take(value)),
      CalcParserFunction::Sign(value) => MathFunction::Sign(self.take(value)),
      CalcParserFunction::Hypot(values) => {
        MathFunction::Hypot(values.into_iter().map(|value| self.take(value)).collect())
      }
    };
    self.insert(Calc::Function(Box::new(function)))
  }

  fn compare_values(&self, left: CalcParserNode, right: CalcParserNode) -> Option<Ordering> {
    match (self.get(left), self.get(right)) {
      (Calc::Value(left), Calc::Value(right)) => left.partial_cmp(right),
      _ => None,
    }
  }

  fn apply_binary(
    &mut self,
    left: CalcParserNode,
    right: CalcParserNode,
    op: CalcParserBinaryOp,
  ) -> Option<CalcParserNode> {
    let value = match op {
      CalcParserBinaryOp::Round(strategy) => Calc::apply_op(self.get(left), self.get(right), |a, b| {
        round(a, b, strategy)
      }),
      CalcParserBinaryOp::Rem => Calc::apply_op(self.get(left), self.get(right), std::ops::Rem::rem),
      CalcParserBinaryOp::Mod => Calc::apply_op(self.get(left), self.get(right), modulo),
      CalcParserBinaryOp::Hypot => Calc::apply_op(self.get(left), self.get(right), f32::hypot),
      CalcParserBinaryOp::HypotSum => {
        Calc::apply_op(self.get(left), self.get(right), |a, b| a + b.powi(2))
      }
    }?;
    Some(self.insert(value))
  }

  fn apply_map(&mut self, node: CalcParserNode, op: CalcParserMapOp) -> Option<CalcParserNode> {
    let value = match op {
      CalcParserMapOp::Abs => Calc::apply_map(self.get(node), f32::abs),
      CalcParserMapOp::Square => Calc::apply_map(self.get(node), |value| value.powi(2)),
      CalcParserMapOp::Sqrt => Calc::apply_map(self.get(node), f32::sqrt),
    }?;
    Some(self.insert(value))
  }

  fn value_sign(&self, node: CalcParserNode) -> Option<CSSNumber> {
    match self.get(node) {
      Calc::Value(value) => value.try_map(|value| value.sign()).and_then(|value| value.try_sign()),
      _ => None,
    }
  }

  fn clone_node(&mut self, node: CalcParserNode) -> CalcParserNode {
    self.insert(self.get(node).clone())
  }

  fn parse_identifier_as_angle(&self, identifier: &str) -> Option<Calc<Angle>> {
    (self.parse_ident)(identifier).and_then(|value| match value {
      Calc::Number(value) => Some(Calc::Number(value)),
      Calc::Value(value) => (*value).try_into().ok().map(|value| Calc::Value(Box::new(value))),
      _ => None,
    })
  }

  fn parse_identifier_as_number(&self, identifier: &str) -> Option<Calc<CSSNumber>> {
    (self.parse_ident)(identifier).and_then(|value| match value {
      Calc::Number(value) => Some(Calc::Number(value)),
      _ => None,
    })
  }

  fn from_angle(&mut self, angle: Angle) -> Option<CalcParserNode> {
    V::try_from(angle)
      .ok()
      .map(|value| self.insert(Calc::Value(Box::new(value))))
  }
}

impl<
    'i,
    V: Parse<'i>
      + std::ops::Mul<f32, Output = V>
      + AddInternal
      + TryOp
      + TryMap
      + TrySign
      + std::cmp::PartialOrd<V>
      + Into<Calc<V>>
      + TryFrom<Calc<V>>
      + TryFrom<Angle>
      + TryInto<Angle>
      + Clone
      + std::fmt::Debug,
  > Parse<'i> for Calc<V>
{
  fn parse<'t>(input: &mut Parser<'i, 't>) -> Result<Self, ParseError<'i, ParserError<'i>>> {
    Self::parse_with(input, |_| None)
  }
}

impl<
    'i,
    V: Parse<'i>
      + std::ops::Mul<f32, Output = V>
      + AddInternal
      + TryOp
      + TryMap
      + TrySign
      + std::cmp::PartialOrd<V>
      + Into<Calc<V>>
      + TryFrom<Calc<V>>
      + TryFrom<Angle>
      + TryInto<Angle>
      + Clone
      + std::fmt::Debug,
  > Calc<V>
{
  /// Parses a calculation while retaining the authored shape of math
  /// functions other than `calc()`.
  ///
  /// CSSOM serializers for some descriptor contexts preserve functions such
  /// as `min()`, `max()`, and `round()` even when their result is statically
  /// known. Arithmetic inside `calc()` is still simplified.
  pub fn parse_preserving_math_functions<'t>(
    input: &mut Parser<'i, 't>,
  ) -> Result<Self, ParseError<'i, ParserError<'i>>> {
    Self::parse_with_options(input, &|_| None, true)
  }

  pub(crate) fn parse_with<'t, Parse: Fn(&str) -> Option<Calc<V>>>(
    input: &mut Parser<'i, 't>,
    parse_ident: Parse,
  ) -> Result<Self, ParseError<'i, ParserError<'i>>> {
    Self::parse_with_options(input, &parse_ident, false)
  }

  pub(crate) fn parse_sum_with<'t, Parse: Fn(&str) -> Option<Calc<V>>>(
    input: &mut Parser<'i, 't>,
    parse_ident: Parse,
  ) -> Result<Self, ParseError<'i, ParserError<'i>>> {
    Self::parse_sum(input, &parse_ident, false)
  }

  fn parse_with_options<'t>(
    input: &mut Parser<'i, 't>,
    parse_ident: &CalcIdentifierParser<'_, V>,
    preserve_math_functions: bool,
  ) -> Result<Self, ParseError<'i, ParserError<'i>>> {
    let location = input.current_source_location();
    let function = input.expect_function()?.clone();
    // calc() is overwhelmingly the most common math function. Keep its small dispatch path
    // monomorphized so ordinary calculations do not pay for the arena and dynamic adapter.
    if function.eq_ignore_ascii_case("calc") {
      let value = input.parse_nested_block(|input| Calc::parse_sum(input, parse_ident, preserve_math_functions))?;
      return Ok(match value {
        Calc::Value(_) | Calc::Number(_) => value,
        _ => Calc::Function(Box::new(MathFunction::Calc(value))),
      })
    }

    let mut ops = TypedCalcParserOps::new(parse_ident);
    let root = parse_calc_function(
      input,
      function,
      location,
      &mut ops,
      preserve_math_functions,
    )?;
    Ok(ops.take(root))
  }

  fn parse_sum<'t>(
    input: &mut Parser<'i, 't>,
    parse_ident: &CalcIdentifierParser<'_, V>,
    preserve_math_functions: bool,
  ) -> Result<Self, ParseError<'i, ParserError<'i>>> {
    let mut cur: Calc<V> = Calc::parse_product(input, parse_ident, preserve_math_functions)?;
    loop {
      let start = input.state();
      match input.next_including_whitespace() {
        Ok(&Token::WhiteSpace(_)) => {
          if input.is_exhausted() {
            break; // allow trailing whitespace
          }
          match *input.next()? {
            Token::Delim('+') => {
              let next = Calc::parse_product(input, parse_ident, preserve_math_functions)?;
              cur = if preserve_math_functions
                && matches!(&cur, Calc::Function(_))
                && !matches!(&next, Calc::Function(_))
              {
                next.add(cur)
              } else {
                cur.add(next)
              }
              .map_err(|_| input.new_custom_error(ParserError::InvalidValue))?;
            }
            Token::Delim('-') => {
              let mut rhs = Calc::parse_product(input, parse_ident, preserve_math_functions)?;
              rhs = rhs * -1.0;
              cur = if preserve_math_functions
                && matches!(&cur, Calc::Function(_))
                && !matches!(&rhs, Calc::Function(_))
              {
                rhs.add(cur)
              } else {
                cur.add(rhs)
              }
              .map_err(|_| input.new_custom_error(ParserError::InvalidValue))?;
            }
            ref t => {
              let t = t.clone();
              return Err(input.new_unexpected_token_error(t));
            }
          }
        }
        _ => {
          input.reset(&start);
          break;
        }
      }
    }
    Ok(cur)
  }

  fn parse_product<'t>(
    input: &mut Parser<'i, 't>,
    parse_ident: &CalcIdentifierParser<'_, V>,
    preserve_math_functions: bool,
  ) -> Result<Self, ParseError<'i, ParserError<'i>>> {
    let mut node = Calc::parse_value(input, parse_ident, preserve_math_functions)?;
    loop {
      let start = input.state();
      match input.next() {
        Ok(&Token::Delim('*')) => {
          // At least one of the operands must be a number.
          let rhs = Self::parse_value(input, parse_ident, preserve_math_functions)?;
          if let Calc::Number(val) = rhs {
            node = node * val;
          } else if let Calc::Number(val) = node {
            node = rhs;
            node = node * val;
          } else if node.resolves_to_number() || rhs.resolves_to_number() {
            if node.resolves_to_dimension() && rhs.resolves_to_dimension() {
              return Err(input.new_unexpected_token_error(Token::Delim('*')));
            }
            node = Calc::multiply_expression(node, rhs);
          } else {
            return Err(input.new_unexpected_token_error(Token::Delim('*')));
          }
        }
        Ok(&Token::Delim('/')) => {
          let rhs = Self::parse_value(input, parse_ident, preserve_math_functions)?;
          if let Calc::Number(val) = rhs {
            let mut multiplier = 1.0 / val;
            if !multiplier.is_finite() {
              if let Calc::Value(value) = &node {
                if let Some(basis) = value.try_map(|value| {
                  multiplier *= value;
                  1.0
                }) {
                  node = Calc::Product(multiplier, Box::new(Calc::Value(Box::new(basis))));
                  continue;
                }
              }
            }
            node = node * multiplier;
            continue;
          }
          if rhs.resolves_to_number() {
            node = Calc::QuotientExpression(Box::new(node), Box::new(rhs));
            continue;
          }
          return Err(input.new_custom_error(ParserError::InvalidValue));
        }
        _ => {
          input.reset(&start);
          break;
        }
      }
    }
    Ok(node)
  }

  fn multiply_expression(left: Calc<V>, right: Calc<V>) -> Calc<V> {
    match right {
      Calc::QuotientExpression(numerator, divisor)
        if matches!(&*numerator, Calc::Number(value) if *value == 1.0) =>
      {
        Calc::QuotientExpression(Box::new(left), divisor)
      }
      right => Calc::ProductExpression(Box::new(left), Box::new(right)),
    }
  }

  fn parse_value<'t>(
    input: &mut Parser<'i, 't>,
    parse_ident: &CalcIdentifierParser<'_, V>,
    preserve_math_functions: bool,
  ) -> Result<Self, ParseError<'i, ParserError<'i>>> {
    // Parse nested calc() and other math functions.
    let nested = if preserve_math_functions {
      input.try_parse(Self::parse_preserving_math_functions)
    } else {
      input.try_parse(Self::parse)
    };
    if let Ok(calc) = nested {
      match calc {
        Calc::Function(f) => {
          return Ok(match *f {
            MathFunction::Calc(c) => c,
            _ => Calc::Function(f),
          })
        }
        c => return Ok(c),
      }
    }

    if input.try_parse(|input| input.expect_parenthesis_block()).is_ok() {
      return input.parse_nested_block(|input| Calc::parse_sum(input, parse_ident, preserve_math_functions));
    }

    if let Ok(num) = input.try_parse(|input| input.expect_number()) {
      return Ok(Calc::Number(num));
    }

    if let Ok(constant) = input.try_parse(Constant::parse) {
      return Ok(Calc::Number(constant.into()));
    }

    let identifier_start = input.state();
    if let Ok(ident) = input.try_parse(|input| input.expect_ident_cloned()) {
      if let Some(v) = parse_ident(ident.as_ref()) {
        return Ok(v);
      }
      input.reset(&identifier_start);
    }

    let value = input.try_parse(V::parse)?;
    Ok(Calc::Value(Box::new(value)))
  }

  fn apply_op<'t, O: FnOnce(f32, f32) -> f32>(a: &Calc<V>, b: &Calc<V>, op: O) -> Option<Self> {
    match (a, b) {
      (Calc::Value(a), Calc::Value(b)) => {
        if let Some(v) = a.try_op(&**b, op) {
          return Some(Calc::Value(Box::new(v)));
        }
      }
      (Calc::Number(a), Calc::Number(b)) => return Some(Calc::Number(op(*a, *b))),
      _ => {}
    }

    None
  }

  fn apply_map<'t, O: FnOnce(f32) -> f32>(v: &Calc<V>, op: O) -> Option<Self> {
    match v {
      Calc::Number(n) => return Some(Calc::Number(op(*n))),
      Calc::Value(v) => {
        if let Some(v) = v.try_map(op) {
          return Some(Calc::Value(Box::new(v)));
        }
      }
      _ => {}
    }

    None
  }

  fn parse_static_cross_dimension_sign<'t>(
    input: &mut Parser<'i, 't>,
  ) -> Result<CSSNumber, ParseError<'i, ParserError<'i>>> {
    let location = input.current_source_location();
    let token = input.next()?.clone();
    input.expect_exhausted()?;
    let Token::Dimension { value, unit, .. } = token else {
      return Err(location.new_unexpected_token_error(token));
    };
    if ![
      "px", "cm", "mm", "q", "in", "pt", "pc", "deg", "grad", "rad", "turn", "s", "ms",
    ]
    .iter()
    .any(|candidate| unit.eq_ignore_ascii_case(candidate))
    {
      return Err(location.new_unexpected_token_error(Token::Ident(unit)));
    }
    Ok(value.sign())
  }

  fn parse_atan2_args<'t>(
    input: &mut Parser<'i, 't>,
    parse_ident: &CalcIdentifierParser<'_, V>,
  ) -> Result<Angle, ParseError<'i, ParserError<'i>>> {
    let a = Calc::<V>::parse_sum(input, parse_ident, false)?;
    input.expect_comma()?;
    let b = Calc::<V>::parse_sum(input, parse_ident, false)?;

    match (&a, &b) {
      (Calc::Value(a), Calc::Value(b)) => {
        if let Some(v) = a.try_op_to(&**b, |a, b| Angle::Rad(a.atan2(b))) {
          return Ok(v);
        }
      }
      (Calc::Number(a), Calc::Number(b)) => return Ok(Angle::Rad(a.atan2(*b))),
      _ => {}
    }

    // We don't have a way to represent arguments that aren't angles, so just error.
    // This will fall back to an unparsed property, leaving the atan2() function intact.
    Err(input.new_custom_error(ParserError::InvalidValue))
  }
}

fn parse_calc_function<'i, 't>(
  input: &mut Parser<'i, 't>,
  function: CowRcStr<'i>,
  location: SourceLocation,
  ops: &mut dyn CalcParserOps<'i>,
  preserve_math_functions: bool,
) -> Result<CalcParserNode, ParseError<'i, ParserError<'i>>> {
  match_ignore_ascii_case! { &function,
    "min" => {
      let args = input.parse_nested_block(|input| {
        input.parse_comma_separated(|input| ops.parse_sum(input, preserve_math_functions))
      })?;
      if preserve_math_functions {
        return Ok(ops.function(CalcParserFunction::Min(args)))
      }
      let mut reduced = reduce_calc_parser_args(ops, args, Ordering::Less);
      if reduced.len() == 1 {
        return Ok(reduced.remove(0))
      }
      Ok(ops.function(CalcParserFunction::Min(reduced)))
    },
    "max" => {
      let args = input.parse_nested_block(|input| {
        input.parse_comma_separated(|input| ops.parse_sum(input, preserve_math_functions))
      })?;
      if preserve_math_functions {
        return Ok(ops.function(CalcParserFunction::Max(args)))
      }
      let mut reduced = reduce_calc_parser_args(ops, args, Ordering::Greater);
      if reduced.len() == 1 {
        return Ok(reduced.remove(0))
      }
      Ok(ops.function(CalcParserFunction::Max(reduced)))
    },
    "clamp" => {
      let (mut min, mut center, mut max) = input.parse_nested_block(|input| {
        let min = Some(ops.parse_sum(input, preserve_math_functions)?);
        input.expect_comma()?;
        let center = ops.parse_sum(input, preserve_math_functions)?;
        input.expect_comma()?;
        let max = Some(ops.parse_sum(input, preserve_math_functions)?);
        Ok((min, center, max))
      })?;

      if preserve_math_functions {
        return Ok(ops.function(CalcParserFunction::Clamp(
          min.take().unwrap(),
          center,
          max.take().unwrap(),
        )))
      }

      // According to the spec, the minimum wins over the maximum if they are in the wrong order.
      let comparison = max.and_then(|max| compare_calc_parser_nodes(ops, center, max));
      match comparison {
        Some(Ordering::Greater) => center = max.take().unwrap(),
        Some(_) => max = None,
        None => {}
      }

      if comparison.is_some() {
        let comparison = min.and_then(|min| compare_calc_parser_nodes(ops, center, min));
        match comparison {
          Some(Ordering::Less) => center = min.take().unwrap(),
          Some(_) => min = None,
          None => {}
        }
      }

      match (min, max) {
        (None, None) => Ok(center),
        (Some(min), None) => Ok(ops.function(CalcParserFunction::Max(vec![min, center]))),
        (None, Some(max)) => Ok(ops.function(CalcParserFunction::Min(vec![center, max]))),
        (Some(min), Some(max)) => Ok(ops.function(CalcParserFunction::Clamp(min, center, max))),
      }
    },
    "round" => input.parse_nested_block(|input| {
      let strategy = if let Ok(strategy) = input.try_parse(RoundingStrategy::parse) {
        input.expect_comma()?;
        strategy
      } else {
        RoundingStrategy::default()
      };
      parse_calc_binary_function(
        input,
        ops,
        CalcParserBinaryOp::Round(strategy),
        preserve_math_functions,
      )
    }),
    "rem" => input.parse_nested_block(|input| {
      parse_calc_binary_function(input, ops, CalcParserBinaryOp::Rem, preserve_math_functions)
    }),
    "mod" => input.parse_nested_block(|input| {
      parse_calc_binary_function(input, ops, CalcParserBinaryOp::Mod, preserve_math_functions)
    }),
    "sin" => parse_calc_trig(input, ops, f32::sin, false),
    "cos" => parse_calc_trig(input, ops, f32::cos, false),
    "tan" => parse_calc_trig(input, ops, f32::tan, false),
    "asin" => parse_calc_trig(input, ops, f32::asin, true),
    "acos" => parse_calc_trig(input, ops, f32::acos, true),
    "atan" => parse_calc_trig(input, ops, f32::atan, true),
    "atan2" => input.parse_nested_block(|input| {
      let angle = parse_calc_atan2(input, ops)?;
      ops.from_angle(angle).ok_or_else(|| input.new_custom_error(ParserError::InvalidValue))
    }),
    "pow" => input.parse_nested_block(|input| {
      let left = parse_calc_numeric(input, ops)?;
      input.expect_comma()?;
      let right = parse_calc_numeric(input, ops)?;
      Ok(ops.number(left.powf(right)))
    }),
    "log" => input.parse_nested_block(|input| {
      let value = parse_calc_numeric(input, ops)?;
      let result = if input.try_parse(|input| input.expect_comma()).is_ok() {
        value.log(parse_calc_numeric(input, ops)?)
      } else {
        value.ln()
      };
      Ok(ops.number(result))
    }),
    "sqrt" => parse_calc_numeric_function(input, ops, f32::sqrt),
    "exp" => parse_calc_numeric_function(input, ops, f32::exp),
    "hypot" => input.parse_nested_block(|input| {
      let args = input.parse_comma_separated(|input| ops.parse_sum(input, preserve_math_functions))?;
      if preserve_math_functions {
        return Ok(ops.function(CalcParserFunction::Hypot(args)))
      }
      parse_calc_hypot(ops, &args)
        .map_or_else(|| Ok(ops.function(CalcParserFunction::Hypot(args))), Ok)
    }),
    "abs" => input.parse_nested_block(|input| {
      let value = ops.parse_sum(input, preserve_math_functions)?;
      if preserve_math_functions {
        return Ok(ops.function(CalcParserFunction::Abs(value)))
      }
      Ok(ops.apply_map(value, CalcParserMapOp::Abs)
        .unwrap_or_else(|| ops.function(CalcParserFunction::Abs(value))))
    }),
    "sign" => input.parse_nested_block(|input| {
      let start = input.state();
      let value = match ops.parse_sum(input, preserve_math_functions) {
        Ok(value) => value,
        Err(error) => {
          input.reset(&start);
          if let Ok(sign) = Calc::<CSSNumber>::parse_static_cross_dimension_sign(input) {
            return Ok(ops.number(sign));
          }
          return Err(error)
        }
      };
      if preserve_math_functions {
        return Ok(ops.function(CalcParserFunction::Sign(value)))
      }
      match ops.node_kind(value) {
        CalcParserNodeKind::Number(number) => return Ok(ops.number(number.sign())),
        CalcParserNodeKind::Value => {
          // First map so percentages are ignored. Their sign depends on their computed value.
          if let Some(sign) = ops.value_sign(value) {
            return Ok(ops.number(sign))
          }
        }
        _ => {}
      }
      Ok(ops.function(CalcParserFunction::Sign(value)))
    }),
    _ => Err(location.new_unexpected_token_error(Token::Ident(function.clone())))
  }
}

fn compare_calc_parser_nodes<'i>(
  ops: &dyn CalcParserOps<'i>,
  left: CalcParserNode,
  right: CalcParserNode,
) -> Option<Ordering> {
  match (ops.node_kind(left), ops.node_kind(right)) {
    (CalcParserNodeKind::Value, CalcParserNodeKind::Value) => ops.compare_values(left, right),
    (CalcParserNodeKind::Number(left), CalcParserNodeKind::Number(right)) => left.partial_cmp(&right),
    _ => None,
  }
}

fn reduce_calc_parser_args<'i>(
  ops: &dyn CalcParserOps<'i>,
  args: Vec<CalcParserNode>,
  comparison: Ordering,
) -> Vec<CalcParserNode> {
  // Combine compatible values in min() and max(), e.g. min(1px, 1em, 2px, 3in)
  // becomes min(1px, 1em). Plain numbers are reduced the same way.
  let mut reduced = Vec::new();
  for argument in args {
    let mut found = None;
    for (index, candidate) in reduced.iter().copied().enumerate() {
      if let Some(ordering) = compare_calc_parser_nodes(ops, argument, candidate) {
        found = Some((index, ordering == comparison));
        break;
      }
    }
    match found {
      Some((index, true)) => reduced[index] = argument,
      Some((_, false)) => {}
      None => reduced.push(argument),
    }
  }
  reduced
}

fn parse_calc_binary_function<'i, 't>(
  input: &mut Parser<'i, 't>,
  ops: &mut dyn CalcParserOps<'i>,
  operation: CalcParserBinaryOp,
  preserve_math_functions: bool,
) -> Result<CalcParserNode, ParseError<'i, ParserError<'i>>> {
  let left = ops.parse_sum(input, preserve_math_functions)?;
  input.expect_comma()?;
  let right = ops.parse_sum(input, preserve_math_functions)?;

  let fallback = || match operation {
    CalcParserBinaryOp::Round(strategy) => CalcParserFunction::Round(strategy, left, right),
    CalcParserBinaryOp::Rem => CalcParserFunction::Rem(left, right),
    CalcParserBinaryOp::Mod => CalcParserFunction::Mod(left, right),
    _ => unreachable!(),
  };
  if preserve_math_functions {
    return Ok(ops.function(fallback()))
  }
  Ok(ops.apply_binary(left, right, operation).unwrap_or_else(|| ops.function(fallback())))
}

fn parse_calc_trig<'i, 't>(
  input: &mut Parser<'i, 't>,
  ops: &mut dyn CalcParserOps<'i>,
  function: fn(f32) -> f32,
  to_angle: bool,
) -> Result<CalcParserNode, ParseError<'i, ParserError<'i>>> {
  input.parse_nested_block(|input| {
    let value: Calc<Angle> = Calc::parse_sum_with(input, |identifier| {
      ops.parse_identifier_as_angle(identifier)
    })?;
    let radians = match value {
      Calc::Value(angle) if !to_angle => function(angle.to_radians()),
      Calc::Number(value) => function(value),
      _ => return Err(input.new_custom_error(ParserError::InvalidValue)),
    };
    if to_angle && !radians.is_nan() {
      ops.from_angle(Angle::Rad(radians))
        .ok_or_else(|| input.new_custom_error(ParserError::InvalidValue))
    } else {
      Ok(ops.number(radians))
    }
  })
}

fn parse_calc_numeric<'i, 't>(
  input: &mut Parser<'i, 't>,
  ops: &dyn CalcParserOps<'i>,
) -> Result<CSSNumber, ParseError<'i, ParserError<'i>>> {
  let value: Calc<CSSNumber> = Calc::parse_sum_with(input, |identifier| {
    ops.parse_identifier_as_number(identifier)
  })?;
  match value {
    Calc::Number(value) => Ok(value),
    Calc::Value(value) => Ok(*value),
    _ => Err(input.new_custom_error(ParserError::InvalidValue)),
  }
}

fn parse_calc_numeric_function<'i, 't>(
  input: &mut Parser<'i, 't>,
  ops: &mut dyn CalcParserOps<'i>,
  function: fn(f32) -> f32,
) -> Result<CalcParserNode, ParseError<'i, ParserError<'i>>> {
  input.parse_nested_block(|input| {
    let value = parse_calc_numeric(input, ops)?;
    Ok(ops.number(function(value)))
  })
}

fn parse_calc_atan2<'i, 't>(
  input: &mut Parser<'i, 't>,
  ops: &dyn CalcParserOps<'i>,
) -> Result<Angle, ParseError<'i, ParserError<'i>>> {
  // atan2 accepts any number, dimension, or percentage pair of the same type, including types
  // that the outer V does not normally support. Try each concrete type before plain numbers.
  if let Ok(value) = input.try_parse(|input| Calc::<Length>::parse_atan2_args(input, &|_| None)) {
    return Ok(value)
  }
  if let Ok(value) = input.try_parse(|input| Calc::<Percentage>::parse_atan2_args(input, &|_| None)) {
    return Ok(value)
  }
  if let Ok(value) = input.try_parse(|input| Calc::<Angle>::parse_atan2_args(input, &|_| None)) {
    return Ok(value)
  }
  if let Ok(value) = input.try_parse(|input| Calc::<Time>::parse_atan2_args(input, &|_| None)) {
    return Ok(value)
  }
  Calc::<CSSNumber>::parse_atan2_args(input, &|identifier| {
    ops.parse_identifier_as_number(identifier)
  })
}

fn parse_calc_hypot<'i>(
  ops: &mut dyn CalcParserOps<'i>,
  args: &[CalcParserNode],
) -> Option<CalcParserNode> {
  if args.len() == 1 {
    return Some(ops.clone_node(args[0]))
  }
  if args.len() == 2 {
    return ops.apply_binary(args[0], args[1], CalcParserBinaryOp::Hypot)
  }
  let mut args = args.iter().copied();
  let first = ops.apply_map(args.next()?, CalcParserMapOp::Square)?;
  let sum = args.try_fold(first, |sum, argument| {
    ops.apply_binary(sum, argument, CalcParserBinaryOp::HypotSum)
  })?;
  ops.apply_map(sum, CalcParserMapOp::Sqrt)
}

impl<V: std::ops::Mul<f32, Output = V>> std::ops::Mul<f32> for Calc<V> {
  type Output = Self;

  fn mul(self, other: f32) -> Self {
    if other == 1.0 {
      return self;
    }

    // Preserve symbolic non-finite coefficients. Multiplying them into a
    // dimension value would turn `infinity`/`NaN` into an implementation-limit
    // float and make CSSOM serialization observably incorrect.
    if !other.is_finite() && !matches!(self, Calc::Number(_)) {
      return Calc::Product(other, Box::new(self));
    }

    match self {
      Calc::Value(v) => Calc::Value(Box::new(*v * other)),
      Calc::Number(n) => Calc::Number(n * other),
      Calc::Sum(a, b) => Calc::Sum(Box::new(*a * other), Box::new(*b * other)),
      Calc::Product(num, calc) => {
        let num = num * other;
        if num == 1.0 {
          return *calc;
        }
        Calc::Product(num, calc)
      }
      Calc::ProductExpression(..) | Calc::QuotientExpression(..) => Calc::Product(other, Box::new(self)),
      Calc::Function(f) => match *f {
        MathFunction::Calc(c) => Calc::Function(Box::new(MathFunction::Calc(c * other))),
        _ => Calc::Product(other, Box::new(Calc::Function(f))),
      },
    }
  }
}

impl<V: AddInternal + std::convert::Into<Calc<V>> + std::convert::TryFrom<Calc<V>> + std::fmt::Debug> Calc<V> {
  pub(crate) fn add(self, other: Calc<V>) -> Result<Calc<V>, <V as TryFrom<Calc<V>>>::Error> {
    Ok(match (self, other) {
      (Calc::Value(a), Calc::Value(b)) => (a.add(*b)).into(),
      (Calc::Number(a), Calc::Number(b)) => Calc::Number(a + b),
      (Calc::Sum(a, b), Calc::Number(c)) => {
        if let Calc::Number(a) = *a {
          Calc::Sum(Box::new(Calc::Number(a + c)), b)
        } else if let Calc::Number(b) = *b {
          Calc::Sum(a, Box::new(Calc::Number(b + c)))
        } else {
          Calc::Sum(Box::new(Calc::Sum(a, b)), Box::new(Calc::Number(c)))
        }
      }
      (Calc::Number(a), Calc::Sum(b, c)) => {
        if let Calc::Number(b) = *b {
          Calc::Sum(Box::new(Calc::Number(a + b)), c)
        } else if let Calc::Number(c) = *c {
          Calc::Sum(Box::new(Calc::Number(a + c)), b)
        } else {
          Calc::Sum(Box::new(Calc::Number(a)), Box::new(Calc::Sum(b, c)))
        }
      }
      (a @ Calc::Number(_), b) => Calc::Sum(Box::new(a), Box::new(b)),
      (a, b @ Calc::Number(_)) => Calc::Sum(Box::new(b), Box::new(a)),
      (a @ Calc::Product(..), b)
      | (a, b @ Calc::Product(..))
      | (a @ Calc::ProductExpression(..), b)
      | (a, b @ Calc::ProductExpression(..))
      | (a @ Calc::QuotientExpression(..), b)
      | (a, b @ Calc::QuotientExpression(..)) => Calc::Sum(Box::new(a), Box::new(b)),
      (Calc::Function(a), b) => Calc::Sum(Box::new(Calc::Function(a)), Box::new(b)),
      (a, Calc::Function(b)) => Calc::Sum(Box::new(a), Box::new(Calc::Function(b))),
      (Calc::Value(a), b) => (a.add(V::try_from(b)?)).into(),
      (a, Calc::Value(b)) => (V::try_from(a)?.add(*b)).into(),
      (a @ Calc::Sum(..), b @ Calc::Sum(..)) => V::try_from(a)?.add(V::try_from(b)?).into(),
    })
  }
}

impl<V: ToCss + std::ops::Mul<f32, Output = V> + TrySign + Clone + std::fmt::Debug> ToCss for Calc<V> {
  fn to_css<W>(&self, dest: &mut Printer<W>) -> Result<(), PrinterError>
  where
    W: std::fmt::Write,
  {
    let was_in_calc = dest.in_calc;
    dest.in_calc = true;

    let res = match self {
      Calc::Value(v) => v.to_css(dest),
      Calc::Number(n) => n.to_css(dest),
      Calc::Sum(a, b) => {
        a.to_css(dest)?;
        // Whitespace is always required.
        let b = &**b;
        if b.is_sign_negative() {
          dest.write_str(" - ")?;
          let b = b.clone() * -1.0;
          b.to_css(dest)
        } else {
          dest.write_str(" + ")?;
          b.to_css(dest)
        }
      }
      Calc::Product(num, calc) => {
        if num.abs() < 1.0 {
          let div = 1.0 / num;
          calc.to_css(dest)?;
          dest.delim('/', true)?;
          div.to_css(dest)
        } else {
          num.to_css(dest)?;
          dest.delim('*', true)?;
          calc.to_css(dest)
        }
      }
      Calc::ProductExpression(left, right) => {
        let (left, right) = if left.resolves_to_number() && right.resolves_to_dimension() {
          (right, left)
        } else {
          (left, right)
        };
        left.to_css(dest)?;
        dest.delim('*', true)?;
        right.to_css(dest)
      }
      Calc::QuotientExpression(value, divisor) => {
        value.to_css(dest)?;
        dest.delim('*', true)?;
        dest.write_str("(1 / ")?;
        divisor.to_css(dest)?;
        dest.write_char(')')
      }
      Calc::Function(f) => f.to_css(dest),
    };

    dest.in_calc = was_in_calc;
    res
  }
}

impl<V: TrySign> TrySign for Calc<V> {
  fn try_sign(&self) -> Option<f32> {
    match self {
      Calc::Number(v) => v.try_sign(),
      Calc::Value(v) => v.try_sign(),
      Calc::Product(c, v) => v.try_sign().map(|s| s * c.sign()),
      Calc::ProductExpression(left, right) => {
        Some(left.try_sign()? * right.try_sign()?)
      }
      Calc::QuotientExpression(value, divisor) => {
        Some(value.try_sign()? / divisor.try_sign()?)
      }
      Calc::Function(f) => f.try_sign(),
      _ => None,
    }
  }
}

impl<V: TrySign> TrySign for MathFunction<V> {
  fn try_sign(&self) -> Option<f32> {
    match self {
      MathFunction::Abs(_) => Some(1.0),
      MathFunction::Max(values) | MathFunction::Min(values) => {
        let mut iter = values.iter();
        if let Some(sign) = iter.next().and_then(|f| f.try_sign()) {
          for value in iter {
            if let Some(s) = value.try_sign() {
              if s != sign {
                return None;
              }
            } else {
              return None;
            }
          }
          return Some(sign);
        } else {
          return None;
        }
      }
      MathFunction::Clamp(a, b, c) => {
        if let (Some(a), Some(b), Some(c)) = (a.try_sign(), b.try_sign(), c.try_sign()) {
          if a == b && b == c {
            return Some(a);
          }
        }
        return None;
      }
      MathFunction::Round(_, a, b) => {
        if let (Some(a), Some(b)) = (a.try_sign(), b.try_sign()) {
          if a == b {
            return Some(a);
          }
        }
        return None;
      }
      MathFunction::Sign(v) => v.try_sign(),
      MathFunction::Calc(v) => v.try_sign(),
      _ => None,
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::stylesheet::PrinterOptions;

  fn parse_percentage(source: &str, preserve_math_functions: bool) -> Calc<Percentage> {
    let mut input = ParserInput::new(source);
    let mut parser = Parser::new(&mut input);
    parser
      .parse_entirely(|input| {
        if preserve_math_functions {
          Calc::parse_preserving_math_functions(input)
        } else {
          Calc::parse(input)
        }
      })
      .unwrap()
  }

  fn serialize(value: &Calc<Percentage>) -> String {
    value.to_css_string(PrinterOptions::default()).unwrap()
  }

  #[test]
  fn optionally_preserves_math_function_shape() {
    for (source, expected) in [
      ("min( 10%,20%)", "min(10%, 20%)"),
      ("max(10%, 20%)", "max(10%, 20%)"),
      ("clamp(10%,20%, 30%)", "clamp(10%, 20%, 30%)"),
      ("round(10%,3%)", "round(10%, 3%)"),
      ("mod(10%,3%)", "mod(10%, 3%)"),
      ("rem(10%,3%)", "rem(10%, 3%)"),
      ("abs(-10%)", "abs(-10%)"),
      ("hypot(3%,4%)", "hypot(3%, 4%)"),
      ("min(max(10%,20%),30%)", "min(max(10%, 20%), 30%)"),
      ("calc(min(10%,20%) + 5%)", "calc(5% + min(10%, 20%))"),
    ] {
      assert_eq!(serialize(&parse_percentage(source, true)), expected, "{source}");
    }
  }

  #[test]
  fn default_parser_keeps_reducing_static_math_functions() {
    assert_eq!(serialize(&parse_percentage("min(10%, 20%)", false)), "10%");
    assert_eq!(serialize(&parse_percentage("clamp(10%, 20%, 30%)", false)), "20%");
  }

  #[test]
  fn distinguishes_invalid_mixed_sum_result_types() {
    let percentage = parse_percentage("min(10%, 20%)", true);
    assert!(percentage.resolves_to_dimension());
    assert!(!percentage.resolves_to_number());

    let mixed = parse_percentage("calc(1 + 1%)", true);
    assert!(!mixed.resolves_to_dimension());
    assert!(!mixed.resolves_to_number());
  }
}
