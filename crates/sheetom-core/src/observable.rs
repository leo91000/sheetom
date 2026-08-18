use crate::catalog::shorthand_longhands;
use crate::recovered_value::RecoveredObservableText;
use crate::syntax::split_top_level_delimiter;
use crate::{
    parse_semantic_property, EngineError, PropertyParseKind, SemanticDeclaration,
    SemanticExtensionValue, SemanticPropertyValue,
};
use cssparser::{Parser, ParserInput, Token, TokenizerWithSpans};
use lightningcss::{
    properties::{Property, PropertyId},
    stylesheet::{ParserOptions, PrinterOptions},
};

#[derive(Clone, Copy)]
enum ObservableCategory {
    Typed,
    PendingSubstitution,
    Custom,
}

pub(crate) struct DeclarationProjection {
    pub(crate) canonical: String,
    pub(crate) observable: String,
}

pub(crate) fn project_declaration(
    declaration: &SemanticDeclaration,
) -> Result<DeclarationProjection, EngineError> {
    let name = declaration.property_name();
    let input = trim_css_whitespace(declaration.recovered().source());
    let mut canonical = declaration.canonical_value()?;
    if starts_math_function(input)
        && !starts_math_function(&canonical)
        && crate::syntax::split_top_level_whitespace(input)
            .is_some_and(|components| components.len() == 1)
        && crate::syntax::split_top_level_whitespace(&canonical)
            .is_some_and(|components| components.len() == 1)
        && crate::property_constraints::rejects_direct_negative_component(name)
        && crate::property_constraints::has_direct_negative_component(&canonical)
    {
        canonical = format!("calc({canonical})");
    }
    let category = match declaration.value() {
        SemanticPropertyValue::Standard(_)
        | SemanticPropertyValue::Extension(_)
        | SemanticPropertyValue::ExpandedShorthand
        | SemanticPropertyValue::FontFaceDescriptor(_) => ObservableCategory::Typed,
        SemanticPropertyValue::PendingSubstitution(_) => ObservableCategory::PendingSubstitution,
        SemanticPropertyValue::CustomTokenStream => ObservableCategory::Custom,
    };
    let preserve_comments = matches!(
        category,
        ObservableCategory::PendingSubstitution | ObservableCategory::Custom
    );
    let recovered = declaration.recovered().observable_text(preserve_comments)?;
    let retained = if preserve_comments {
        trim_token_stream_trivia(&recovered.retained)
    } else {
        trim_css_whitespace(&recovered.retained)
    };
    let closed = trim_css_whitespace(&recovered.closed);
    let observable = if matches!(
        declaration.value(),
        SemanticPropertyValue::FontFaceDescriptor(_)
    ) && name == "font-variant"
    {
        String::new()
    } else if !matches!(category, ObservableCategory::Typed) {
        if matches!(category, ObservableCategory::Custom) && recovered.unterminated_url {
            closed.to_owned()
        } else {
            retained.to_owned()
        }
    } else if let SemanticPropertyValue::Extension(
        SemanticExtensionValue::CrossDimensionCalculation(value),
    ) = declaration.value()
    {
        value.list_observable_value(input).unwrap_or_else(|| {
            serialize_typed_observable(name, input, closed, &canonical, &recovered)
        })
    } else if let SemanticPropertyValue::Extension(SemanticExtensionValue::WebkitPerspective(
        value,
    )) = declaration.value()
    {
        value.observable_value()?
    } else if matches!(
        declaration.value(),
        SemanticPropertyValue::Extension(SemanticExtensionValue::WebkitBorderImage(_))
    ) {
        if declaration.recovered().contains_context_dependent_sign() {
            canonical.clone()
        } else {
            serialize_webkit_border_image_observable(input, &canonical)
        }
    } else if matches!(
        declaration.value(),
        SemanticPropertyValue::Extension(SemanticExtensionValue::WebkitMaskBoxImageComponent(_))
    ) {
        if name == "-webkit-mask-box-image-slice" {
            serialize_webkit_mask_box_image_slice_observable(input, &canonical)
        } else {
            canonical.clone()
        }
    } else if let SemanticPropertyValue::Extension(SemanticExtensionValue::Geometric(value)) =
        declaration.value()
    {
        if let Some(gradient) = value.gradient_observable_value()? {
            serialize_gradient_observable(&gradient)
        } else if let Some(observable) = value.image_set_observable_value()? {
            observable
        } else {
            canonical.clone()
        }
    } else if let SemanticPropertyValue::Extension(SemanticExtensionValue::GapRuleLonghand(value)) =
        declaration.value()
    {
        value.observable_value()?
    } else {
        serialize_typed_observable(name, input, closed, &canonical, &recovered)
    };

    Ok(DeclarationProjection {
        canonical,
        observable,
    })
}

pub(crate) fn project_observable_value(name: &str, source: &str) -> Option<String> {
    let declaration = parse_semantic_property(name, source).ok()?;
    project_declaration(&declaration)
        .ok()
        .map(|projection| projection.observable)
}

fn serialize_gradient_observable(input: &str) -> String {
    let mut value = canonicalize_unquoted_urls(input);
    value = replace_gradient_color_tokens(&value);
    value = replace_comments_with_space(&value);
    value = normalize_comma_whitespace(&value);
    value = canonicalize_leading_decimal(&value);
    value = canonicalize_color_identifiers(&value);
    if value
        .get(..value.find('(').unwrap_or(value.len()))
        .is_some_and(|name| name.eq_ignore_ascii_case("-webkit-gradient"))
    {
        value = normalize_webkit_gradient_points(&value);
    }
    if let Some(open) = value.find('(') {
        value[..open].make_ascii_lowercase();
    }
    value
}

fn canonicalize_unquoted_urls(input: &str) -> String {
    let mut tokenizer = TokenizerWithSpans::new(input);
    let mut replacements = Vec::new();
    while let Ok(token) = tokenizer.next_token() {
        let Token::UnquotedUrl(value) = token.token else {
            continue;
        };
        let escaped = value.replace('\\', "\\\\").replace('"', "\\\"");
        replacements.push((
            token.start.byte_index(),
            token.end.byte_index(),
            format!("url(\"{escaped}\")"),
        ));
    }
    let mut output = input.to_owned();
    for (start, end, replacement) in replacements.into_iter().rev() {
        output.replace_range(start..end, &replacement);
    }
    output
}

fn normalize_webkit_gradient_points(input: &str) -> String {
    let mut tokenizer = TokenizerWithSpans::new(input);
    let mut replacements = Vec::new();
    while let Ok(token) = tokenizer.next_token() {
        let Token::Ident(identifier) = token.token else {
            continue;
        };
        let replacement =
            if identifier.eq_ignore_ascii_case("left") || identifier.eq_ignore_ascii_case("top") {
                "0%"
            } else if identifier.eq_ignore_ascii_case("right")
                || identifier.eq_ignore_ascii_case("bottom")
            {
                "100%"
            } else if identifier.eq_ignore_ascii_case("center") {
                "50%"
            } else {
                continue;
            };
        replacements.push((
            token.start.byte_index(),
            token.end.byte_index(),
            replacement,
        ));
    }

    let mut output = input.to_owned();
    for (start, end, replacement) in replacements.into_iter().rev() {
        output.replace_range(start..end, replacement);
    }
    output
}

