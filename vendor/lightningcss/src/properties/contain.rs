//! CSS properties related to containment.

#![allow(non_upper_case_globals)]

use cssparser::*;
use smallvec::SmallVec;

#[cfg(feature = "visitor")]
use crate::visitor::Visit;
use crate::{
  context::PropertyHandlerContext,
  declaration::{DeclarationBlock, DeclarationList},
  error::{ParserError, PrinterError},
  macros::{define_shorthand, shorthand_handler},
  printer::Printer,
  properties::{Property, PropertyId},
  rules::container::ContainerName as ContainerIdent,
  targets::Browsers,
  traits::{IsCompatible, Parse, PropertyHandler, Shorthand, ToCss},
};

/// A value for the [container-type](https://drafts.csswg.org/css-contain-3/#container-type) property.
/// Establishes the element as a query container for size and scroll-state queries.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "visitor", derive(Visit))]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "jsonschema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "into_owned", derive(static_self::IntoOwned))]
pub enum ContainerType {
  /// The element is not a size or scroll-state query container.
  #[cfg_attr(feature = "serde", serde(rename = "normal"))]
  Normal,
  /// Establishes size containment on the inline axis.
  #[cfg_attr(feature = "serde", serde(rename = "inline-size"))]
  InlineSize,
  /// Establishes size containment on both axes.
  #[cfg_attr(feature = "serde", serde(rename = "size"))]
  Size,
  /// Establishes a scroll-state query container.
  #[cfg_attr(feature = "serde", serde(rename = "scroll-state"))]
  ScrollState,
  /// Establishes inline-size and scroll-state containment.
  #[cfg_attr(feature = "serde", serde(rename = "inline-size scroll-state"))]
  InlineSizeScrollState,
  /// Establishes size and scroll-state containment.
  #[cfg_attr(feature = "serde", serde(rename = "size scroll-state"))]
  SizeScrollState,
}

impl<'i> Parse<'i> for ContainerType {
  fn parse<'t>(input: &mut Parser<'i, 't>) -> Result<Self, ParseError<'i, ParserError<'i>>> {
    if input.try_parse(|input| input.expect_ident_matching("normal")).is_ok() {
      return Ok(Self::Normal);
    }

    let mut size = None;
    let mut scroll_state = false;
    loop {
      if size.is_none() && input.try_parse(|input| input.expect_ident_matching("inline-size")).is_ok() {
        size = Some(Self::InlineSize);
        continue;
      }
      if size.is_none() && input.try_parse(|input| input.expect_ident_matching("size")).is_ok() {
        size = Some(Self::Size);
        continue;
      }
      if !scroll_state && input.try_parse(|input| input.expect_ident_matching("scroll-state")).is_ok() {
        scroll_state = true;
        continue;
      }
      break;
    }

    match (size, scroll_state) {
      (Some(Self::InlineSize), true) => Ok(Self::InlineSizeScrollState),
      (Some(Self::Size), true) => Ok(Self::SizeScrollState),
      (Some(value), false) => Ok(value),
      (None, true) => Ok(Self::ScrollState),
      _ => Err(input.new_custom_error(ParserError::InvalidValue)),
    }
  }
}

impl ContainerType {
  /// Returns the canonical CSS representation.
  pub fn as_str(&self) -> &str {
    match self {
      Self::Normal => "normal",
      Self::InlineSize => "inline-size",
      Self::Size => "size",
      Self::ScrollState => "scroll-state",
      Self::InlineSizeScrollState => "inline-size scroll-state",
      Self::SizeScrollState => "size scroll-state",
    }
  }
}

impl ToCss for ContainerType {
  fn to_css<W>(&self, dest: &mut Printer<W>) -> Result<(), PrinterError>
  where
    W: std::fmt::Write,
  {
    dest.write_str(self.as_str())
  }
}

impl Default for ContainerType {
  fn default() -> Self {
    ContainerType::Normal
  }
}

impl IsCompatible for ContainerType {
  fn is_compatible(&self, _browsers: Browsers) -> bool {
    true
  }
}

