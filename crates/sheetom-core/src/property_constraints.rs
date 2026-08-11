use cssparser::{Parser, ParserInput, Token};
use lightningcss::{
    error::ParserError,
    properties::{
        animation::{
            AnimationDirection, AnimationDuration, AnimationFillMode, AnimationIterationCount,
            AnimationName, AnimationPlayState,
        },
        border::BorderSideWidth,
        Property,
    },
    stylesheet::PrinterOptions,
    traits::Parse,
    values::{
        color::CssColor,
        easing::EasingFunction,
        length::Length,
        position::{HorizontalPosition, VerticalPosition},
        time::Time,
    },
};

use crate::{syntax::split_top_level_delimiter, EngineError};

/// Validates browser parse-time constraints that are narrower than the
/// reusable value types in Lightning CSS.
///
/// The input has already been parsed into a typed property. Shorthands are
/// inspected through their typed longhands so a constraint is defined once at
/// the semantic state boundary rather than repeated in every shorthand codec.
pub(crate) fn validate_standard_property(
    authored_name: &str,
    authored_value: &str,
    property: &Property<'_>,
) -> Result<(), EngineError> {
    let canonical = property
        .value_to_css_string(PrinterOptions::default())
        .map_err(|error| EngineError::Serialize(error.to_string()))?;
    let canonical_name = authored_name
        .strip_prefix("-webkit-")
        .unwrap_or(authored_name);
    if canonical_name.eq_ignore_ascii_case("animation-duration")
        && !valid_duration_list(&canonical, true)
    {
        return invalid(authored_name, authored_value);
    }
    if canonical_name.eq_ignore_ascii_case("transition-duration")
        && !valid_duration_list(&canonical, false)
    {
        return invalid(authored_name, authored_value);
    }
    if authored_name.eq_ignore_ascii_case("-webkit-text-stroke")
        && !valid_webkit_text_stroke(authored_value)
    {
        return invalid(authored_name, authored_value);
    }
    validate_authored_property_capability(authored_name, &canonical)?;
    if matches!(
        property,
        Property::TransformOrigin(position, _)
            if has_side_offset(&position.x) && has_vertical_side_offset(&position.y)
    ) {
        return invalid(authored_name, authored_value);
    }
    if authored_name
        .strip_prefix("-webkit-")
        .unwrap_or(authored_name)
        .eq_ignore_ascii_case("animation")
        && !valid_chromium_animation(authored_value)
    {
        return invalid(authored_name, authored_value);
    }
    let has_direct_negative = has_direct_negative_component(authored_value);
    validate_longhand_candidate(authored_name, &canonical, has_direct_negative)?;

    let property_id = property.property_id();
    let Some(longhands) = property_id.longhands() else {
        return Ok(());
    };
    for longhand_id in longhands {
        let Some(longhand) = property.longhand(&longhand_id) else {
            continue;
        };
        let value = longhand
            .value_to_css_string(PrinterOptions::default())
            .map_err(|error| EngineError::Serialize(error.to_string()))?;
        validate_longhand_candidate(longhand_id.name(), &value, has_direct_negative)?;
    }
    Ok(())
}