fn replace_gradient_color_tokens(input: &str) -> String {
    #[derive(Clone, Copy)]
    enum Opening {
        Parenthesis { color_start: Option<usize> },
        Square,
        Curly,
    }

    let mut tokenizer = TokenizerWithSpans::new(input);
    let mut openings = Vec::new();
    let mut replacements = Vec::<(usize, usize, String)>::new();
    while let Ok(token) = tokenizer.next_token() {
        match token.token {
            Token::Function(name) => openings.push(Opening::Parenthesis {
                color_start: is_serializable_color_function(&name)
                    .then_some(token.start.byte_index()),
            }),
            Token::ParenthesisBlock => openings.push(Opening::Parenthesis { color_start: None }),
            Token::SquareBracketBlock => openings.push(Opening::Square),
            Token::CurlyBracketBlock => openings.push(Opening::Curly),
            Token::CloseParenthesis => {
                let Some(Opening::Parenthesis { color_start }) = openings.pop() else {
                    continue;
                };
                let Some(start) = color_start else {
                    continue;
                };
                let end = token.end.byte_index();
                let Some(source) = input.get(start..end) else {
                    continue;
                };
                if let Some(color) = canonicalize_nested_color(source) {
                    replacements.push((start, end, color));
                }
            }
            Token::CloseSquareBracket => {
                if matches!(openings.last(), Some(Opening::Square)) {
                    openings.pop();
                }
            }
            Token::CloseCurlyBracket => {
                if matches!(openings.last(), Some(Opening::Curly)) {
                    openings.pop();
                }
            }
            Token::Hash(value) | Token::IDHash(value) => {
                let source = format!("#{value}");
                if let Some(color) = serialize_hex_color(&source) {
                    replacements.push((token.start.byte_index(), token.end.byte_index(), color));
                }
            }
            _ => {}
        }
    }

    replacements.sort_unstable_by_key(|(start, _, _)| *start);
    let mut outermost = Vec::with_capacity(replacements.len());
    let mut covered_until = 0usize;
    for replacement in replacements {
        if replacement.0 < covered_until {
            continue;
        }
        covered_until = replacement.1;
        outermost.push(replacement);
    }
    let mut output = input.to_owned();
    for (start, end, replacement) in outermost.into_iter().rev() {
        output.replace_range(start..end, &replacement);
    }
    output
}

fn is_serializable_color_function(name: &str) -> bool {
    [
        "rgb", "rgba", "hsl", "hsla", "hwb", "lab", "lch", "oklab", "oklch", "color",
    ]
    .iter()
    .any(|candidate| name.eq_ignore_ascii_case(candidate))
}

fn canonicalize_nested_color(source: &str) -> Option<String> {
    let property =
        Property::parse_string(PropertyId::Color, source, ParserOptions::default()).ok()?;
    let safe = property
        .value_to_css_string(PrinterOptions::default())
        .ok()?;
    Some(serialize_color(source, &safe))
}

fn replace_comments_with_space(input: &str) -> String {
    let mut tokenizer = TokenizerWithSpans::new(input);
    let mut comments = Vec::new();
    while let Ok(token) = tokenizer.next_token() {
        if matches!(token.token, Token::Comment(_)) {
            comments.push((token.start.byte_index(), token.end.byte_index()));
        }
    }
    let mut output = input.to_owned();
    for (start, end) in comments.into_iter().rev() {
        output.replace_range(start..end, " ");
    }
    output
}

