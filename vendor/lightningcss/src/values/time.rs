//! CSS time values.

use super::angle::impl_try_from_angle;
use super::calc::{Calc, MathFunction};
use super::number::CSSNumber;
use crate::error::{ParserError, PrinterError};
use crate::printer::Printer;
use crate::traits::private::AddInternal;
use crate::traits::{Parse, Sign, ToCss, TryMap, TryOp, TrySign, Zero};
#[cfg(feature = "visitor")]
use crate::visitor::Visit;
use cssparser::*;

/// A CSS [`<time>`](https://www.w3.org/TR/css-values-4/#time) value, in either
/// seconds or milliseconds.
///
/// Time values may be explicit or computed by `calc()`, but are always stored and serialized
/// as their computed value.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "visitor", derive(Visit))]
#[cfg_attr(feature = "visitor", visit(visit_time, TIMES))]
#[cfg_attr(
  feature = "serde",
  derive(serde::Serialize, serde::Deserialize),
  serde(tag = "type", content = "value", rename_all = "kebab-case")
)]
#[cfg_attr(feature = "jsonschema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "into_owned", derive(static_self::IntoOwned))]
pub enum Time {
  /// A calculation that depends on element or layout context.
  Calculation(Box<Calc<Time>>),
  /// A time in seconds.
  Seconds(CSSNumber),
  /// A time in milliseconds.
  Milliseconds(CSSNumber),
}

impl Time {
  /// Returns the time in milliseconds.
  pub fn to_ms(&self) -> CSSNumber {
    match self {
      Time::Calculation(_) => f32::NAN,
      Time::Seconds(s) => s * 1000.0,
      Time::Milliseconds(ms) => *ms,
    }
  }
}

impl Zero for Time {
  fn zero() -> Self {
    Time::Milliseconds(0.0)
  }

  fn is_zero(&self) -> bool {
    match self {
      Time::Calculation(_) => false,
      Time::Seconds(s) => s.is_zero(),
      Time::Milliseconds(s) => s.is_zero(),
    }
  }
}

impl<'i> Parse<'i> for Time {
  fn parse<'t>(input: &mut Parser<'i, 't>) -> Result<Self, ParseError<'i, ParserError<'i>>> {
    match input.try_parse(Calc::parse) {
      Ok(Calc::Value(v)) => return Ok(*v),
      Ok(value) if value.resolves_to_dimension() => return Ok(Self::Calculation(Box::new(value))),
      Ok(_) => return Err(input.new_custom_error(ParserError::InvalidValue)),
      _ => {}
    }

    let location = input.current_source_location();
    match *input.next()? {
      Token::Dimension { value, ref unit, .. } => {
        match_ignore_ascii_case! { unit,
          "s" => Ok(Time::Seconds(value)),
          "ms" => Ok(Time::Milliseconds(value)),
          _ => Err(location.new_unexpected_token_error(Token::Ident(unit.clone())))
        }
      }
      ref t => Err(location.new_unexpected_token_error(t.clone())),
    }
  }
}

impl<'i> TryFrom<&Token<'i>> for Time {
  type Error = ();

  fn try_from(token: &Token) -> Result<Self, Self::Error> {
    match token {
      Token::Dimension { value, ref unit, .. } => match_ignore_ascii_case! { unit,
        "s" => Ok(Time::Seconds(*value)),
        "ms" => Ok(Time::Milliseconds(*value)),
        _ => Err(()),
      },
      _ => Err(()),
    }
  }
}

impl ToCss for Time {
  fn to_css<W>(&self, dest: &mut Printer<W>) -> Result<(), PrinterError>
  where
    W: std::fmt::Write,
  {
    if let Self::Calculation(value) = self {
      return match value.as_ref() {
        Calc::Function(_) => value.to_css(dest),
        value => MathFunction::Calc(value.clone()).to_css(dest),
      };
    }
    // 0.1s is shorter than 100ms
    // anything smaller is longer
    match self {
      Time::Calculation(_) => unreachable!(),
      Time::Seconds(s) => {
        if *s > 0.0 && *s < 0.1 {
          (*s * 1000.0).to_css(dest)?;
          dest.write_str("ms")
        } else {
          s.to_css(dest)?;
          dest.write_str("s")
        }
      }
      Time::Milliseconds(ms) => {
        if *ms == 0.0 || *ms >= 100.0 {
          (*ms / 1000.0).to_css(dest)?;
          dest.write_str("s")
        } else {
          ms.to_css(dest)?;
          dest.write_str("ms")
        }
      }
    }
  }
}