fn validate_longhand_candidate(
    name: &str,
    value: &str,
    has_direct_negative: bool,
) -> Result<(), EngineError> {
    let canonical_name = name.strip_prefix("-webkit-").unwrap_or(name);

    if matches!(canonical_name, "appearance") && !valid_appearance(value) {
        return invalid(name, value);
    }
    if NON_NEGATIVE_LONGHANDS.contains(&canonical_name)
        && has_direct_negative
        && has_direct_negative_component(value)
    {
        return invalid(name, value);
    }
    if PADDING_LONGHANDS.contains(&canonical_name) && value.eq_ignore_ascii_case("auto") {
        return invalid(name, value);
    }
    if SCROLL_MARGIN_LONGHANDS.contains(&canonical_name) && !is_length(value) {
        return invalid(name, value);
    }
    if SIZE_LONGHANDS.contains(&canonical_name) && value.eq_ignore_ascii_case("contain") {
        return invalid(name, value);
    }
    if SIZE_LONGHANDS.contains(&canonical_name) && has_top_level_function(value, "fit-content") {
        return invalid(name, value);
    }
    if matches!(canonical_name, "align-content")
        && has_all_top_level_idents(value, &["last", "baseline"])
    {
        return invalid(name, value);
    }
    if matches!(canonical_name, "outline-style") && value.eq_ignore_ascii_case("hidden") {
        return invalid(name, value);
    }
    if matches!(canonical_name, "user-select") && value.eq_ignore_ascii_case("contain") {
        return invalid(name, value);
    }
    if matches!(canonical_name, "view-transition-name") && value.eq_ignore_ascii_case("auto") {
        return invalid(name, value);
    }
    if matches!(canonical_name, "z-index") && is_non_integer_number(value) {
        return invalid(name, value);
    }

    Ok(())
}

pub(crate) fn validate_authored_property_capability(
    name: &str,
    value: &str,
) -> Result<(), EngineError> {
    if valid_chromium_property_capability(name, value) {
        return Ok(());
    }
    invalid(name, value)
}

fn valid_chromium_property_capability(name: &str, value: &str) -> bool {
    if is_css_wide_keyword(value) {
        return true;
    }
    let canonical_name = name.strip_prefix("-webkit-").unwrap_or(name);

    if canonical_name.eq_ignore_ascii_case("text-transform")
        && has_any_top_level_ident(value, &["full-size-kana", "full-width"])
    {
        return false;
    }
    if canonical_name.eq_ignore_ascii_case("stroke-linejoin") && has_top_level_ident(value, "arcs")
    {
        return false;
    }
    if canonical_name.eq_ignore_ascii_case("display")
        && has_any_top_level_ident(
            value,
            &[
                "run-in",
                "ruby-base",
                "ruby-base-container",
                "ruby-text-container",
            ],
        )
    {
        return false;
    }
    if canonical_name.eq_ignore_ascii_case("text-align")
        && has_any_top_level_ident(value, &["justify-all", "match-parent"])
    {
        return false;
    }
    if canonical_name.eq_ignore_ascii_case("text-align-last")
        && has_top_level_ident(value, "match-parent")
    {
        return false;
    }
    if matches!(canonical_name, "list-style" | "list-style-type")
        && has_top_level_function(value, "symbols")
    {
        return false;
    }
    if matches!(canonical_name, "transition") && !valid_transition_list(value) {
        return false;
    }

    if name.eq_ignore_ascii_case("-webkit-column-break-before")
        || name.eq_ignore_ascii_case("-webkit-column-break-after")
    {
        return matches_ascii_case(value, &["auto", "always", "avoid"]);
    }
    if name.eq_ignore_ascii_case("-webkit-column-break-inside") {
        return matches_ascii_case(value, &["auto", "avoid"]);
    }
    if name.eq_ignore_ascii_case("-webkit-mask")
        && has_any_top_level_ident(
            value,
            &[
                "fill-box",
                "margin-box",
                "no-clip",
                "stroke-box",
                "view-box",
            ],
        )
    {
        return false;
    }
    if name.eq_ignore_ascii_case("mask") && has_top_level_ident(value, "margin-box") {
        return false;
    }
    if name.eq_ignore_ascii_case("-webkit-mask-clip")
        && has_any_top_level_ident(value, &["fill-box", "no-clip", "stroke-box", "view-box"])
    {
        return false;
    }
    if name.eq_ignore_ascii_case("-webkit-mask-origin")
        && has_any_top_level_ident(value, &["fill-box", "stroke-box", "view-box"])
    {
        return false;
    }
    if name.eq_ignore_ascii_case("-webkit-mask-composite")
        && has_any_top_level_ident(value, &["add", "exclude", "intersect", "subtract"])
    {
        return false;
    }

    true
}