/// A value for the [container-name](https://drafts.csswg.org/css-contain-3/#container-name) property.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "visitor", derive(Visit))]
#[cfg_attr(feature = "into_owned", derive(static_self::IntoOwned))]
#[cfg_attr(
  feature = "serde",
  derive(serde::Serialize, serde::Deserialize),
  serde(tag = "type", content = "value", rename_all = "kebab-case")
)]
#[cfg_attr(feature = "jsonschema", derive(schemars::JsonSchema))]
pub enum ContainerNameList<'i> {
  /// The `none` keyword.
  None,
  /// A list of container names.
  #[cfg_attr(feature = "serde", serde(borrow))]
  Names(SmallVec<[ContainerIdent<'i>; 1]>),
}

impl<'i> Default for ContainerNameList<'i> {
  fn default() -> Self {
    ContainerNameList::None
  }
}

impl<'i> Parse<'i> for ContainerNameList<'i> {
  fn parse<'t>(input: &mut Parser<'i, 't>) -> Result<Self, ParseError<'i, ParserError<'i>>> {
    if input.try_parse(|input| input.expect_ident_matching("none")).is_ok() {
      return Ok(ContainerNameList::None);
    }

    let mut names = SmallVec::new();
    while let Ok(name) = input.try_parse(ContainerIdent::parse) {
      names.push(name);
    }

    if names.is_empty() {
      return Err(input.new_error_for_next_token());
    } else {
      return Ok(ContainerNameList::Names(names));
    }
  }
}

impl<'i> ToCss for ContainerNameList<'i> {
  fn to_css<W>(&self, dest: &mut Printer<W>) -> Result<(), PrinterError>
  where
    W: std::fmt::Write,
  {
    match self {
      ContainerNameList::None => dest.write_str("none"),
      ContainerNameList::Names(names) => {
        let mut first = true;
        for name in names {
          if first {
            first = false;
          } else {
            dest.write_char(' ')?;
          }
          name.to_css(dest)?;
        }
        Ok(())
      }
    }
  }
}

impl IsCompatible for ContainerNameList<'_> {
  fn is_compatible(&self, _browsers: Browsers) -> bool {
    true
  }
}

define_shorthand! {
  /// A value for the [container](https://drafts.csswg.org/css-contain-3/#container-shorthand) shorthand property.
  pub struct Container<'i> {
    /// The container name.
    #[cfg_attr(feature = "serde", serde(borrow))]
    name: ContainerName(ContainerNameList<'i>),
    /// The container type.
    container_type: ContainerType(ContainerType),
  }
}

impl<'i> Parse<'i> for Container<'i> {
  fn parse<'t>(input: &mut Parser<'i, 't>) -> Result<Self, ParseError<'i, ParserError<'i>>> {
    let name = ContainerNameList::parse(input)?;
    let container_type = if input.try_parse(|input| input.expect_delim('/')).is_ok() {
      ContainerType::parse(input)?
    } else {
      ContainerType::default()
    };
    Ok(Container { name, container_type })
  }
}

impl<'i> ToCss for Container<'i> {
  fn to_css<W>(&self, dest: &mut Printer<W>) -> Result<(), PrinterError>
  where
    W: std::fmt::Write,
  {
    self.name.to_css(dest)?;
    if self.container_type != ContainerType::default() {
      dest.delim('/', true)?;
      self.container_type.to_css(dest)?;
    }
    Ok(())
  }
}

shorthand_handler!(ContainerHandler -> Container<'i> {
  name: ContainerName(ContainerNameList<'i>),
  container_type: ContainerType(ContainerType),
});

#[cfg(test)]
mod tests {
  use crate::printer::PrinterOptions;
  use crate::properties::{Property, PropertyId};
  use crate::stylesheet::ParserOptions;

  #[test]
  fn parses_size_and_scroll_state_container_types_in_any_order() {
    for (source, expected) in [
      ("size scroll-state", "size scroll-state"),
      ("scroll-state size", "size scroll-state"),
      ("inline-size scroll-state", "inline-size scroll-state"),
      ("scroll-state inline-size", "inline-size scroll-state"),
      ("scroll-state", "scroll-state"),
    ] {
      let property = Property::parse_string(PropertyId::from("container-type"), source, ParserOptions::default())
        .expect("container type should parse");
      assert_eq!(
        property
          .value_to_css_string(PrinterOptions::default())
          .expect("container type should serialize"),
        expected,
        "{source}",
      );
    }
  }

  #[test]
  fn rejects_conflicting_or_duplicate_container_types() {
    for source in [
      "normal scroll-state",
      "size inline-size",
      "size size",
      "scroll-state scroll-state",
    ] {
      let property = Property::parse_string(PropertyId::from("container-type"), source, ParserOptions::default())
        .expect("known declarations retain an unparsed fallback");
      assert!(matches!(property, Property::Unparsed(_)), "{source}");
    }
  }
}