impl std::convert::Into<Calc<Time>> for Time {
  fn into(self) -> Calc<Time> {
    match self {
      Self::Calculation(value) => *value,
      value => Calc::Value(Box::new(value)),
    }
  }
}

impl std::convert::TryFrom<Calc<Time>> for Time {
  type Error = ();

  fn try_from(calc: Calc<Time>) -> Result<Time, Self::Error> {
    match calc {
      Calc::Value(v) => Ok(*v),
      value if value.resolves_to_dimension() => Ok(Self::Calculation(Box::new(value))),
      _ => Err(()),
    }
  }
}

impl std::ops::Mul<f32> for Time {
  type Output = Self;

  fn mul(self, other: f32) -> Time {
    match self {
      Self::Calculation(value) => Self::Calculation(Box::new(*value * other)),
      Time::Seconds(t) => Time::Seconds(t * other),
      Time::Milliseconds(t) => Time::Milliseconds(t * other),
    }
  }
}

impl AddInternal for Time {
  fn add(self, other: Self) -> Self {
    self + other
  }
}

impl std::cmp::PartialOrd<Time> for Time {
  fn partial_cmp(&self, other: &Time) -> Option<std::cmp::Ordering> {
    self.to_ms().partial_cmp(&other.to_ms())
  }
}

impl TryOp for Time {
  fn try_op<F: FnOnce(f32, f32) -> f32>(&self, to: &Self, op: F) -> Option<Self> {
    if matches!(self, Self::Calculation(_)) || matches!(to, Self::Calculation(_)) {
      return None;
    }
    Some(match (self, to) {
      (Time::Calculation(_), _) | (_, Time::Calculation(_)) => unreachable!(),
      (Time::Seconds(a), Time::Seconds(b)) => Time::Seconds(op(*a, *b)),
      (Time::Milliseconds(a), Time::Milliseconds(b)) => Time::Milliseconds(op(*a, *b)),
      (Time::Seconds(a), Time::Milliseconds(b)) => Time::Seconds(op(*a, b / 1000.0)),
      (Time::Milliseconds(a), Time::Seconds(b)) => Time::Milliseconds(op(*a, b * 1000.0)),
    })
  }

  fn try_op_to<T, F: FnOnce(f32, f32) -> T>(&self, rhs: &Self, op: F) -> Option<T> {
    if matches!(self, Self::Calculation(_)) || matches!(rhs, Self::Calculation(_)) {
      return None;
    }
    Some(match (self, rhs) {
      (Time::Calculation(_), _) | (_, Time::Calculation(_)) => unreachable!(),
      (Time::Seconds(a), Time::Seconds(b)) => op(*a, *b),
      (Time::Milliseconds(a), Time::Milliseconds(b)) => op(*a, *b),
      (Time::Seconds(a), Time::Milliseconds(b)) => op(*a, b / 1000.0),
      (Time::Milliseconds(a), Time::Seconds(b)) => op(*a, b * 1000.0),
    })
  }
}

impl TryMap for Time {
  fn try_map<F: FnOnce(f32) -> f32>(&self, op: F) -> Option<Self> {
    if matches!(self, Self::Calculation(_)) {
      return None;
    }
    Some(match self {
      Self::Calculation(_) => unreachable!(),
      Time::Seconds(t) => Time::Seconds(op(*t)),
      Time::Milliseconds(t) => Time::Milliseconds(op(*t)),
    })
  }
}

impl TrySign for Time {
  fn try_sign(&self) -> Option<f32> {
    match self {
      Self::Calculation(_) => None,
      Time::Seconds(v) | Time::Milliseconds(v) => Some(v.sign()),
    }
  }
}

impl std::ops::Add for Time {
  type Output = Self;
  fn add(self, other: Self) -> Self {
    self
      .try_op(&other, |a, b| a + b)
      .unwrap_or_else(|| Self::Calculation(Box::new(Calc::Sum(Box::new(self.into()), Box::new(other.into())))))
  }
}
impl std::ops::Rem for Time {
  type Output = Self;
  fn rem(self, other: Self) -> Self {
    self.try_op(&other, |a, b| a % b).unwrap_or_else(|| {
      Self::Calculation(Box::new(Calc::Function(Box::new(MathFunction::Rem(
        self.into(),
        other.into(),
      )))))
    })
  }
}

impl_try_from_angle!(Time);