fn valid_transition_list(source: &str) -> bool {
    let Some(entries) = split_top_level_delimiter(source, b',') else {
        return false;
    };
    entries.len() == 1
        || entries
            .iter()
            .all(|entry| !has_top_level_ident(entry, "none"))
}

fn is_css_wide_keyword(value: &str) -> bool {
    matches_ascii_case(
        value,
        &["inherit", "initial", "revert", "revert-layer", "unset"],
    )
}

fn matches_ascii_case(value: &str, candidates: &[&str]) -> bool {
    candidates
        .iter()
        .any(|candidate| value.eq_ignore_ascii_case(candidate))
}

fn has_all_top_level_idents(source: &str, expected: &[&str]) -> bool {
    expected
        .iter()
        .all(|expected| has_top_level_ident(source, expected))
}

fn has_any_top_level_ident(source: &str, expected: &[&str]) -> bool {
    expected
        .iter()
        .any(|expected| has_top_level_ident(source, expected))
}

fn has_top_level_ident(source: &str, expected: &str) -> bool {
    let mut input = ParserInput::new(source);
    let mut parser = Parser::new(&mut input);
    while let Ok(token) = parser.next() {
        if matches!(token, Token::Ident(value) if value.eq_ignore_ascii_case(expected)) {
            return true;
        }
    }
    false
}

fn has_top_level_function(source: &str, expected: &str) -> bool {
    let mut input = ParserInput::new(source);
    let mut parser = Parser::new(&mut input);
    while let Ok(token) = parser.next() {
        if matches!(token, Token::Function(value) if value.eq_ignore_ascii_case(expected)) {
            return true;
        }
    }
    false
}

fn invalid<T>(name: &str, value: &str) -> Result<T, EngineError> {
    Err(EngineError::Parse(format!(
        "property violates its Chromium parse-time constraints: {name}: {value}"
    )))
}

fn valid_appearance(value: &str) -> bool {
    matches!(
        value.to_ascii_lowercase().as_str(),
        "none"
            | "auto"
            | "base-select"
            | "textfield"
            | "menulist-button"
            | "button"
            | "checkbox"
            | "listbox"
            | "menulist"
            | "meter"
            | "progress-bar"
            | "push-button"
            | "radio"
            | "searchfield"
            | "slider-horizontal"
            | "square-button"
            | "textarea"
    )
}

fn valid_chromium_animation(source: &str) -> bool {
    let mut input = ParserInput::new(source);
    let mut parser = Parser::new(&mut input);
    parser
        .parse_entirely(|input| input.parse_comma_separated(parse_chromium_animation_item))
        .is_ok()
}

fn parse_chromium_animation_item<'i, 't>(
    input: &mut Parser<'i, 't>,
) -> Result<(), cssparser::ParseError<'i, ParserError<'i>>> {
    let mut duration = false;
    let mut timing_function = false;
    let mut delay = false;
    let mut iteration_count = false;
    let mut direction = false;
    let mut fill_mode = false;
    let mut play_state = false;
    let mut name = false;
    let mut consumed = false;

    macro_rules! parse_once {
        ($seen:ident, $parser:expr) => {
            if !$seen && input.try_parse($parser).is_ok() {
                $seen = true;
                consumed = true;
                continue;
            }
        };
    }

    loop {
        parse_once!(duration, parse_non_negative_animation_duration);
        parse_once!(timing_function, EasingFunction::parse);
        parse_once!(delay, Time::parse);
        parse_once!(iteration_count, AnimationIterationCount::parse);
        parse_once!(direction, AnimationDirection::parse);
        parse_once!(fill_mode, AnimationFillMode::parse);
        parse_once!(play_state, AnimationPlayState::parse);
        parse_once!(name, AnimationName::parse);
        break;
    }

    if consumed {
        Ok(())
    } else {
        Err(input.new_custom_error(ParserError::InvalidValue))
    }
}