fn normalize_comma_whitespace(source: &str) -> String {
    let mut output = String::with_capacity(source.len() + 4);
    let mut characters = source.chars().peekable();
    let mut quote = None;
    let mut escaped = false;
    while let Some(character) = characters.next() {
        if let Some(delimiter) = quote {
            output.push(character);
            if escaped {
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == delimiter {
                quote = None;
            }
            continue;
        }
        if matches!(character, '\'' | '"') {
            quote = Some(character);
            output.push(character);
            continue;
        }
        if character != ',' {
            output.push(character);
            continue;
        }
        while output.chars().last().is_some_and(char::is_whitespace) {
            output.pop();
        }
        output.push_str(", ");
        while characters.next_if(|value| value.is_whitespace()).is_some() {}
    }
    output
}

fn serialize_typed_observable(
    name: &str,
    input: &str,
    closed: &str,
    canonical: &str,
    recovered: &RecoveredObservableText,
) -> String {
    match name {
        "font-family" => {
            return serialize_font_family(
                input,
                canonical,
                recovered.single_string.as_deref(),
                recovered.recovered,
            );
        }
        "text-emphasis-style" => return serialize_text_emphasis_style(input, canonical),
        "text-emphasis-position" => return serialize_text_emphasis_position(input, canonical),
        "view-timeline-inset" | "scroll-snap-align" => {
            if let Some(value) = serialize_compressible_pair(name, input, recovered) {
                return value;
            }
        }
        "overflow-clip-margin" => return serialize_overflow_clip_margin(canonical),
        "font-style" => return serialize_font_style(input, canonical),
        "text-shadow" => return serialize_text_shadow(input, canonical),
        _ => {}
    }
    if is_single_color_property(name) {
        return serialize_color(closed, canonical);
    }
    if let Some(value) = serialize_plain_time_list(input) {
        return value;
    }
    if let Some(value) = serialize_zero_percentage_as_number(name, input, canonical) {
        return value;
    }
    if let Some(value) = serialize_explicit_zero_dimension(name, input, canonical) {
        return value;
    }
    if let Some(value) = serialize_dimensionless_zero(name, input) {
        return value;
    }
    match name {
        "transform-origin" => return serialize_transform_origin(input, canonical),
        "border-image-slice" => {
            return serialize_border_image_slice_observable(input, canonical);
        }
        "scrollbar-color" => {
            return serialize_color_pair(input).unwrap_or_else(|| canonical.to_owned());
        }
        "aspect-ratio" => return serialize_aspect_ratio(input, canonical),
        "cursor" => return serialize_cursor_observable(canonical),
        _ if is_position_pair_property(name) => {
            return serialize_position_pair(input, canonical);
        }
        _ => {}
    }
    if shorthand_longhands(name).is_some_and(|longhands| longhands.len() > 1)
        && !recovered.recovered
    {
        return serialize_shorthand_observable(name, input, canonical);
    }
    match name {
        "z-index" => {
            if let Some(value) = serialize_integer_calculation(closed) {
                return value;
            }
        }
        "object-position" => {
            if starts_math_function(input) && !starts_math_function(canonical) {
                if let Some((first, rest)) = canonical.split_once(' ') {
                    return format!("calc({first}) {rest}");
                }
            }
            return canonicalize_leading_decimal(canonical);
        }
        _ => {}
    }
    if starts_math_function(closed) {
        let value = canonicalize_leading_decimal(canonical);
        if starts_math_function(&value) {
            return value;
        }
        return format!("calc({value})");
    }
    if starts_image_set_function(closed) {
        return serialize_gradient_observable(closed);
    }
    if closed.contains("gradient(") {
        return closed.to_owned();
    }
    serialize_default_observable(input, closed, canonical, recovered)
}

fn serialize_cursor_observable(canonical: &str) -> String {
    let Some(layers) = split_top_level_delimiter(canonical, b',') else {
        return canonical.to_owned();
    };
    let mut serialized = String::new();
    for layer in layers {
        let layer = serialize_cursor_layer(layer);
        push_delimited(&mut serialized, ", ", &layer);
    }
    serialized
}

fn serialize_cursor_layer(layer: &str) -> String {
    let Some(mut components) = crate::syntax::split_top_level_whitespace(layer) else {
        return layer.to_owned();
    };
    if components.len() < 3 {
        return layer.to_owned();
    }
    let x_index = components.len() - 2;
    let y_index = components.len() - 1;
    let (Ok(x), Ok(y)) = (
        components[x_index].parse::<f64>(),
        components[y_index].parse::<f64>(),
    ) else {
        return layer.to_owned();
    };
    if !x.is_finite() || !y.is_finite() {
        return layer.to_owned();
    }
    let x = serialize_finite_number(x.trunc());
    let y = serialize_finite_number(y.trunc());
    components[x_index] = &x;
    components[y_index] = &y;
    components.join(" ")
}

fn serialize_compressible_pair(
    name: &str,
    input: &str,
    recovered: &RecoveredObservableText,
) -> Option<String> {
    let components = crate::syntax::split_top_level_whitespace(input)?;
    let [first, second] = components.as_slice() else {
        return None;
    };
    let project = |component: &str| {
        let declaration = parse_semantic_property(name, component).ok()?;
        let canonical = declaration.canonical_value().ok()?;
        Some(serialize_typed_observable(
            name, component, component, &canonical, recovered,
        ))
    };
    let first = project(first)?;
    let second = project(second)?;
    Some(if first == second {
        first
    } else {
        format!("{first} {second}")
    })
}

fn serialize_overflow_clip_margin(canonical: &str) -> String {
    let Some(components) = crate::syntax::split_top_level_whitespace(canonical) else {
        return canonical.to_owned();
    };
    match components.as_slice() {
        [visual_box, zero] if is_zero_dimension(zero) => (*visual_box).to_owned(),
        _ => canonical.to_owned(),
    }
}

fn serialize_font_style(input: &str, canonical: &str) -> String {
    let Some(components) = crate::syntax::split_top_level_whitespace(input) else {
        return canonical.to_owned();
    };
    if matches!(components.as_slice(), [keyword, angle] if keyword.eq_ignore_ascii_case("oblique") && is_zero_angle(angle))
    {
        return "normal".to_owned();
    }
    canonical.to_owned()
}

fn is_zero_angle(value: &str) -> bool {
    let unit_start = value
        .find(|character: char| character.is_ascii_alphabetic())
        .unwrap_or(value.len());
    let (number, unit) = value.split_at(unit_start);
    number.parse::<f64>().is_ok_and(|number| number == 0.0)
        && ["deg", "grad", "rad", "turn"]
            .iter()
            .any(|candidate| unit.eq_ignore_ascii_case(candidate))
}

fn serialize_text_shadow(input: &str, canonical: &str) -> String {
    let Some(layers) = split_top_level_delimiter(input, b',') else {
        return canonical.to_owned();
    };
    let mut serialized = String::new();
    for layer in layers {
        let Some(layer) = serialize_text_shadow_layer(layer) else {
            return canonical.to_owned();
        };
        push_delimited(&mut serialized, ", ", &layer);
    }
    serialized
}

fn serialize_text_shadow_layer(layer: &str) -> Option<String> {
    let components = crate::syntax::split_top_level_whitespace(layer)?;
    let color_index = components
        .iter()
        .position(|component| semantic_accepts("color", component));
    let mut serialized = String::new();
    if let Some(index) = color_index {
        let color = project_observable_value("color", components[index])
            .unwrap_or_else(|| components[index].to_owned());
        push_delimited(&mut serialized, " ", &color);
    }
    for (index, component) in components.into_iter().enumerate() {
        if Some(index) == color_index {
            continue;
        }
        let component = project_observable_value("margin-left", component)
            .unwrap_or_else(|| component.to_owned());
        push_delimited(&mut serialized, " ", &component);
    }
    Some(serialized)
}

fn serialize_text_emphasis_style(input: &str, canonical: &str) -> String {
    let Some(components) = crate::syntax::split_top_level_whitespace(input) else {
        return canonical.to_owned();
    };
    let fill = components.iter().find_map(|component| {
        if component.eq_ignore_ascii_case("filled") {
            Some("filled")
        } else if component.eq_ignore_ascii_case("open") {
            Some("open")
        } else {
            None
        }
    });
    let shape = components.iter().find_map(|component| {
        ["dot", "circle", "double-circle", "triangle", "sesame"]
            .into_iter()
            .find(|shape| component.eq_ignore_ascii_case(shape))
    });
    match (fill, shape) {
        (Some(fill), Some(shape)) => format!("{fill} {shape}"),
        (Some(fill), None) => fill.to_owned(),
        _ => canonical.to_owned(),
    }
}

fn serialize_text_emphasis_position(input: &str, canonical: &str) -> String {
    let Some(components) = crate::syntax::split_top_level_whitespace(input) else {
        return canonical.to_owned();
    };
    let vertical = components.iter().find_map(|component| {
        ["over", "under"]
            .into_iter()
            .find(|value| component.eq_ignore_ascii_case(value))
    });
    let horizontal = components.iter().find_map(|component| {
        ["left", "right"]
            .into_iter()
            .find(|value| component.eq_ignore_ascii_case(value))
    });
    match (vertical, horizontal) {
        (Some(vertical), Some(horizontal)) => format!("{vertical} {horizontal}"),
        _ => canonical.to_owned(),
    }
}

fn is_position_pair_property(name: &str) -> bool {
    matches!(
        name,
        "background-position"
            | "mask-position"
            | "offset-anchor"
            | "offset-position"
            | "perspective-origin"
            | "transform-origin"
    )
}

fn serialize_position_pair(input: &str, canonical: &str) -> String {
    let components = crate::syntax::split_top_level_whitespace(input).unwrap_or_default();
    if components.len() != 1 {
        return canonicalize_leading_decimal(input);
    }
    let component = components[0];
    if matches!(component, "auto" | "normal") {
        return canonical.to_owned();
    }
    if component.eq_ignore_ascii_case("center") {
        return "center center".to_owned();
    }
    if component.eq_ignore_ascii_case("top") || component.eq_ignore_ascii_case("bottom") {
        return format!("center {}", component.to_ascii_lowercase());
    }
    if component.eq_ignore_ascii_case("left") || component.eq_ignore_ascii_case("right") {
        return format!("{} center", component.to_ascii_lowercase());
    }
    let canonical_components =
        crate::syntax::split_top_level_whitespace(canonical).unwrap_or_else(|| vec![canonical]);
    let canonical_first = canonical_components.first().copied().unwrap_or(canonical);
    let canonical_second = canonical_components.get(1).copied().unwrap_or("center");
    let first = if starts_math_function(component) && !starts_math_function(canonical_first) {
        format!("calc({canonical_first})")
    } else {
        canonicalize_leading_decimal(canonical_first)
    };
    format!("{first} {canonical_second}")
}

fn serialize_transform_origin(input: &str, canonical: &str) -> String {
    let authored = crate::syntax::split_top_level_whitespace(input).unwrap_or_default();
    let canonical =
        crate::syntax::split_top_level_whitespace(canonical).unwrap_or_else(|| vec![canonical]);
    let mut output = String::new();

    for (index, component) in canonical.into_iter().enumerate() {
        let authored_component = authored.get(index).copied().unwrap_or_default();
        let component = if component == "0" {
            "0px".to_owned()
        } else {
            canonicalize_leading_decimal(component)
        };
        if starts_math_function(authored_component) && !starts_math_function(&component) {
            output.reserve(usize::from(!output.is_empty()) + "calc()".len() + component.len());
            if !output.is_empty() {
                output.push(' ');
            }
            output.push_str("calc(");
            output.push_str(&component);
            output.push(')');
        } else {
            push_delimited(&mut output, " ", &component);
        }
    }

    output
}

fn serialize_border_image_slice_observable(input: &str, canonical: &str) -> String {
    let Some(authored) = border_image_slice_components(input) else {
        return canonical.to_owned();
    };
    let Some(canonical_components) = border_image_slice_components(canonical) else {
        return canonical.to_owned();
    };
    let Some(authored) = expand_four_components(&authored) else {
        return canonical.to_owned();
    };
    let Some(canonical_components) = expand_four_components(&canonical_components) else {
        return canonical.to_owned();
    };
    let projected = authored
        .into_iter()
        .zip(canonical_components)
        .map(|(authored, canonical)| {
            if starts_math_function(authored) && !starts_math_function(canonical) {
                format!("calc({canonical})")
            } else {
                canonicalize_leading_decimal(canonical)
            }
        })
        .collect::<Vec<_>>();
    let mut observable = compress_four_components(&projected);
    if crate::syntax::split_top_level_whitespace(canonical).is_some_and(|components| {
        components
            .iter()
            .any(|value| value.eq_ignore_ascii_case("fill"))
    }) {
        observable.push_str(" fill");
    }
    observable
}

fn border_image_slice_components(value: &str) -> Option<Vec<&str>> {
    let components = crate::syntax::split_top_level_whitespace(value)?;
    let values = components
        .into_iter()
        .filter(|component| !component.eq_ignore_ascii_case("fill"))
        .collect::<Vec<_>>();
    (1..=4).contains(&values.len()).then_some(values)
}

fn expand_four_components<'a>(values: &[&'a str]) -> Option<[&'a str; 4]> {
    match values {
        [first] => Some([first, first, first, first]),
        [first, second] => Some([first, second, first, second]),
        [first, second, third] => Some([first, second, third, second]),
        [first, second, third, fourth] => Some([first, second, third, fourth]),
        _ => None,
    }
}