fn parse_non_negative_animation_duration<'i, 't>(
    input: &mut Parser<'i, 't>,
) -> Result<(), cssparser::ParseError<'i, ParserError<'i>>> {
    let duration = AnimationDuration::parse(input)?;
    if matches!(duration, AnimationDuration::Time(ref time) if time.to_ms() < 0.0) {
        return Err(input.new_custom_error(ParserError::InvalidValue));
    }
    Ok(())
}

fn valid_duration_list(source: &str, allow_auto: bool) -> bool {
    let mut input = ParserInput::new(source);
    let mut parser = Parser::new(&mut input);
    parser
        .parse_entirely(|input| {
            input.parse_comma_separated(|input| {
                if allow_auto
                    && input
                        .try_parse(|input| input.expect_ident_matching("auto"))
                        .is_ok()
                {
                    return Ok(());
                }
                let time = Time::parse(input)?;
                if time.to_ms() < 0.0 {
                    return Err(input.new_custom_error(ParserError::InvalidValue));
                }
                Ok(())
            })
        })
        .is_ok()
}

pub(crate) fn has_direct_negative_component(source: &str) -> bool {
    let mut input = ParserInput::new(source);
    let mut parser = Parser::new(&mut input);
    while let Ok(token) = parser.next_including_whitespace_and_comments() {
        let negative = match token {
            Token::Number { value, .. }
            | Token::Percentage {
                unit_value: value, ..
            }
            | Token::Dimension { value, .. } => *value < 0.0,
            _ => false,
        };
        if negative {
            return true;
        }
    }
    false
}

pub(crate) fn rejects_direct_negative_component(name: &str) -> bool {
    let canonical_name = name.strip_prefix("-webkit-").unwrap_or(name);
    NON_NEGATIVE_LONGHANDS.contains(&canonical_name)
}

fn valid_webkit_text_stroke(source: &str) -> bool {
    let mut input = ParserInput::new(source);
    let mut parser = Parser::new(&mut input);
    parser
        .parse_entirely(|input| {
            let mut width = false;
            let mut color = false;
            loop {
                if !width
                    && input
                        .try_parse(|input| BorderSideWidth::parse(input))
                        .is_ok()
                {
                    width = true;
                    continue;
                }
                if !color && input.try_parse(CssColor::parse).is_ok() {
                    color = true;
                    continue;
                }
                break;
            }
            if width || color {
                return Ok(());
            }
            Err(input.new_error_for_next_token::<()>())
        })
        .is_ok()
}

fn has_side_offset(position: &HorizontalPosition) -> bool {
    matches!(
        position,
        HorizontalPosition::Side {
            offset: Some(_),
            ..
        }
    )
}

fn has_vertical_side_offset(position: &VerticalPosition) -> bool {
    matches!(
        position,
        VerticalPosition::Side {
            offset: Some(_),
            ..
        }
    )
}

fn is_length(source: &str) -> bool {
    let mut input = ParserInput::new(source);
    let mut parser = Parser::new(&mut input);
    Length::parse(&mut parser).is_ok() && parser.expect_exhausted().is_ok()
}

fn is_non_integer_number(source: &str) -> bool {
    let mut input = ParserInput::new(source);
    let mut parser = Parser::new(&mut input);
    let Ok(token) = parser.next() else {
        return false;
    };
    let Token::Number { int_value, .. } = token else {
        return false;
    };
    int_value.is_none() && parser.expect_exhausted().is_ok()
}

const PADDING_LONGHANDS: &[&str] = &[
    "padding-block-end",
    "padding-block-start",
    "padding-bottom",
    "padding-inline-end",
    "padding-inline-start",
    "padding-left",
    "padding-right",
    "padding-top",
];

const SCROLL_MARGIN_LONGHANDS: &[&str] = &[
    "scroll-margin-block-end",
    "scroll-margin-block-start",
    "scroll-margin-bottom",
    "scroll-margin-inline-end",
    "scroll-margin-inline-start",
    "scroll-margin-left",
    "scroll-margin-right",
    "scroll-margin-top",
];

const SIZE_LONGHANDS: &[&str] = &[
    "block-size",
    "height",
    "inline-size",
    "logical-height",
    "logical-width",
    "max-block-size",
    "max-height",
    "max-inline-size",
    "max-logical-height",
    "max-logical-width",
    "max-width",
    "min-block-size",
    "min-height",
    "min-inline-size",
    "min-logical-height",
    "min-logical-width",
    "min-width",
    "width",
];

const NON_NEGATIVE_LONGHANDS: &[&str] = &[
    "animation-iteration-count",
    "aspect-ratio",
    "background-size",
    "block-size",
    "border-block-end-width",
    "border-block-start-width",
    "border-bottom-left-radius",
    "border-bottom-right-radius",
    "border-bottom-width",
    "border-end-end-radius",
    "border-end-start-radius",
    "border-image-outset",
    "border-image-slice",
    "border-image-width",
    "border-inline-end-width",
    "border-inline-start-width",
    "border-left-width",
    "border-right-width",
    "border-spacing",
    "border-start-end-radius",
    "border-start-start-radius",
    "border-top-left-radius",
    "border-top-right-radius",
    "border-top-width",
    "column-gap",
    "column-rule-width",
    "flex-basis",
    "flex-grow",
    "flex-shrink",
    "font-size",
    "grid-auto-columns",
    "grid-auto-rows",
    "grid-template-columns",
    "grid-template-rows",
    "height",
    "inline-size",
    "mask-border-outset",
    "mask-border-slice",
    "mask-border-width",
    "mask-size",
    "max-block-size",
    "max-height",
    "max-inline-size",
    "max-width",
    "min-block-size",
    "min-height",
    "min-inline-size",
    "min-width",
    "outline-width",
    "padding-block-end",
    "padding-block-start",
    "padding-bottom",
    "padding-inline-end",
    "padding-inline-start",
    "padding-left",
    "padding-right",
    "padding-top",
    "perspective",
    "row-gap",
    "row-rule-width",
    "rule-width",
    "scroll-padding-block-end",
    "scroll-padding-block-start",
    "scroll-padding-bottom",
    "scroll-padding-inline-end",
    "scroll-padding-inline-start",
    "scroll-padding-left",
    "scroll-padding-right",
    "scroll-padding-top",
    "shape-margin",
    "text-stroke-width",
    "width",
];

#[cfg(test)]
mod tests {
    use super::*;
    use lightningcss::{properties::PropertyId, stylesheet::ParserOptions, traits::IntoOwned};