fn compress_four_components(values: &[String]) -> String {
    if values[0] == values[1] && values[0] == values[2] && values[0] == values[3] {
        return values[0].clone();
    }
    if values[0] == values[2] && values[1] == values[3] {
        return format!("{} {}", values[0], values[1]);
    }
    if values[1] == values[3] {
        return format!("{} {} {}", values[0], values[1], values[2]);
    }
    values.join(" ")
}

fn serialize_color_pair(input: &str) -> Option<String> {
    let components = crate::syntax::split_top_level_whitespace(input)?;
    let [first, second] = components.as_slice() else {
        return None;
    };
    Some(format!(
        "{} {}",
        project_observable_value("color", first)?,
        project_observable_value("color", second)?
    ))
}

fn serialize_aspect_ratio(input: &str, canonical: &str) -> String {
    if input.eq_ignore_ascii_case("auto") || input.contains('/') || canonical.contains('/') {
        return canonicalize_leading_decimal(canonical);
    }
    let ratio = if starts_math_function(input) && !starts_math_function(canonical) {
        format!("calc({canonical})")
    } else {
        canonicalize_leading_decimal(canonical)
    };
    format!("{ratio} / 1")
}

fn starts_image_set_function(value: &str) -> bool {
    starts_with_ignore_ascii_case(value, "image-set(")
        || starts_with_ignore_ascii_case(value, "-webkit-image-set(")
}

fn serialize_shorthand_observable(name: &str, input: &str, canonical: &str) -> String {
    if crate::syntax::split_top_level_whitespace(input).is_some_and(|values| values.len() == 1) {
        if let Some(value) = shorthand_longhands(name).and_then(|longhands| {
            longhands
                .iter()
                .find_map(|longhand| project_observable_value(longhand, input))
        }) {
            return value;
        }
    }
    if starts_math_function(input) && !starts_math_function(canonical) {
        return format!("calc({canonical})");
    }
    serialize_shorthand_tokens(input)
}

fn serialize_shorthand_tokens(input: &str) -> String {
    let value = canonicalize_unquoted_urls(input);
    let value = replace_gradient_color_tokens(&value);
    let value = replace_comments_with_space(&value);
    let value = normalize_comma_whitespace(&value);
    let value = canonicalize_leading_decimal(&value);
    canonicalize_color_identifiers(&value)
}

fn serialize_webkit_border_image_observable(input: &str, canonical: &str) -> String {
    let value = serialize_shorthand_tokens(input);
    if split_top_level_delimiter(&value, b'/').is_some_and(|sections| sections.len() > 2) {
        return canonical.to_owned();
    }
    let Some(components) = crate::syntax::split_top_level_whitespace(&value) else {
        return value;
    };
    let Some(first) = components.first().copied() else {
        return value;
    };
    if components.contains(&"fill") || !is_numeric_or_math_component(first) {
        return value;
    }
    let authored_sections = split_top_level_delimiter(&value, b'/').unwrap_or_default();
    let canonical_sections = split_top_level_delimiter(canonical, b'/').unwrap_or_default();
    let Some(section) = canonical_sections.first().map(|section| section.trim()) else {
        return canonical.to_owned();
    };
    let slice = section.strip_prefix("none ").unwrap_or(section);
    let slice = if starts_math_function(first) && !starts_math_function(slice) {
        slice
            .strip_suffix(" fill")
            .map_or_else(|| slice.to_owned(), |value| format!("calc({value}) fill"))
    } else {
        slice.to_owned()
    };
    if authored_sections.len() == 1 {
        return slice;
    }
    if authored_sections.len() == 2 {
        let Some(width) = canonical_sections.get(1).map(|section| section.trim()) else {
            return canonical.to_owned();
        };
        return format!("{slice} / {width}");
    }
    value
}

fn is_numeric_or_math_component(value: &str) -> bool {
    value
        .chars()
        .next()
        .is_some_and(|character| character.is_ascii_digit() || matches!(character, '+' | '-' | '.'))
        || starts_math_function(value)
}

fn serialize_webkit_mask_box_image_slice_observable(input: &str, canonical: &str) -> String {
    if starts_math_function(input) && !starts_math_function(canonical) {
        return canonical.strip_suffix(" fill").map_or_else(
            || canonical.to_owned(),
            |value| format!("calc({value}) fill"),
        );
    }
    canonical.to_owned()
}

fn serialize_default_observable(
    input: &str,
    closed: &str,
    canonical: &str,
    recovered: &RecoveredObservableText,
) -> String {
    if !recovered.recovered {
        return canonicalize_leading_decimal(canonical);
    }
    if starts_with_ignore_ascii_case(closed, "url(") || input.starts_with(['\'', '"']) {
        return canonical.to_owned();
    }
    if input.contains("/*") {
        return trim_css_whitespace(&recovered.retained).to_owned();
    }
    closed.to_owned()
}

fn canonicalize_leading_decimal(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut result = String::with_capacity(value.len() + 1);
    let mut index = 0usize;
    while index < bytes.len() {
        let signed = matches!(bytes[index], b'+' | b'-');
        let dot = if signed { index + 1 } else { index };
        if bytes.get(dot) == Some(&b'.')
            && bytes.get(dot + 1).is_some_and(u8::is_ascii_digit)
            && (index == 0
                || !bytes[index - 1].is_ascii_alphanumeric()
                    && !matches!(bytes[index - 1], b'_' | b'-'))
        {
            if signed {
                result.push(bytes[index] as char);
            }
            result.push_str("0.");
            index = dot + 1;
            continue;
        }
        let character = value[index..].chars().next().unwrap_or('\u{fffd}');
        result.push(character);
        index += character.len_utf8();
    }
    result
}

fn trim_token_stream_trivia(mut value: &str) -> &str {
    loop {
        value = trim_css_whitespace(value);
        let Some(comment) = value.strip_prefix("/*") else {
            break;
        };
        let Some(end) = comment.find("*/") else {
            break;
        };
        value = &comment[end + 2..];
    }
    loop {
        value = trim_css_whitespace(value);
        let Some(comment_body) = value.strip_suffix("*/") else {
            break;
        };
        let Some(start) = comment_body.rfind("/*") else {
            break;
        };
        value = &comment_body[..start];
    }
    trim_css_whitespace(value)
}

fn trim_css_whitespace(value: &str) -> &str {
    value.trim_matches(|character| matches!(character, ' ' | '\t' | '\n' | '\r' | '\u{000c}'))
}

fn serialize_font_family(
    input: &str,
    safe_value: &str,
    single_string: Option<&str>,
    recovered: bool,
) -> String {
    if let Some(value) = single_string {
        if is_identifier(value) && !is_generic_font_family(value) {
            return value.to_owned();
        }
        if !recovered && input.ends_with(['\'', '"']) && safe_value.starts_with(['\'', '"']) {
            return safe_value.to_owned();
        }
        return quote_css_string(value);
    }

    let Some(families) = split_top_level_delimiter(safe_value, b',') else {
        return safe_value.to_owned();
    };
    let mut serialized = String::new();
    for family in families {
        push_delimited(&mut serialized, ", ", &serialize_font_family_member(family));
    }
    serialized
}

fn serialize_font_family_member(value: &str) -> String {
    let value = trim_css_whitespace(value);
    if value.starts_with(['\'', '"']) || is_identifier(value) {
        return value.to_owned();
    }
    quote_css_string(value)
}

fn quote_css_string(value: &str) -> String {
    let mut quoted = String::with_capacity(value.len() + 2);
    quoted.push('"');
    let mut cursor = 0;
    for (index, byte) in value.bytes().enumerate() {
        if !matches!(byte, b'\\' | b'"') {
            continue;
        }
        quoted.push_str(&value[cursor..index]);
        quoted.push('\\');
        quoted.push(char::from(byte));
        cursor = index + 1;
    }
    quoted.push_str(&value[cursor..]);
    quoted.push('"');
    quoted
}

fn is_identifier(value: &str) -> bool {
    let mut input = ParserInput::new(value);
    let mut parser = Parser::new(&mut input);
    parser.expect_ident().is_ok() && parser.expect_exhausted().is_ok()
}

fn is_generic_font_family(value: &str) -> bool {
    [
        "serif",
        "sans-serif",
        "monospace",
        "cursive",
        "fantasy",
        "system-ui",
        "ui-serif",
        "ui-sans-serif",
        "ui-monospace",
        "ui-rounded",
        "math",
        "fangsong",
        "emoji",
    ]
    .iter()
    .any(|candidate| value.eq_ignore_ascii_case(candidate))
}

fn serialize_color(value: &str, safe_value: &str) -> String {
    if is_relative_color_function(value) {
        return safe_value.to_owned();
    }
    if let Some(color) = serialize_rgb_color(value) {
        return color;
    }
    if let Some(color) = serialize_hex_color(value) {
        return color;
    }
    if value
        .chars()
        .all(|character| character.is_ascii_alphabetic() || character == '-')
    {
        return value.to_ascii_lowercase();
    }
    if ["hsl(", "hsla(", "hwb("]
        .iter()
        .any(|prefix| starts_with_ignore_ascii_case(value, prefix))
    {
        return serialize_hex_color(safe_value).unwrap_or_else(|| value.to_owned());
    }
    if ["lab(", "lch(", "oklab(", "oklch(", "color("]
        .iter()
        .any(|prefix| starts_with_ignore_ascii_case(value, prefix))
    {
        return canonicalize_modern_color(safe_value);
    }
    canonicalize_color_identifiers(value)
}

fn is_single_color_property(name: &str) -> bool {
    if !(name == "color" || name.ends_with("-color") || matches!(name, "fill" | "stroke")) {
        return false;
    }
    semantic_accepts(name, "red") && !semantic_accepts(name, "red blue")
}

fn semantic_accepts(name: &str, value: &str) -> bool {
    parse_semantic_property(name, value).is_ok_and(|declaration| {
        matches!(
            declaration.parse_kind(),
            PropertyParseKind::Typed | PropertyParseKind::SheetomTyped
        )
    })
}

fn serialize_plain_time_list(input: &str) -> Option<String> {
    let values = split_top_level_delimiter(input, b',')?;
    let mut serialized = String::new();
    for value in values {
        push_delimited(&mut serialized, ", ", &serialize_plain_time(value.trim())?);
    }
    Some(serialized)
}

fn push_delimited(output: &mut String, separator: &str, value: &str) {
    if !output.is_empty() {
        output.push_str(separator);
    }
    output.push_str(value);
}

fn serialize_plain_time(input: &str) -> Option<String> {
    let unit_start = input.find(|character: char| character.is_ascii_alphabetic())?;
    let (number, unit) = input.split_at(unit_start);
    let unit = if unit.eq_ignore_ascii_case("s") {
        "s"
    } else if unit.eq_ignore_ascii_case("ms") {
        "ms"
    } else {
        return None;
    };
    let number = number.parse::<f64>().ok()?;
    number
        .is_finite()
        .then(|| format!("{}{unit}", serialize_finite_number(number)))
}

fn serialize_dimensionless_zero(name: &str, input: &str) -> Option<String> {
    let number = input.trim().parse::<f64>().ok()?;
    if number != 0.0 || semantic_accepts(name, "1") || !semantic_accepts(name, "1px") {
        return None;
    }
    let one = parse_semantic_property(name, "1px")
        .ok()?
        .canonical_value()
        .ok()?;
    let zero = replace_one_pixel_with_zero(&one)?;
    if is_position_pair_property(name) && !zero.contains(' ') {
        return Some(format!("{zero} center"));
    }
    Some(zero)
}

fn serialize_explicit_zero_dimension(name: &str, input: &str, canonical: &str) -> Option<String> {
    if canonical != "0" {
        return None;
    }
    let input = input.trim();
    let unit_start =
        input.find(|character: char| character.is_ascii_alphabetic() || character == '%')?;
    let (number, unit) = input.split_at(unit_start);
    if number.parse::<f64>().ok()? != 0.0 || unit.is_empty() {
        return None;
    }
    let value = format!("0{}", unit.to_ascii_lowercase());
    if is_position_pair_property(name) {
        return Some(format!("{value} center"));
    }
    Some(value)
}