    fn parsed(name: &str, value: &str) -> Property<'static> {
        Property::parse_string(PropertyId::from(name), value, ParserOptions::default())
            .unwrap()
            .into_owned()
    }

    #[test]
    fn rejects_direct_negative_components_but_not_deferred_calculations() {
        assert!(!has_direct_negative_component("calc(-10px)"));
        assert!(!has_direct_negative_component("rem(-5px, 2px)"));
        assert!(validate_standard_property("width", "-10px", &parsed("width", "-10px")).is_err());
        assert!(validate_standard_property(
            "width",
            "calc(-10px)",
            &parsed("width", "calc(-10px)")
        )
        .is_ok());
        assert!(validate_standard_property(
            "width",
            "rem(-5px, 2px)",
            &parsed("width", "rem(-5px, 2px)")
        )
        .is_ok());
        assert!(validate_standard_property(
            "border",
            "-1px solid",
            &parsed("border", "-1px solid")
        )
        .is_err());
    }

    #[test]
    fn enforces_property_specific_domains() {
        for (name, value) in [
            ("appearance", "red"),
            ("animation-duration", "-1s"),
            ("padding", "auto"),
            ("scroll-margin", "10%"),
            ("width", "contain"),
            ("outline", "hidden"),
            ("transition-duration", "-1s"),
            ("z-index", "1.5"),
        ] {
            assert!(
                validate_standard_property(name, value, &parsed(name, value)).is_err(),
                "{name}: {value}"
            );
        }
        assert!(valid_appearance("base-select"));
        assert!(!valid_appearance("base"));
    }

    #[test]
    fn enforces_pinned_chromium_capability_boundaries() {
        for (name, value) in [
            ("width", "fit-content(10px)"),
            ("-webkit-logical-width", "fit-content(10px)"),
            ("text-transform", "capitalize full-width"),
            ("stroke-linejoin", "arcs"),
            ("display", "run-in flow"),
            ("align-content", "last baseline"),
            ("list-style-type", "symbols(cyclic \"x\")"),
            ("transition", "none, none"),
            ("-webkit-column-break-before", "page"),
            ("-webkit-column-break-inside", "avoid-page"),
            ("mask", "margin-box"),
            ("-webkit-mask", "view-box"),
            ("-webkit-mask-clip", "no-clip"),
            ("-webkit-mask-origin", "fill-box"),
            ("-webkit-mask-composite", "add"),
        ] {
            assert!(
                validate_standard_property(name, value, &parsed(name, value)).is_err(),
                "{name}: {value}"
            );
        }

        for (name, value) in [
            ("width", "fit-content"),
            ("text-transform", "math-auto"),
            ("display", "ruby-text"),
            ("align-content", "baseline"),
            ("list-style-type", "sheetom-ident"),
            ("transition", "none"),
            ("transition", "opacity 1s, transform 2s"),
            ("-webkit-column-break-before", "always"),
            ("-webkit-column-break-inside", "avoid"),
            ("-webkit-mask", "content-box"),
            ("-webkit-mask-clip", "content-box"),
            ("-webkit-mask-origin", "padding-box"),
            ("-webkit-mask-composite", "source-over"),
        ] {
            assert!(
                validate_standard_property(name, value, &parsed(name, value)).is_ok(),
                "{name}: {value}"
            );
        }
    }

    #[test]
    fn enforces_property_specific_compound_grammars() {
        for (name, value) in [
            ("-webkit-text-stroke", "none"),
            ("-webkit-text-stroke", "2px dashed red"),
            ("transform-origin", "left 10px top 20px"),
        ] {
            assert!(
                validate_standard_property(name, value, &parsed(name, value)).is_err(),
                "{name}: {value}"
            );
        }
        for value in ["red", "2px", "red 2px", "calc(1px + 2px) red"] {
            assert!(
                validate_standard_property(
                    "-webkit-text-stroke",
                    value,
                    &parsed("-webkit-text-stroke", value)
                )
                .is_ok(),
                "{value}"
            );
        }
        for value in ["left top 20px", "right 10px bottom"] {
            assert!(
                validate_standard_property(
                    "transform-origin",
                    value,
                    &parsed("transform-origin", value)
                )
                .is_ok(),
                "{value}"
            );
        }
        assert!(validate_standard_property(
            "animation",
            "auto ease 1s foo",
            &parsed("animation", "auto ease 1s foo")
        )
        .is_ok());
        assert!(
            validate_standard_property("animation", "-1s", &parsed("animation", "-1s")).is_ok()
        );
        assert!(validate_standard_property(
            "transition",
            "none 1s linear -1s normal",
            &parsed("transition", "none 1s linear -1s normal")
        )
        .is_ok());
        assert!(validate_standard_property(
            "animation",
            "1s foo scroll(root)",
            &parsed("animation", "1s foo scroll(root)")
        )
        .is_err());
    }
}