fn is_zero_dimension(input: &str) -> bool {
    let input = input.trim();
    let Some(unit_start) =
        input.find(|character: char| character.is_ascii_alphabetic() || character == '%')
    else {
        return false;
    };
    let (number, unit) = input.split_at(unit_start);
    !unit.is_empty() && number.parse::<f64>().is_ok_and(|number| number == 0.0)
}

fn serialize_zero_percentage_as_number(name: &str, input: &str, canonical: &str) -> Option<String> {
    if canonical != "0" || !semantic_accepts(name, "1") || !semantic_accepts(name, "100%") {
        return None;
    }
    let percentage = input.trim().strip_suffix('%')?.parse::<f64>().ok()?;
    (percentage == 0.0).then(|| "0".to_owned())
}

fn replace_one_pixel_with_zero(input: &str) -> Option<String> {
    let mut tokenizer = TokenizerWithSpans::new(input);
    let mut replacements = Vec::new();
    while let Ok(token) = tokenizer.next_token() {
        let Token::Dimension {
            value, ref unit, ..
        } = token.token
        else {
            continue;
        };
        if value == 1.0 && unit.eq_ignore_ascii_case("px") {
            replacements.push((token.start.byte_index(), token.end.byte_index()));
        }
    }
    if replacements.is_empty() {
        return None;
    }
    let mut output = input.to_owned();
    for (start, end) in replacements.into_iter().rev() {
        output.replace_range(start..end, "0px");
    }
    Some(output)
}

fn canonicalize_modern_color(input: &str) -> String {
    let (scale_lightness, unscale_lightness) = if starts_with_ignore_ascii_case(input, "oklab(")
        || starts_with_ignore_ascii_case(input, "oklch(")
    {
        (true, false)
    } else if starts_with_ignore_ascii_case(input, "lab(")
        || starts_with_ignore_ascii_case(input, "lch(")
    {
        (false, true)
    } else {
        (false, false)
    };
    let mut tokenizer = TokenizerWithSpans::new(input);
    let mut depth = 0usize;
    let mut first_component = true;
    let mut alpha_component = false;
    let mut replacements = Vec::<(usize, usize, String)>::new();
    while let Ok(token) = tokenizer.next_token() {
        match token.token {
            Token::Function(_)
            | Token::ParenthesisBlock
            | Token::SquareBracketBlock
            | Token::CurlyBracketBlock => {
                depth += 1;
            }
            Token::CloseParenthesis | Token::CloseSquareBracket | Token::CloseCurlyBracket => {
                depth = depth.saturating_sub(1);
            }
            Token::Delim('/') if depth == 1 => alpha_component = true,
            Token::Percentage { unit_value, .. }
                if depth == 1 && (first_component || alpha_component) =>
            {
                let value = if first_component && unscale_lightness {
                    f64::from(unit_value) * 100.0
                } else if first_component && scale_lightness || alpha_component {
                    f64::from(unit_value)
                } else {
                    continue;
                };
                replacements.push((
                    token.start.byte_index(),
                    token.end.byte_index(),
                    serialize_finite_number(value),
                ));
                first_component = false;
                alpha_component = false;
            }
            Token::WhiteSpace(_) | Token::Comment(_) if depth == 1 => {}
            _ if depth == 1 => {
                first_component = false;
                alpha_component = false;
            }
            _ => {}
        }
    }
    let mut output = input.to_owned();
    for (start, end, replacement) in replacements.into_iter().rev() {
        output.replace_range(start..end, &replacement);
    }
    if let Some(open) = output.find('(') {
        output[..open].make_ascii_lowercase();
    }
    canonicalize_leading_decimal(&output)
}

fn serialize_finite_number(value: f64) -> String {
    if value == 0.0 {
        return "0".to_owned();
    }
    let mut output = value.to_string();
    if output.contains('.') {
        while output.ends_with('0') {
            output.pop();
        }
        if output.ends_with('.') {
            output.pop();
        }
    }
    output
}

pub(crate) fn serialize_observable_color(value: &str) -> String {
    serialize_color(value, value)
}

fn is_relative_color_function(value: &str) -> bool {
    let mut tokenizer = TokenizerWithSpans::new(value);
    let Some(Token::Function(function)) = next_significant_token(&mut tokenizer) else {
        return false;
    };
    if ![
        "rgb", "rgba", "hsl", "hsla", "hwb", "lab", "lch", "oklab", "oklch", "color",
    ]
    .iter()
    .any(|candidate| function.eq_ignore_ascii_case(candidate))
    {
        return false;
    }
    next_significant_token(&mut tokenizer).is_some_and(
        |token| matches!(token, Token::Ident(ident) if ident.eq_ignore_ascii_case("from")),
    )
}

fn next_significant_token<'i>(tokenizer: &mut TokenizerWithSpans<'i>) -> Option<Token<'i>> {
    loop {
        let token = tokenizer.next_token().ok()?.token;
        if !matches!(token, Token::WhiteSpace(_) | Token::Comment(_)) {
            return Some(token);
        }
    }
}

fn canonicalize_color_identifiers(value: &str) -> String {
    let mut tokenizer = TokenizerWithSpans::new(value);
    let mut output = String::with_capacity(value.len());
    let mut cursor = 0usize;
    while let Ok(token) = tokenizer.next_token() {
        let Token::Ident(identifier) = token.token else {
            continue;
        };
        if !identifier.eq_ignore_ascii_case("currentcolor") {
            continue;
        }
        let start = token.start.byte_index();
        let end = token.end.byte_index();
        let Some(prefix) = value.get(cursor..start) else {
            return value.to_owned();
        };
        output.push_str(prefix);
        output.push_str("currentcolor");
        cursor = end;
    }
    let Some(suffix) = value.get(cursor..) else {
        return value.to_owned();
    };
    output.push_str(suffix);
    output
}

fn serialize_hex_color(value: &str) -> Option<String> {
    let hex = value.strip_prefix('#')?;
    let bytes = hex.as_bytes();
    let (red, green, blue, alpha) = match bytes {
        [red, green, blue] => (
            parse_hex_digit(*red)? * 17,
            parse_hex_digit(*green)? * 17,
            parse_hex_digit(*blue)? * 17,
            None,
        ),
        [red, green, blue, alpha] => (
            parse_hex_digit(*red)? * 17,
            parse_hex_digit(*green)? * 17,
            parse_hex_digit(*blue)? * 17,
            Some(parse_hex_digit(*alpha)? * 17),
        ),
        [red_high, red_low, green_high, green_low, blue_high, blue_low] => (
            parse_hex_byte(*red_high, *red_low)?,
            parse_hex_byte(*green_high, *green_low)?,
            parse_hex_byte(*blue_high, *blue_low)?,
            None,
        ),
        [red_high, red_low, green_high, green_low, blue_high, blue_low, alpha_high, alpha_low] => (
            parse_hex_byte(*red_high, *red_low)?,
            parse_hex_byte(*green_high, *green_low)?,
            parse_hex_byte(*blue_high, *blue_low)?,
            Some(parse_hex_byte(*alpha_high, *alpha_low)?),
        ),
        _ => return None,
    };
    let Some(alpha) = alpha else {
        return Some(format!("rgb({red}, {green}, {blue})"));
    };
    Some(format!(
        "rgba({red}, {green}, {blue}, {})",
        format_number(f64::from(alpha) / 255.0)
    ))
}

fn parse_hex_byte(high: u8, low: u8) -> Option<u8> {
    Some((parse_hex_digit(high)? << 4) | parse_hex_digit(low)?)
}

fn parse_hex_digit(value: u8) -> Option<u8> {
    match value {
        b'0'..=b'9' => Some(value - b'0'),
        b'a'..=b'f' => Some(value - b'a' + 10),
        b'A'..=b'F' => Some(value - b'A' + 10),
        _ => None,
    }
}

fn serialize_rgb_color(value: &str) -> Option<String> {
    let open = value.find('(')?;
    let function = value[..open].trim();
    if !(function.eq_ignore_ascii_case("rgb") || function.eq_ignore_ascii_case("rgba"))
        || !value.ends_with(')')
    {
        return None;
    }
    let body = value[open + 1..value.len() - 1].trim();
    let (channels, slash_alpha) = body.split_once('/').map_or((body, None), |(left, right)| {
        (left.trim(), Some(right.trim()))
    });
    let mut parts = [None; 4];
    let mut part_count = 0;
    let mut push_part = |part| {
        let Some(slot) = parts.get_mut(part_count) else {
            return false;
        };
        *slot = Some(part);
        part_count += 1;
        true
    };
    if channels.contains(',') {
        for part in channels.split(',').map(str::trim) {
            if !push_part(part) {
                return None;
            }
        }
    } else {
        for part in channels.split_ascii_whitespace() {
            if !push_part(part) {
                return None;
            }
        }
    }
    let alpha = if part_count == 4 {
        parts[3]
    } else {
        slash_alpha
    };
    if part_count != 3 && part_count != 4 {
        return None;
    }
    let channels = [
        parse_color_channel(parts[0]?)?,
        parse_color_channel(parts[1]?)?,
        parse_color_channel(parts[2]?)?,
    ];
    if let Some(alpha) = alpha {
        return Some(format!(
            "rgba({}, {}, {}, {})",
            channels[0],
            channels[1],
            channels[2],
            format_number(parse_alpha(alpha)?)
        ));
    }
    Some(format!(
        "rgb({}, {}, {})",
        channels[0], channels[1], channels[2]
    ))
}

fn starts_with_ignore_ascii_case(value: &str, prefix: &str) -> bool {
    value
        .get(..prefix.len())
        .is_some_and(|candidate| candidate.eq_ignore_ascii_case(prefix))
}

fn parse_color_channel(value: &str) -> Option<u8> {
    let number = value.trim_end_matches('%').parse::<f64>().ok()?;
    let normalized = if value.ends_with('%') {
        number.clamp(0.0, 100.0) * 255.0 / 100.0
    } else {
        number.clamp(0.0, 255.0)
    };
    Some(normalized.round() as u8)
}

fn parse_alpha(value: &str) -> Option<f64> {
    let number = value.trim_end_matches('%').parse::<f64>().ok()?;
    Some(if value.ends_with('%') {
        (number / 100.0).clamp(0.0, 1.0)
    } else {
        number.clamp(0.0, 1.0)
    })
}

fn format_number(value: f64) -> String {
    let rounded = (value * 1000.0).round() / 1000.0;
    if rounded.fract() == 0.0 {
        format!("{rounded:.0}")
    } else {
        rounded.to_string()
    }
}

fn serialize_integer_calculation(value: &str) -> Option<String> {
    let body = value.strip_prefix("calc(")?.strip_suffix(')')?.trim();
    for operator in ['+', '-'] {
        let Some((left, right)) = body.split_once(operator) else {
            continue;
        };
        let left = left.trim().parse::<f64>().ok()?;
        let right = right.trim().parse::<f64>().ok()?;
        let result = if operator == '+' {
            left + right
        } else {
            left - right
        };
        if result.fract() == 0.0 {
            return Some(format!("calc({result:.0})"));
        }
    }
    None
}

fn starts_math_function(value: &str) -> bool {
    let mut tokenizer = TokenizerWithSpans::new(value);
    let Some(Token::Function(function)) = next_significant_token(&mut tokenizer) else {
        return false;
    };
    [
        "calc", "min", "max", "clamp", "round", "rem", "mod", "abs", "sign", "hypot", "sin", "cos",
        "tan", "asin", "acos", "atan", "atan2", "pow", "sqrt", "log", "exp",
    ]
    .iter()
    .any(|candidate| function.eq_ignore_ascii_case(candidate))
}

#[cfg(test)]
mod tests {
    use super::{project_declaration, quote_css_string};
    use crate::parse_semantic_property;

    fn observable(name: &str, input: &str) -> String {
        let declaration = parse_semantic_property(name, input).unwrap();
        project_declaration(&declaration).unwrap().observable
    }

    #[test]
    fn recovers_browser_facing_token_text() {
        assert_eq!(observable("--x", "red/*comment"), "red");
        assert_eq!(observable("--x", "foo\\"), "foo�");
        assert_eq!(observable("width", "calc(1px"), "calc(1px)");
    }

    #[test]
    fn preserves_numeric_units_and_per_item_math_provenance() {
        for (name, input, expected) in [
            ("width", "0", "0px"),
            ("font-size", "-0", "0px"),
            ("transform-origin", "0", "0px center"),
            ("stroke-width", "0", "0"),
            ("stroke-width", "0px", "0px"),
        ] {
            assert_eq!(observable(name, input), expected, "{name}: {input}");
        }
        assert_eq!(
            observable("stroke-dasharray", "calc(1 + 1) 2"),
            "calc(2), 2"
        );
        for name in [
            "opacity",
            "fill-opacity",
            "flood-opacity",
            "shape-image-threshold",
            "scale",
            "stop-opacity",
            "stroke-opacity",
        ] {
            assert_eq!(observable(name, "0%"), "0", "{name}");
        }
        assert_eq!(observable("width", "0%"), "0%");
    }

    #[test]
    fn serializes_transform_origin_like_chromium() {
        for (input, expected) in [
            ("left", "left center"),
            ("top", "center top"),
            ("top left", "left top"),
            ("0 0", "0px 0px"),
            ("left top 1px", "left top 1px"),
            ("center center 0", "center center 0px"),
            ("center center calc(1px + 2px)", "center center calc(3px)"),
            (
                "calc(1px + 2px) center calc(3px + 4px)",
                "calc(3px) center calc(7px)",
            ),
        ] {
            assert_eq!(observable("transform-origin", input), expected, "{input}");
        }
    }

    #[test]
    fn preserves_explicit_text_emphasis_defaults() {
        for (name, input, expected) in [
            ("text-emphasis-style", "filled dot", "filled dot"),
            ("text-emphasis-style", "dot filled", "filled dot"),
            ("text-emphasis-style", "dot", "dot"),
            ("text-emphasis-position", "right over", "over right"),
            ("text-emphasis-position", "over right", "over right"),
            ("text-emphasis-position", "over", "over"),
        ] {
            assert_eq!(observable(name, input), expected, "{name}: {input}");
        }
    }

    #[test]
    fn canonicalizes_composite_observable_defaults() {
        for (name, input, expected) in [
            ("view-timeline-inset", "auto auto", "auto"),
            ("view-timeline-inset", "0", "0px"),
            ("view-timeline-inset", "min(1px, 2px)", "calc(1px)"),
            ("view-timeline-inset", "0 0", "0px"),
            ("view-timeline-inset", "min(1px, 2px) 2px", "calc(1px) 2px"),
            ("scroll-snap-align", "none none", "none"),
            ("overflow-clip-margin", "content-box 0px", "content-box"),
            ("font-style", "oblique 0deg", "normal"),
            ("text-shadow", "1px 2px red", "red 1px 2px"),
            (
                "text-shadow",
                "1px 2px red, 1px 2px blue",
                "red 1px 2px, blue 1px 2px",
            ),
        ] {
            assert_eq!(observable(name, input), expected, "{name}: {input}");
        }
    }

    #[test]
    fn serializes_border_image_slice_math_per_component() {
        for (input, expected) in [
            ("calc(-1) fill", "calc(-1) fill"),
            ("calc(1 + 1) fill", "calc(2) fill"),
            ("min(1, 2) fill", "calc(1) fill"),
            ("min(1%, 2%) fill", "min(1%, 2%) fill"),
            ("calc(1 + 1) 2 fill", "calc(2) 2 fill"),
            ("2 calc(1 + 1) fill", "2 calc(2) fill"),
            ("calc(1 + 1) calc(1 + 1) fill", "calc(2) fill"),
        ] {
            assert_eq!(observable("border-image-slice", input), expected, "{input}");
        }
    }

    #[test]
    fn preserves_authored_time_units_outside_math() {
        for (input, expected) in [
            ("100ms", "100ms"),
            ("100MS", "100ms"),
            (".1s", "0.1s"),
            ("0.10s", "0.1s"),
            ("100ms, .2S", "100ms, 0.2s"),
            ("calc(100ms)", "calc(0.1s)"),
        ] {
            assert_eq!(observable("animation-duration", input), expected, "{input}");
        }
    }

    #[test]
    fn serializes_legacy_webkit_border_image_like_chromium() {
        for (input, expected) in [
            ("url(\"x.png\")", "url(\"x.png\")"),
            ("10%", "10% fill"),
            ("1 / 2", "1 fill / 2"),
            ("calc(1 + 1)", "calc(2) fill"),
            (
                "url(\"x.png\") 30 / 10 / 0 stretch",
                "url(\"x.png\") 30 fill / 10 / 0 stretch",
            ),
        ] {
            assert_eq!(
                observable("-webkit-border-image", input),
                expected,
                "{input}"
            );
        }
    }

    #[test]
    fn serializes_typed_math_like_chromium_cssom() {
        for (name, input, expected) in [
            ("width", "calc(1px / 2)", "calc(0.5px)"),
            ("width", "min(1px, 2%)", "min(1px, 2%)"),
            ("width", "round(1px, 2px)", "calc(2px)"),
            ("width", "hypot(3px, 4px)", "calc(5px)"),
            ("rotate", "atan2(1, 1)", "calc(45deg)"),
            ("opacity", "pow(2, 3)", "calc(8)"),
        ] {
            assert_eq!(observable(name, input), expected, "{name}: {input}");
        }

        let declaration = parse_semantic_property("width", "rem(-5px, 2px)").unwrap();
        let projection = project_declaration(&declaration).unwrap();
        assert_eq!(projection.observable, "calc(-1px)");
        assert_eq!(projection.canonical, "calc(-1px)");
    }

    #[test]
    fn preserves_internal_comments_for_custom_and_pending_token_streams() {
        for (name, input, expected) in [
            ("--x", "a/*c*/b", "a/*c*/b"),
            ("--x", "\u{00a0}red\u{00a0}", "\u{00a0}red\u{00a0}"),
            ("--x", "/*c*/a/*tail*/", "a"),
            (
                "width",
                "calc(var(--x)/*c*/ + 1px)",
                "calc(var(--x)/*c*/ + 1px)",
            ),
            ("width", "--f(a/*c*/,b)", "--f(a/*c*/,b)"),
            ("width", "--f(a)/*c*/", "--f(a)"),
            ("width", "--f(a/*c)", "--f(a"),
        ] {
            assert_eq!(observable(name, input), expected, "{input}");
        }
    }

    #[test]
    fn serializes_cssom_colors() {
        assert_eq!(
            observable("color", "rgb(1 2 3 / 50%)"),
            "rgba(1, 2, 3, 0.5)"
        );
        assert_eq!(observable("color", "white"), "white");
        assert_eq!(
            observable(
                "color",
                "color-mix(in srgb, contrast-color(red), currentColor)",
            ),
            "color-mix(in srgb, contrast-color(red), currentcolor)"
        );
        assert_eq!(
            observable("color", "contrast-color(current\\43 olor)"),
            "contrast-color(currentcolor)"
        );
        assert_eq!(
            observable(
                "color",
                "RGBA(from rgb(20%, 40%, 60%, 80%) r calc(g * .5 + g * .5) b / alpha)",
            ),
            "rgb(from rgba(51, 102, 153, 0.8) r calc((0.5 * g) + (0.5 * g)) b / alpha)"
        );
        assert_eq!(
            observable(
                "color",
                "lab(from var(--mycolor) l a b / calc(alpha * 0.8))",
            ),
            "lab(from var(--mycolor) l a b / calc(alpha * 0.8))"
        );
        for (name, input, expected) in [
            ("fill", "#123456", "rgb(18, 52, 86)"),
            ("stroke", "rgb(1 2 3 / 50%)", "rgba(1, 2, 3, 0.5)"),
            ("color", "lab(50% 20 30 / 50%)", "lab(50 20 30 / 0.5)"),
            (
                "color",
                "oklch(50% .2 120 / 50%)",
                "oklch(0.5 0.2 120 / 0.5)",
            ),
            (
                "color",
                "color(display-p3 .1 .2 .3 / 50%)",
                "color(display-p3 0.1 0.2 0.3 / 0.5)",
            ),
        ] {
            assert_eq!(observable(name, input), expected, "{name}: {input}");
        }
    }

    #[test]
    fn serializes_font_family_lists_like_cssom() {
        for (input, expected) in [
            ("left top", "\"left top\""),
            ("safe center", "\"safe center\""),
            ("--x block", "\"--x block\""),
            ("foo, bar baz", "foo, \"bar baz\""),
            ("serif", "serif"),
            ("\u{00a0}A\u{00a0}", "\u{00a0}A\u{00a0}"),
        ] {
            assert_eq!(observable("font-family", input), expected, "{input}");
        }
        assert_eq!(quote_css_string("A\\B\"é"), "\"A\\\\B\\\"é\"");
    }

    #[test]
    fn serializes_gradient_images_without_erasing_authored_color_identity() {
        for (input, expected) in [
            ("linear-gradient(red,blue)", "linear-gradient(red, blue)"),
            (
                "linear-gradient(red 0%, rgb(0 0 255) 100%)",
                "linear-gradient(red 0%, rgb(0, 0, 255) 100%)",
            ),
            (
                "LINEAR-GRADIENT(#f00 0%, rgba(0,0,255,.5) 100%)",
                "linear-gradient(rgb(255, 0, 0) 0%, rgba(0, 0, 255, 0.5) 100%)",
            ),
            (
                "linear-gradient(hsl(120 100% 50%), transparent)",
                "linear-gradient(rgb(0, 255, 0), transparent)",
            ),
        ] {
            assert_eq!(observable("shape-outside", input), expected, "{input}");
        }
    }
}
