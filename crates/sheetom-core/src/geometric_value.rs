use cssparser::{Parser, ParserInput, Token, TokenizerWithSpans};
use lightningcss::{
    error::ParserError,
    stylesheet::PrinterOptions,
    traits::{IntoOwned, Parse, ToCss, TrySign},
    values::{
        angle::{Angle, AnglePercentage},
        color::CssColor,
        gradient::{
            ConicGradient, Gradient, GradientItem, LineDirection, LinearGradient, RadialGradient,
        },
        image::Image,
        length::{LengthPercentage, LengthValue},
        position::{HorizontalPosition, Position, PositionComponent, VerticalPosition},
        string::CSSString,
    },
};
use svgtypes::{PathParser, PathSegment};

use crate::{syntax::split_top_level_delimiter, EngineError};

type ParseError<'i> = cssparser::ParseError<'i, ParserError<'i>>;

const GEOMETRIC_PROPERTIES: &[&str] = &[
    "border-shape",
    "clip-path",
    "d",
    "object-view-box",
    "shape-outside",
];

pub(crate) fn has_geometric_property_grammar(property_name: &str) -> bool {
    GEOMETRIC_PROPERTIES.contains(&property_name)
}

#[derive(Clone, Debug, PartialEq)]
pub enum GeometricValue {
    BorderShape(Box<BorderShapeValue>),
    ClipPath(Box<ClipPathValue>),
    D(Box<PathPropertyValue>),
    ObjectViewBox(Box<ObjectViewBoxValue>),
    ShapeOutside(Box<ShapeOutsideValue>),
}

#[derive(Clone, Debug, PartialEq)]
pub struct ClipPathValue {
    shape: BasicShapeValue,
    geometry_box: Option<&'static str>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum PathPropertyValue {
    None,
    Path(SvgPathData),
}

#[derive(Clone, Debug, PartialEq)]
pub enum ObjectViewBoxValue {
    None,
    Shape(BasicShapeValue),
}

#[derive(Clone, Debug, PartialEq)]
pub enum ShapeOutsideValue {
    Image {
        value: Image<'static>,
        authored_gradient: Option<AuthoredGradient>,
    },
    Box(&'static str),
    Shape {
        shape: BasicShapeValue,
        geometry_box: Option<&'static str>,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct AuthoredGradient {
    stops: Vec<AuthoredGradientStop>,
    position: Option<Position>,
    interpolation: Option<GradientInterpolation>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct GradientInterpolation {
    color_space: &'static str,
    hue_method: Option<&'static str>,
}

#[derive(Clone, Debug, PartialEq)]
struct AuthoredGradientStop {
    color: CssColor,
    source: String,
    occurrences: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub enum BorderShapeValue {
    None,
    Shapes {
        outer: Box<BorderShapeItem>,
        inner: Option<Box<BorderShapeItem>>,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct BorderShapeItem {
    shape: BasicShapeValue,
    geometry_box: Option<&'static str>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum BasicShapeValue {
    Inset(Box<InsetShape>),
    Circle(Box<CircleShape>),
    Ellipse(Box<EllipseShape>),
    Polygon(Box<PolygonShape>),
    Rect(Box<RectShape>),
    Xywh(Box<XywhShape>),
    Path(Box<BasicShapePath>),
    Shape(Box<ShapeFunction>),
}

#[derive(Clone, Debug, PartialEq)]
pub struct InsetShape {
    sides: [ShapeLength; 4],
    radii: Option<ShapeRadii>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CircleShape {
    radius: ShapeRadius,
    position: Position,
    position_explicit: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EllipseShape {
    radius_x: ShapeRadius,
    radius_y: ShapeRadius,
    position: Position,
    position_explicit: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PolygonShape {
    fill_rule: FillRule,
    points: Vec<CoordinatePair>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RectShape {
    sides: [RectSide; 4],
    radii: Option<ShapeRadii>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct XywhShape {
    x: ShapeLength,
    y: ShapeLength,
    width: ShapeLength,
    height: ShapeLength,
    radii: Option<ShapeRadii>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BasicShapePath {
    fill_rule: FillRule,
    path: SvgPathData,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum FillRule {
    #[default]
    Nonzero,
    Evenodd,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ShapeRadius {
    ClosestSide,
    FarthestSide,
    Length(ShapeLength),
}

#[derive(Clone, Debug, PartialEq)]
pub enum RectSide {
    Auto,
    Length(ShapeLength),
}

#[derive(Clone, Debug, PartialEq)]
pub struct ShapeLength {
    value: LengthPercentage,
    authored_math: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ShapeRadii {
    horizontal: [ShapeLength; 4],
    vertical: [ShapeLength; 4],
}

#[derive(Clone, Debug, PartialEq)]
pub struct CoordinatePair {
    x: ShapeLength,
    y: ShapeLength,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SvgPathData {
    commands: Vec<SvgPathCommand>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum SvgPathCommand {
    MoveTo {
        absolute: bool,
        x: f64,
        y: f64,
    },
    LineTo {
        absolute: bool,
        x: f64,
        y: f64,
    },
    HorizontalLineTo {
        absolute: bool,
        x: f64,
    },
    VerticalLineTo {
        absolute: bool,
        y: f64,
    },
    Cubic {
        absolute: bool,
        x1: f64,
        y1: f64,
        x2: f64,
        y2: f64,
        x: f64,
        y: f64,
    },
    SmoothCubic {
        absolute: bool,
        x2: f64,
        y2: f64,
        x: f64,
        y: f64,
    },
    Quadratic {
        absolute: bool,
        x1: f64,
        y1: f64,
        x: f64,
        y: f64,
    },
    SmoothQuadratic {
        absolute: bool,
        x: f64,
        y: f64,
    },
    Arc {
        absolute: bool,
        rx: f64,
        ry: f64,
        rotation: f64,
        large: bool,
        sweep: bool,
        x: f64,
        y: f64,
    },
    Close,
}

pub(crate) fn parse_geometric_property(
    property_name: &str,
    source: &str,
) -> Result<Option<GeometricValue>, EngineError> {
    let value = match property_name {
        "border-shape" => GeometricValue::BorderShape(Box::new(parse_border_shape(source)?)),
        "clip-path" => GeometricValue::ClipPath(Box::new(parse_clip_path(source)?)),
        "d" => GeometricValue::D(Box::new(parse_path_property(source)?)),
        "object-view-box" => {
            GeometricValue::ObjectViewBox(Box::new(parse_object_view_box(source)?))
        }
        "shape-outside" => GeometricValue::ShapeOutside(Box::new(parse_shape_outside(source)?)),
        _ => return Ok(None),
    };
    Ok(Some(value))
}

fn parse_clip_path(source: &str) -> Result<ClipPathValue, EngineError> {
    validate_strict_shape_numbers(source)?;
    parse_entire(source, |input| {
        let leading_box = input.try_parse(parse_clip_geometry_box).ok();
        let shape = parse_basic_shape(input)?;
        let geometry_box = if leading_box.is_some() {
            leading_box
        } else {
            input.try_parse(parse_clip_geometry_box).ok()
        };
        Ok(ClipPathValue {
            shape,
            geometry_box: geometry_box.filter(|value| *value != "border-box"),
        })
    })
}

fn parse_path_property(source: &str) -> Result<PathPropertyValue, EngineError> {
    parse_entire(source, |input| {
        if input
            .try_parse(|input| input.expect_ident_matching("none"))
            .is_ok()
        {
            return Ok(PathPropertyValue::None);
        }
        let location = input.current_source_location();
        let function = input.expect_function()?.clone();
        if !function.eq_ignore_ascii_case("path") {
            return Err(location.new_unexpected_token_error(Token::Function(function)));
        }
        input.parse_nested_block(|input| {
            let path = parse_svg_path_string(input)?;
            if path.commands.is_empty() {
                return Ok(PathPropertyValue::None);
            }
            Ok(PathPropertyValue::Path(path))
        })
    })
}

fn parse_object_view_box(source: &str) -> Result<ObjectViewBoxValue, EngineError> {
    validate_strict_shape_numbers(source)?;
    parse_entire(source, |input| {
        if input
            .try_parse(|input| input.expect_ident_matching("none"))
            .is_ok()
        {
            return Ok(ObjectViewBoxValue::None);
        }
        let shape = parse_basic_shape(input)?;
        if matches!(
            shape,
            BasicShapeValue::Inset(_) | BasicShapeValue::Rect(_) | BasicShapeValue::Xywh(_)
        ) {
            return Ok(ObjectViewBoxValue::Shape(shape));
        }
        Err(invalid(input))
    })
}

fn parse_shape_outside(source: &str) -> Result<ShapeOutsideValue, EngineError> {
    if let Ok(image) = parse_geometric_image(source) {
        let authored_gradient = match &image {
            Image::Gradient(gradient) => Some(extract_authored_gradient(source, gradient)?),
            _ => None,
        };
        return Ok(ShapeOutsideValue::Image {
            value: image.into_owned(),
            authored_gradient,
        });
    }
    validate_strict_shape_numbers(source)?;
    parse_entire(source, |input| {
        let leading_box = input.try_parse(parse_shape_box).ok();
        let shape = input.try_parse(parse_basic_shape).ok();
        if let Some(shape) = shape {
            let geometry_box = leading_box.or_else(|| input.try_parse(parse_shape_box).ok());
            return Ok(ShapeOutsideValue::Shape {
                shape,
                geometry_box: geometry_box.filter(|value| *value != "margin-box"),
            });
        }
        if let Some(geometry_box) = leading_box {
            return Ok(ShapeOutsideValue::Box(geometry_box));
        }
        Err(invalid(input))
    })
}

fn parse_geometric_image(source: &str) -> Result<Image<'static>, EngineError> {
    if let Ok(image) = parse_entire(source, Image::parse) {
        return Ok(image.into_owned());
    }
    let Some(stripped) = strip_gradient_interpolation(source)? else {
        return Err(EngineError::Parse("invalid geometric image".to_owned()));
    };
    parse_entire(&stripped, Image::parse).map(IntoOwned::into_owned)
}

fn strip_gradient_interpolation(source: &str) -> Result<Option<String>, EngineError> {
    let open = source
        .find('(')
        .ok_or_else(|| EngineError::Parse("gradient has no opening parenthesis".to_owned()))?;
    let close = source
        .rfind(')')
        .ok_or_else(|| EngineError::Parse("gradient has no closing parenthesis".to_owned()))?;
    let function = source[..open].trim();
    if !matches_ignore_ascii_case(
        function,
        &[
            "linear-gradient",
            "repeating-linear-gradient",
            "radial-gradient",
            "repeating-radial-gradient",
            "conic-gradient",
            "repeating-conic-gradient",
        ],
    ) {
        return Ok(None);
    }
    let body = source
        .get(open + 1..close)
        .ok_or_else(|| EngineError::Parse("gradient body is outside its source".to_owned()))?;
    let mut segments = split_top_level_delimiter(body, b',')
        .ok_or_else(|| EngineError::Parse("gradient items are malformed".to_owned()))?;
    let Some(header) = segments.first().copied() else {
        return Ok(None);
    };
    let Some((start, end, _)) = parse_gradient_interpolation(header)? else {
        return Ok(None);
    };
    let mut stripped_header = header.to_owned();
    stripped_header.replace_range(start..end, "");
    let stripped_header = stripped_header.trim();
    if stripped_header.is_empty() {
        segments.remove(0);
    } else {
        segments[0] = stripped_header;
    }
    Ok(Some(format!(
        "{}({})",
        function,
        segments
            .iter()
            .map(|segment| segment.trim())
            .collect::<Vec<_>>()
            .join(", ")
    )))
}

fn parse_gradient_interpolation(
    header: &str,
) -> Result<Option<(usize, usize, GradientInterpolation)>, EngineError> {
    let mut tokenizer = TokenizerWithSpans::new(header);
    let mut depth = 0usize;
    let mut tokens = Vec::<(usize, usize, Option<String>)>::new();
    while let Ok(token) = tokenizer.next_token() {
        match token.token {
            Token::Function(_)
            | Token::ParenthesisBlock
            | Token::SquareBracketBlock
            | Token::CurlyBracketBlock => {
                if depth == 0 {
                    tokens.push((token.start.byte_index(), token.end.byte_index(), None));
                }
                depth += 1;
            }
            Token::CloseParenthesis | Token::CloseSquareBracket | Token::CloseCurlyBracket => {
                depth = depth.saturating_sub(1);
            }
            Token::WhiteSpace(_) | Token::Comment(_) => {}
            Token::Ident(identifier) if depth == 0 => tokens.push((
                token.start.byte_index(),
                token.end.byte_index(),
                Some(identifier.to_ascii_lowercase()),
            )),
            _ if depth == 0 => {
                tokens.push((token.start.byte_index(), token.end.byte_index(), None));
            }
            _ => {}
        }
    }

    let Some(index) = tokens
        .iter()
        .position(|(_, _, identifier)| identifier.as_deref() == Some("in"))
    else {
        return Ok(None);
    };
    let Some(color_space) = tokens
        .get(index + 1)
        .and_then(|(_, _, identifier)| identifier.as_deref())
        .and_then(parse_gradient_color_space)
    else {
        return Err(EngineError::Parse(
            "gradient interpolation color space is invalid".to_owned(),
        ));
    };
    let mut last = index + 1;
    let hue_method = tokens
        .get(index + 2)
        .and_then(|(_, _, identifier)| identifier.as_deref())
        .and_then(parse_gradient_hue_method);
    if hue_method.is_some() {
        if !is_polar_gradient_color_space(color_space)
            || tokens
                .get(index + 3)
                .and_then(|(_, _, identifier)| identifier.as_deref())
                != Some("hue")
        {
            return Err(EngineError::Parse(
                "gradient hue interpolation is invalid".to_owned(),
            ));
        }
        last = index + 3;
    } else if tokens
        .get(index + 2)
        .and_then(|(_, _, identifier)| identifier.as_deref())
        == Some("hue")
    {
        return Err(EngineError::Parse(
            "gradient hue interpolation has no method".to_owned(),
        ));
    }
    if index != 0 && last + 1 != tokens.len() {
        return Err(EngineError::Parse(
            "gradient interpolation must precede or follow the other header terms".to_owned(),
        ));
    }
    Ok(Some((
        tokens[index].0,
        tokens[last].1,
        GradientInterpolation {
            color_space,
            hue_method,
        },
    )))
}

fn parse_gradient_color_space(value: &str) -> Option<&'static str> {
    [
        "srgb",
        "srgb-linear",
        "display-p3",
        "display-p3-linear",
        "a98-rgb",
        "prophoto-rgb",
        "rec2020",
        "lab",
        "oklab",
        "xyz",
        "xyz-d50",
        "xyz-d65",
        "hsl",
        "hwb",
        "lch",
        "oklch",
    ]
    .into_iter()
    .find(|candidate| value == *candidate)
}

fn parse_gradient_hue_method(value: &str) -> Option<&'static str> {
    ["shorter", "longer", "increasing", "decreasing"]
        .into_iter()
        .find(|candidate| value == *candidate)
}

fn is_polar_gradient_color_space(value: &str) -> bool {
    matches!(value, "hsl" | "hwb" | "lch" | "oklch")
}

fn extract_authored_gradient(
    source: &str,
    gradient: &Gradient,
) -> Result<AuthoredGradient, EngineError> {
    let open = source
        .find('(')
        .ok_or_else(|| EngineError::Parse("gradient has no opening parenthesis".to_owned()))?;
    let close = source
        .rfind(')')
        .ok_or_else(|| EngineError::Parse("gradient has no closing parenthesis".to_owned()))?;
    let body = source
        .get(open + 1..close)
        .ok_or_else(|| EngineError::Parse("gradient body is outside its source".to_owned()))?;
    let segments = split_top_level_delimiter(body, b',')
        .ok_or_else(|| EngineError::Parse("gradient items are malformed".to_owned()))?;
    let conic = matches!(gradient, Gradient::Conic(_) | Gradient::RepeatingConic(_));
    let header = segments.first().copied().unwrap_or_default();
    let interpolation = parse_gradient_interpolation(header)?.map(|(_, _, value)| value);
    let position = if matches!(
        gradient,
        Gradient::Radial(_)
            | Gradient::RepeatingRadial(_)
            | Gradient::Conic(_)
            | Gradient::RepeatingConic(_)
    ) {
        parse_authored_gradient_position(header)?
    } else {
        None
    };
    let mut stops = Vec::new();
    for segment in segments {
        let mut input = ParserInput::new(segment);
        let mut parser = Parser::new(&mut input);
        let Ok(color) = CssColor::parse(&mut parser) else {
            if matches!(gradient, Gradient::WebKitGradient(_)) {
                if let Some(stop) = extract_webkit_authored_color_stop(segment)? {
                    stops.push(stop);
                }
            }
            continue;
        };
        let color_end = parser.position().byte_index();
        let source = segment
            .get(..color_end)
            .ok_or_else(|| EngineError::Parse("gradient color span is invalid".to_owned()))?
            .trim()
            .to_owned();
        let mut position_count = 0usize;
        while !parser.is_exhausted() && position_count < 2 {
            if conic {
                parse_gradient_angle_percentage(&mut parser)
                    .map_err(|_| EngineError::Parse("gradient stop is malformed".to_owned()))?;
            } else {
                parse_gradient_length_percentage(&mut parser)
                    .map_err(|_| EngineError::Parse("gradient stop is malformed".to_owned()))?;
            }
            position_count += 1;
        }
        if !parser.is_exhausted() {
            return Err(EngineError::Parse(
                "gradient stop has too many positions".to_owned(),
            ));
        }
        stops.push(AuthoredGradientStop {
            color: color.into_owned(),
            source,
            occurrences: position_count.max(1),
        });
    }
    Ok(AuthoredGradient {
        stops,
        position,
        interpolation,
    })
}

fn parse_authored_gradient_position(header: &str) -> Result<Option<Position>, EngineError> {
    let mut header = header.to_owned();
    if let Some((start, end, _)) = parse_gradient_interpolation(&header)? {
        header.replace_range(start..end, "");
    }
    let mut tokenizer = TokenizerWithSpans::new(&header);
    let mut depth = 0usize;
    while let Ok(token) = tokenizer.next_token() {
        match token.token {
            Token::Function(_)
            | Token::ParenthesisBlock
            | Token::SquareBracketBlock
            | Token::CurlyBracketBlock => depth += 1,
            Token::CloseParenthesis | Token::CloseSquareBracket | Token::CloseCurlyBracket => {
                depth = depth.saturating_sub(1);
            }
            Token::Ident(identifier) if depth == 0 && identifier.eq_ignore_ascii_case("at") => {
                let source = header
                    .get(token.end.byte_index()..)
                    .ok_or_else(|| {
                        EngineError::Parse("gradient position span is invalid".to_owned())
                    })?
                    .trim();
                return parse_entire(source, Position::parse).map(Some);
            }
            _ => {}
        }
    }
    Ok(None)
}

fn extract_webkit_authored_color_stop(
    segment: &str,
) -> Result<Option<AuthoredGradientStop>, EngineError> {
    let segment = segment.trim();
    let Some(open) = segment.find('(') else {
        return Ok(None);
    };
    let Some(close) = segment.rfind(')') else {
        return Ok(None);
    };
    let function = segment[..open].trim();
    let body = segment
        .get(open + 1..close)
        .ok_or_else(|| EngineError::Parse("legacy gradient stop span is invalid".to_owned()))?;
    let source = if function.eq_ignore_ascii_case("from") || function.eq_ignore_ascii_case("to") {
        body.trim()
    } else if function.eq_ignore_ascii_case("color-stop") {
        let parts = split_top_level_delimiter(body, b',')
            .ok_or_else(|| EngineError::Parse("legacy gradient stop is malformed".to_owned()))?;
        let [_, color] = parts.as_slice() else {
            return Ok(None);
        };
        color.trim()
    } else {
        return Ok(None);
    };
    let color = parse_entire(source, CssColor::parse)?.into_owned();
    Ok(Some(AuthoredGradientStop {
        color,
        source: source.to_owned(),
        occurrences: 1,
    }))
}

fn parse_gradient_angle_percentage<'i, 't>(
    input: &mut Parser<'i, 't>,
) -> Result<AnglePercentage, ParseError<'i>> {
    if input
        .try_parse(|input| -> Result<(), ParseError<'i>> {
            let location = input.current_source_location();
            let token = input.next()?;
            match token {
                Token::Number { value, .. } if *value == 0.0 => Ok(()),
                _ => Err(location.new_unexpected_token_error(token.clone())),
            }
        })
        .is_ok()
    {
        return Ok(AnglePercentage::Dimension(Angle::Deg(0.0)));
    }
    AnglePercentage::parse(input)
}

fn parse_gradient_length_percentage<'i, 't>(
    input: &mut Parser<'i, 't>,
) -> Result<LengthPercentage, ParseError<'i>> {
    if input
        .try_parse(|input| -> Result<(), ParseError<'i>> {
            let location = input.current_source_location();
            let token = input.next()?;
            match token {
                Token::Number { value, .. } if *value == 0.0 => Ok(()),
                _ => Err(location.new_unexpected_token_error(token.clone())),
            }
        })
        .is_ok()
    {
        return Ok(LengthPercentage::Dimension(LengthValue::Px(0.0)));
    }
    LengthPercentage::parse(input)
}

fn parse_border_shape(source: &str) -> Result<BorderShapeValue, EngineError> {
    validate_strict_shape_numbers(source)?;
    parse_entire(source, |input| {
        if input
            .try_parse(|input| input.expect_ident_matching("none"))
            .is_ok()
        {
            return Ok(BorderShapeValue::None);
        }
        let outer = parse_border_shape_item(input)?;
        let mut inner = input.try_parse(parse_border_shape_item).ok().map(Box::new);
        if inner.as_ref().is_some_and(|inner| {
            outer.geometry_box.is_some() && inner.geometry_box.is_some() && outer == **inner
        }) {
            inner = None;
        }
        Ok(BorderShapeValue::Shapes {
            outer: Box::new(outer),
            inner,
        })
    })
}

fn parse_border_shape_item<'i, 't>(
    input: &mut Parser<'i, 't>,
) -> Result<BorderShapeItem, ParseError<'i>> {
    Ok(BorderShapeItem {
        shape: parse_basic_shape(input)?,
        geometry_box: input.try_parse(parse_border_geometry_box).ok(),
    })
}

fn parse_basic_shape<'i, 't>(
    input: &mut Parser<'i, 't>,
) -> Result<BasicShapeValue, ParseError<'i>> {
    let location = input.current_source_location();
    let function = input.expect_function()?.clone();
    input.parse_nested_block(|input| {
        if function.eq_ignore_ascii_case("inset") {
            return parse_inset(input).map(|value| BasicShapeValue::Inset(Box::new(value)));
        }
        if function.eq_ignore_ascii_case("circle") {
            return parse_circle(input).map(|value| BasicShapeValue::Circle(Box::new(value)));
        }
        if function.eq_ignore_ascii_case("ellipse") {
            return parse_ellipse(input).map(|value| BasicShapeValue::Ellipse(Box::new(value)));
        }
        if function.eq_ignore_ascii_case("polygon") {
            return parse_polygon(input).map(|value| BasicShapeValue::Polygon(Box::new(value)));
        }
        if function.eq_ignore_ascii_case("rect") {
            return parse_rect(input).map(|value| BasicShapeValue::Rect(Box::new(value)));
        }
        if function.eq_ignore_ascii_case("xywh") {
            return parse_xywh(input).map(|value| BasicShapeValue::Xywh(Box::new(value)));
        }
        if function.eq_ignore_ascii_case("path") {
            return parse_basic_shape_path(input)
                .map(|value| BasicShapeValue::Path(Box::new(value)));
        }
        if function.eq_ignore_ascii_case("shape") {
            return parse_shape_function(input)
                .map(|value| BasicShapeValue::Shape(Box::new(value)));
        }
        Err(location.new_unexpected_token_error(Token::Function(function)))
    })
}

fn parse_inset<'i, 't>(input: &mut Parser<'i, 't>) -> Result<InsetShape, ParseError<'i>> {
    let sides = parse_one_to_four(input, parse_shape_length)?;
    let radii = parse_optional_radii(input)?;
    Ok(InsetShape { sides, radii })
}

fn parse_circle<'i, 't>(input: &mut Parser<'i, 't>) -> Result<CircleShape, ParseError<'i>> {
    let radius = input
        .try_parse(parse_shape_radius)
        .unwrap_or(ShapeRadius::ClosestSide);
    let position_explicit = input
        .try_parse(|input| input.expect_ident_matching("at"))
        .is_ok();
    let position = if position_explicit {
        Position::parse(input)?
    } else {
        Position::center()
    };
    Ok(CircleShape {
        radius,
        position,
        position_explicit,
    })
}

fn parse_ellipse<'i, 't>(input: &mut Parser<'i, 't>) -> Result<EllipseShape, ParseError<'i>> {
    let radii = input.try_parse(
        |input| -> Result<(ShapeRadius, ShapeRadius), ParseError<'i>> {
            Ok((parse_shape_radius(input)?, parse_shape_radius(input)?))
        },
    );
    let (radius_x, radius_y) =
        radii.unwrap_or((ShapeRadius::ClosestSide, ShapeRadius::ClosestSide));
    let position_explicit = input
        .try_parse(|input| input.expect_ident_matching("at"))
        .is_ok();
    let position = if position_explicit {
        Position::parse(input)?
    } else {
        Position::center()
    };
    Ok(EllipseShape {
        radius_x,
        radius_y,
        position,
        position_explicit,
    })
}

fn parse_polygon<'i, 't>(input: &mut Parser<'i, 't>) -> Result<PolygonShape, ParseError<'i>> {
    let fill_rule = input
        .try_parse(|input| -> Result<FillRule, ParseError<'i>> {
            let fill_rule = parse_fill_rule(input)?;
            input.expect_comma()?;
            Ok(fill_rule)
        })
        .unwrap_or_default();
    let points = input.parse_comma_separated(parse_coordinate_pair)?;
    if points.is_empty() {
        return Err(invalid(input));
    }
    Ok(PolygonShape { fill_rule, points })
}

fn parse_rect<'i, 't>(input: &mut Parser<'i, 't>) -> Result<RectShape, ParseError<'i>> {
    let sides = [
        parse_rect_side(input)?,
        parse_rect_side(input)?,
        parse_rect_side(input)?,
        parse_rect_side(input)?,
    ];
    let radii = parse_optional_radii(input)?;
    Ok(RectShape { sides, radii })
}

fn parse_xywh<'i, 't>(input: &mut Parser<'i, 't>) -> Result<XywhShape, ParseError<'i>> {
    let x = parse_shape_length(input)?;
    let y = parse_shape_length(input)?;
    let width = parse_non_negative_shape_length(input)?;
    let height = parse_non_negative_shape_length(input)?;
    let radii = parse_optional_radii(input)?;
    Ok(XywhShape {
        x,
        y,
        width,
        height,
        radii,
    })
}

fn parse_basic_shape_path<'i, 't>(
    input: &mut Parser<'i, 't>,
) -> Result<BasicShapePath, ParseError<'i>> {
    let fill_rule = input
        .try_parse(|input| -> Result<FillRule, ParseError<'i>> {
            let fill_rule = parse_fill_rule(input)?;
            input.expect_comma()?;
            Ok(fill_rule)
        })
        .unwrap_or_default();
    let path = parse_svg_path_string(input)?;
    if path.commands.is_empty() {
        return Err(invalid(input));
    }
    Ok(BasicShapePath { fill_rule, path })
}

fn parse_fill_rule<'i, 't>(input: &mut Parser<'i, 't>) -> Result<FillRule, ParseError<'i>> {
    let location = input.current_source_location();
    let value = input.expect_ident_cloned()?;
    if value.eq_ignore_ascii_case("evenodd") {
        return Ok(FillRule::Evenodd);
    }
    if value.eq_ignore_ascii_case("nonzero") {
        return Ok(FillRule::Nonzero);
    }
    Err(location.new_unexpected_token_error(Token::Ident(value)))
}

fn parse_shape_radius<'i, 't>(input: &mut Parser<'i, 't>) -> Result<ShapeRadius, ParseError<'i>> {
    if input
        .try_parse(|input| input.expect_ident_matching("closest-side"))
        .is_ok()
    {
        return Ok(ShapeRadius::ClosestSide);
    }
    if input
        .try_parse(|input| input.expect_ident_matching("farthest-side"))
        .is_ok()
    {
        return Ok(ShapeRadius::FarthestSide);
    }
    parse_non_negative_shape_length(input).map(ShapeRadius::Length)
}

fn parse_rect_side<'i, 't>(input: &mut Parser<'i, 't>) -> Result<RectSide, ParseError<'i>> {
    if input
        .try_parse(|input| input.expect_ident_matching("auto"))
        .is_ok()
    {
        return Ok(RectSide::Auto);
    }
    parse_shape_length(input).map(RectSide::Length)
}

fn parse_coordinate_pair<'i, 't>(
    input: &mut Parser<'i, 't>,
) -> Result<CoordinatePair, ParseError<'i>> {
    Ok(CoordinatePair {
        x: parse_shape_length(input)?,
        y: parse_shape_length(input)?,
    })
}

fn parse_shape_length<'i, 't>(input: &mut Parser<'i, 't>) -> Result<ShapeLength, ParseError<'i>> {
    reject_current_nonzero_number(input)?;
    let authored_math = current_token_is_math_function(input)?;
    let value = LengthPercentage::parse(input)?;
    Ok(ShapeLength {
        value,
        authored_math,
    })
}

fn parse_non_negative_shape_length<'i, 't>(
    input: &mut Parser<'i, 't>,
) -> Result<ShapeLength, ParseError<'i>> {
    let value = parse_shape_length(input)?;
    if !value.authored_math && value.value.try_sign().is_some_and(|sign| sign < 0.0) {
        return Err(invalid(input));
    }
    Ok(value)
}

fn parse_optional_radii<'i, 't>(
    input: &mut Parser<'i, 't>,
) -> Result<Option<ShapeRadii>, ParseError<'i>> {
    if input
        .try_parse(|input| input.expect_ident_matching("round"))
        .is_err()
    {
        return Ok(None);
    }
    let horizontal = parse_one_to_four(input, parse_non_negative_shape_length)?;
    let vertical = if input.try_parse(|input| input.expect_delim('/')).is_ok() {
        parse_one_to_four(input, parse_non_negative_shape_length)?
    } else {
        horizontal.clone()
    };
    Ok(Some(ShapeRadii {
        horizontal,
        vertical,
    }))
}

fn parse_one_to_four<'i, 't, T, F>(
    input: &mut Parser<'i, 't>,
    mut parse: F,
) -> Result<[T; 4], ParseError<'i>>
where
    T: Clone,
    F: FnMut(&mut Parser<'i, 't>) -> Result<T, ParseError<'i>>,
{
    let first = parse(input)?;
    let second = input.try_parse(&mut parse).ok();
    let third = input.try_parse(&mut parse).ok();
    let fourth = input.try_parse(&mut parse).ok();
    Ok(match (second, third, fourth) {
        (None, None, None) => [first.clone(), first.clone(), first.clone(), first],
        (Some(second), None, None) => [first.clone(), second.clone(), first, second],
        (Some(second), Some(third), None) => [first, second.clone(), third, second],
        (Some(second), Some(third), Some(fourth)) => [first, second, third, fourth],
        _ => return Err(invalid(input)),
    })
}

fn parse_shape_box<'i, 't>(input: &mut Parser<'i, 't>) -> Result<&'static str, ParseError<'i>> {
    parse_keyword(
        input,
        &["content-box", "padding-box", "border-box", "margin-box"],
    )
}

fn parse_border_geometry_box<'i, 't>(
    input: &mut Parser<'i, 't>,
) -> Result<&'static str, ParseError<'i>> {
    parse_keyword(
        input,
        &[
            "border-box",
            "padding-box",
            "content-box",
            "margin-box",
            "fill-box",
            "stroke-box",
            "view-box",
            "half-border-box",
        ],
    )
}

fn parse_clip_geometry_box<'i, 't>(
    input: &mut Parser<'i, 't>,
) -> Result<&'static str, ParseError<'i>> {
    parse_keyword(
        input,
        &[
            "border-box",
            "padding-box",
            "content-box",
            "margin-box",
            "fill-box",
            "stroke-box",
            "view-box",
        ],
    )
}

fn parse_keyword<'i, 't>(
    input: &mut Parser<'i, 't>,
    accepted: &'static [&'static str],
) -> Result<&'static str, ParseError<'i>> {
    let location = input.current_source_location();
    let value = input.expect_ident_cloned()?;
    accepted
        .iter()
        .copied()
        .find(|candidate| value.eq_ignore_ascii_case(candidate))
        .ok_or_else(|| location.new_unexpected_token_error(Token::Ident(value)))
}

fn parse_svg_path_string<'i, 't>(
    input: &mut Parser<'i, 't>,
) -> Result<SvgPathData, ParseError<'i>> {
    let value = CSSString::parse(input)?;
    SvgPathData::parse(&value.0).map_err(|()| invalid(input))
}

impl SvgPathData {
    pub(crate) fn parse(source: &str) -> Result<Self, ()> {
        let mut commands = Vec::new();
        for segment in PathParser::from(source) {
            commands.push(SvgPathCommand::from_segment(segment.map_err(|_| ())?));
        }
        Ok(Self { commands })
    }
}

impl SvgPathCommand {
    fn from_segment(segment: PathSegment) -> Self {
        match segment {
            PathSegment::MoveTo { abs, x, y } => Self::MoveTo {
                absolute: abs,
                x,
                y,
            },
            PathSegment::LineTo { abs, x, y } => Self::LineTo {
                absolute: abs,
                x,
                y,
            },
            PathSegment::HorizontalLineTo { abs, x } => Self::HorizontalLineTo { absolute: abs, x },
            PathSegment::VerticalLineTo { abs, y } => Self::VerticalLineTo { absolute: abs, y },
            PathSegment::CurveTo {
                abs,
                x1,
                y1,
                x2,
                y2,
                x,
                y,
            } => Self::Cubic {
                absolute: abs,
                x1,
                y1,
                x2,
                y2,
                x,
                y,
            },
            PathSegment::SmoothCurveTo { abs, x2, y2, x, y } => Self::SmoothCubic {
                absolute: abs,
                x2,
                y2,
                x,
                y,
            },
            PathSegment::Quadratic { abs, x1, y1, x, y } => Self::Quadratic {
                absolute: abs,
                x1,
                y1,
                x,
                y,
            },
            PathSegment::SmoothQuadratic { abs, x, y } => Self::SmoothQuadratic {
                absolute: abs,
                x,
                y,
            },
            PathSegment::EllipticalArc {
                abs,
                rx,
                ry,
                x_axis_rotation,
                large_arc,
                sweep,
                x,
                y,
            } => Self::Arc {
                absolute: abs,
                rx,
                ry,
                rotation: x_axis_rotation,
                large: large_arc,
                sweep,
                x,
                y,
            },
            PathSegment::ClosePath { .. } => Self::Close,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ShapeFunction {
    fill_rule: FillRule,
    origin: Position,
    commands: Vec<ShapeCommand>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ShapeCommand {
    Move(ShapeEndpoint),
    Line(ShapeEndpoint),
    Hline(AxisEndpoint<ShapeHorizontalPosition>),
    Vline(AxisEndpoint<ShapeVerticalPosition>),
    Curve {
        endpoint: ShapeEndpoint,
        controls: Vec<ShapeControlPoint>,
    },
    Smooth {
        endpoint: ShapeEndpoint,
        control: Option<ShapeControlPoint>,
    },
    Arc {
        endpoint: ShapeEndpoint,
        radius_x: ShapeLength,
        radius_y: ShapeLength,
        single_radius: bool,
        clockwise: bool,
        large: bool,
        angle: Option<Angle>,
    },
    Close,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ShapeHorizontalPosition {
    XStart,
    XEnd,
    Component(HorizontalPosition),
}

#[derive(Clone, Debug, PartialEq)]
pub enum ShapeVerticalPosition {
    YStart,
    YEnd,
    Component(VerticalPosition),
}

#[derive(Clone, Debug, PartialEq)]
pub enum ShapeEndpoint {
    To(Position),
    By(CoordinatePair),
}

#[derive(Clone, Debug, PartialEq)]
pub enum AxisEndpoint<T> {
    To(T),
    By(ShapeLength),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ControlPointOrigin {
    Start,
    End,
    Origin,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ShapeControlPoint {
    To {
        position: Position,
        origin: ControlPointOrigin,
    },
    By {
        coordinates: CoordinatePair,
        origin: ControlPointOrigin,
    },
}

fn parse_shape_function<'i, 't>(
    input: &mut Parser<'i, 't>,
) -> Result<ShapeFunction, ParseError<'i>> {
    let fill_rule = input.try_parse(parse_fill_rule).unwrap_or_default();
    input.expect_ident_matching("from")?;
    let origin = Position::parse(input)?;
    input.expect_comma()?;

    let mut commands = Vec::new();
    while !input.is_exhausted() {
        commands.push(parse_shape_command(input)?);
        if input.is_exhausted() {
            break;
        }
        input.expect_comma()?;
    }
    if commands.is_empty() {
        return Err(invalid(input));
    }
    Ok(ShapeFunction {
        fill_rule,
        origin,
        commands,
    })
}

fn parse_shape_command<'i, 't>(input: &mut Parser<'i, 't>) -> Result<ShapeCommand, ParseError<'i>> {
    let location = input.current_source_location();
    let command = input.expect_ident_cloned()?;
    if command.eq_ignore_ascii_case("move") {
        return parse_shape_endpoint(input).map(ShapeCommand::Move);
    }
    if command.eq_ignore_ascii_case("line") {
        return parse_shape_endpoint(input).map(ShapeCommand::Line);
    }
    if command.eq_ignore_ascii_case("hline") {
        return parse_axis_endpoint(input, parse_shape_horizontal_position)
            .map(ShapeCommand::Hline);
    }
    if command.eq_ignore_ascii_case("vline") {
        return parse_axis_endpoint(input, parse_shape_vertical_position).map(ShapeCommand::Vline);
    }
    if command.eq_ignore_ascii_case("curve") {
        return parse_curve_command(input);
    }
    if command.eq_ignore_ascii_case("smooth") {
        return parse_smooth_command(input);
    }
    if command.eq_ignore_ascii_case("arc") {
        return parse_arc_command(input);
    }
    if command.eq_ignore_ascii_case("close") {
        return Ok(ShapeCommand::Close);
    }
    Err(location.new_unexpected_token_error(Token::Ident(command)))
}

fn parse_shape_horizontal_position<'i, 't>(
    input: &mut Parser<'i, 't>,
) -> Result<ShapeHorizontalPosition, ParseError<'i>> {
    if input
        .try_parse(|input| input.expect_ident_matching("x-start"))
        .is_ok()
    {
        return Ok(ShapeHorizontalPosition::XStart);
    }
    if input
        .try_parse(|input| input.expect_ident_matching("x-end"))
        .is_ok()
    {
        return Ok(ShapeHorizontalPosition::XEnd);
    }
    let value = HorizontalPosition::parse(input)?;
    reject_axis_side_offset(input, &value)?;
    Ok(ShapeHorizontalPosition::Component(value))
}

fn parse_shape_vertical_position<'i, 't>(
    input: &mut Parser<'i, 't>,
) -> Result<ShapeVerticalPosition, ParseError<'i>> {
    if input
        .try_parse(|input| input.expect_ident_matching("y-start"))
        .is_ok()
    {
        return Ok(ShapeVerticalPosition::YStart);
    }
    if input
        .try_parse(|input| input.expect_ident_matching("y-end"))
        .is_ok()
    {
        return Ok(ShapeVerticalPosition::YEnd);
    }
    let value = VerticalPosition::parse(input)?;
    reject_axis_side_offset(input, &value)?;
    Ok(ShapeVerticalPosition::Component(value))
}

fn reject_axis_side_offset<'i, 't, S>(
    input: &Parser<'i, 't>,
    value: &PositionComponent<S>,
) -> Result<(), ParseError<'i>> {
    if matches!(
        value,
        PositionComponent::Side {
            offset: Some(_),
            ..
        }
    ) {
        return Err(invalid(input));
    }
    Ok(())
}

fn parse_shape_endpoint<'i, 't>(
    input: &mut Parser<'i, 't>,
) -> Result<ShapeEndpoint, ParseError<'i>> {
    let location = input.current_source_location();
    let origin = input.expect_ident_cloned()?;
    if origin.eq_ignore_ascii_case("to") {
        return Position::parse(input).map(ShapeEndpoint::To);
    }
    if origin.eq_ignore_ascii_case("by") {
        return parse_coordinate_pair(input).map(ShapeEndpoint::By);
    }
    Err(location.new_unexpected_token_error(Token::Ident(origin)))
}

fn parse_axis_endpoint<'i, 't, T, F>(
    input: &mut Parser<'i, 't>,
    parse_to: F,
) -> Result<AxisEndpoint<T>, ParseError<'i>>
where
    F: FnOnce(&mut Parser<'i, 't>) -> Result<T, ParseError<'i>>,
{
    let location = input.current_source_location();
    let origin = input.expect_ident_cloned()?;
    if origin.eq_ignore_ascii_case("to") {
        return parse_to(input).map(AxisEndpoint::To);
    }
    if origin.eq_ignore_ascii_case("by") {
        return parse_shape_length(input).map(AxisEndpoint::By);
    }
    Err(location.new_unexpected_token_error(Token::Ident(origin)))
}

fn parse_curve_command<'i, 't>(input: &mut Parser<'i, 't>) -> Result<ShapeCommand, ParseError<'i>> {
    let endpoint = parse_shape_endpoint(input)?;
    input.expect_ident_matching("with")?;
    let first = parse_control_point(input, endpoint.is_relative())?;
    let mut controls = vec![first];
    if input.try_parse(|input| input.expect_delim('/')).is_ok() {
        controls.push(parse_control_point(input, endpoint.is_relative())?);
    }
    Ok(ShapeCommand::Curve { endpoint, controls })
}

fn parse_smooth_command<'i, 't>(
    input: &mut Parser<'i, 't>,
) -> Result<ShapeCommand, ParseError<'i>> {
    let endpoint = parse_shape_endpoint(input)?;
    let control = if input
        .try_parse(|input| input.expect_ident_matching("with"))
        .is_ok()
    {
        Some(parse_control_point(input, endpoint.is_relative())?)
    } else {
        None
    };
    Ok(ShapeCommand::Smooth { endpoint, control })
}

fn parse_control_point<'i, 't>(
    input: &mut Parser<'i, 't>,
    relative: bool,
) -> Result<ShapeControlPoint, ParseError<'i>> {
    let value = if relative {
        ShapeControlPoint::By {
            coordinates: parse_coordinate_pair(input)?,
            origin: ControlPointOrigin::Start,
        }
    } else {
        let position = Position::parse(input)?;
        if position_has_identifier_component(&position) {
            return Ok(ShapeControlPoint::To {
                position,
                origin: ControlPointOrigin::Origin,
            });
        }
        ShapeControlPoint::To {
            position,
            origin: ControlPointOrigin::Origin,
        }
    };
    if input
        .try_parse(|input| input.expect_ident_matching("from"))
        .is_err()
    {
        return Ok(value);
    }
    let origin = parse_control_point_origin(input)?;
    Ok(match value {
        ShapeControlPoint::To { position, .. } => ShapeControlPoint::To { position, origin },
        ShapeControlPoint::By { coordinates, .. } => ShapeControlPoint::By {
            coordinates,
            origin,
        },
    })
}

fn position_has_identifier_component(position: &Position) -> bool {
    position_component_is_identifier(&position.x) || position_component_is_identifier(&position.y)
}

fn position_component_is_identifier<S>(component: &PositionComponent<S>) -> bool {
    matches!(
        component,
        PositionComponent::Center | PositionComponent::Side { offset: None, .. }
    )
}

fn parse_control_point_origin<'i, 't>(
    input: &mut Parser<'i, 't>,
) -> Result<ControlPointOrigin, ParseError<'i>> {
    let location = input.current_source_location();
    let value = input.expect_ident_cloned()?;
    if value.eq_ignore_ascii_case("start") {
        return Ok(ControlPointOrigin::Start);
    }
    if value.eq_ignore_ascii_case("end") {
        return Ok(ControlPointOrigin::End);
    }
    if value.eq_ignore_ascii_case("origin") {
        return Ok(ControlPointOrigin::Origin);
    }
    Err(location.new_unexpected_token_error(Token::Ident(value)))
}

fn parse_arc_command<'i, 't>(input: &mut Parser<'i, 't>) -> Result<ShapeCommand, ParseError<'i>> {
    let endpoint = parse_shape_endpoint(input)?;
    let mut radius = None;
    let mut clockwise = false;
    let mut has_sweep = false;
    let mut large = false;
    let mut has_size = false;
    let mut angle = None;

    while !input.is_exhausted() {
        let state = input.state();
        let at_comma = matches!(input.next(), Ok(Token::Comma));
        input.reset(&state);
        if at_comma {
            break;
        }
        if input
            .try_parse(|input| input.expect_ident_matching("of"))
            .is_ok()
        {
            if radius.is_some() {
                return Err(invalid(input));
            }
            let x = parse_shape_length(input)?;
            let y = input.try_parse(parse_shape_length).ok();
            radius = Some((x, y));
            continue;
        }
        if input
            .try_parse(|input| input.expect_ident_matching("cw"))
            .is_ok()
        {
            if has_sweep {
                return Err(invalid(input));
            }
            clockwise = true;
            has_sweep = true;
            continue;
        }
        if input
            .try_parse(|input| input.expect_ident_matching("ccw"))
            .is_ok()
        {
            if has_sweep {
                return Err(invalid(input));
            }
            has_sweep = true;
            continue;
        }
        if input
            .try_parse(|input| input.expect_ident_matching("large"))
            .is_ok()
        {
            if has_size {
                return Err(invalid(input));
            }
            large = true;
            has_size = true;
            continue;
        }
        if input
            .try_parse(|input| input.expect_ident_matching("small"))
            .is_ok()
        {
            if has_size {
                return Err(invalid(input));
            }
            has_size = true;
            continue;
        }
        if input
            .try_parse(|input| input.expect_ident_matching("rotate"))
            .is_ok()
        {
            if angle.is_some() {
                return Err(invalid(input));
            }
            angle = Some(Angle::parse_with_unitless_zero(input)?);
            continue;
        }
        break;
    }

    let Some((radius_x, radius_y)) = radius else {
        return Err(invalid(input));
    };
    let single_radius = radius_y.is_none();
    let radius_y = radius_y.unwrap_or_else(|| radius_x.clone());
    let angle = angle.filter(|value| value.to_degrees() != 0.0);
    Ok(ShapeCommand::Arc {
        endpoint,
        radius_x,
        radius_y,
        single_radius,
        clockwise,
        large,
        angle,
    })
}

impl ShapeEndpoint {
    fn is_relative(&self) -> bool {
        matches!(self, Self::By(_))
    }
}

impl GeometricValue {
    pub(crate) fn canonical_value(&self) -> Result<String, EngineError> {
        match self {
            Self::BorderShape(value) => value.canonical_value(),
            Self::ClipPath(value) => value.canonical_value(),
            Self::D(value) => value.canonical_value(),
            Self::ObjectViewBox(value) => value.canonical_value(),
            Self::ShapeOutside(value) => value.canonical_value(),
        }
    }

    pub(crate) fn image_set_observable_value(&self) -> Result<Option<String>, EngineError> {
        let Self::ShapeOutside(value) = self else {
            return Ok(None);
        };
        let ShapeOutsideValue::Image {
            value: Image::ImageSet(image_set),
            ..
        } = value.as_ref()
        else {
            return Ok(None);
        };

        let mut output = String::from("image-set(");
        for (index, option) in image_set.options.iter().enumerate() {
            if index > 0 {
                output.push_str(", ");
            }
            output.push_str(&serialize_typed(&option.image)?);
            output.push(' ');
            output.push_str(&serialize_typed(&option.resolution)?);
            if let Some(file_type) = &option.file_type {
                output.push_str(" type(");
                cssparser::serialize_string(file_type, &mut output)
                    .map_err(|error| EngineError::Serialize(error.to_string()))?;
                output.push(')');
            }
        }
        output.push(')');
        Ok(Some(output))
    }

    pub(crate) fn gradient_observable_value(&self) -> Result<Option<String>, EngineError> {
        let Self::ShapeOutside(value) = self else {
            return Ok(None);
        };
        let ShapeOutsideValue::Image {
            value: Image::Gradient(gradient),
            authored_gradient: Some(authored),
        } = value.as_ref()
        else {
            return Ok(None);
        };
        authored.canonical_value(gradient).map(Some)
    }
}

impl ClipPathValue {
    fn canonical_value(&self) -> Result<String, EngineError> {
        let mut output = self.shape.canonical_value()?;
        if let Some(geometry_box) = self.geometry_box {
            output.push(' ');
            output.push_str(geometry_box);
        }
        Ok(output)
    }
}

impl PathPropertyValue {
    fn canonical_value(&self) -> Result<String, EngineError> {
        match self {
            Self::None => Ok("none".to_owned()),
            Self::Path(path) => Ok(format!("path({})", path.css_string()?)),
        }
    }
}

impl ObjectViewBoxValue {
    fn canonical_value(&self) -> Result<String, EngineError> {
        match self {
            Self::None => Ok("none".to_owned()),
            Self::Shape(shape) => shape.canonical_value(),
        }
    }
}

impl ShapeOutsideValue {
    fn canonical_value(&self) -> Result<String, EngineError> {
        match self {
            Self::Image {
                value: Image::Gradient(gradient),
                authored_gradient: Some(authored),
            } => authored.canonical_value(gradient),
            Self::Image { value, .. } => serialize_typed(value),
            Self::Box(value) => Ok((*value).to_owned()),
            Self::Shape {
                shape,
                geometry_box,
            } => {
                let mut output = shape.canonical_value()?;
                if let Some(geometry_box) = geometry_box {
                    output.push(' ');
                    output.push_str(geometry_box);
                }
                Ok(output)
            }
        }
    }
}

impl AuthoredGradient {
    fn canonical_value(&self, gradient: &Gradient) -> Result<String, EngineError> {
        let (name, prefix) = match gradient {
            Gradient::Linear(value) => ("linear-gradient", serialize_typed(&value.vendor_prefix)?),
            Gradient::RepeatingLinear(value) => (
                "repeating-linear-gradient",
                serialize_typed(&value.vendor_prefix)?,
            ),
            Gradient::Radial(value) => ("radial-gradient", serialize_typed(&value.vendor_prefix)?),
            Gradient::RepeatingRadial(value) => (
                "repeating-radial-gradient",
                serialize_typed(&value.vendor_prefix)?,
            ),
            Gradient::Conic(_) => ("conic-gradient", String::new()),
            Gradient::RepeatingConic(_) => ("repeating-conic-gradient", String::new()),
            Gradient::WebKitGradient(_) => return serialize_webkit_gradient(gradient, self),
        };
        let mut output = prefix;
        output.push_str(name);
        output.push('(');
        match gradient {
            Gradient::Linear(value) | Gradient::RepeatingLinear(value) => {
                append_linear_gradient(value, self, &mut output)?;
            }
            Gradient::Radial(value) | Gradient::RepeatingRadial(value) => {
                append_radial_gradient(value, self, &mut output)?;
            }
            Gradient::Conic(value) | Gradient::RepeatingConic(value) => {
                append_conic_gradient(value, self, &mut output)?;
            }
            Gradient::WebKitGradient(_) => unreachable!(),
        }
        output.push(')');
        Ok(output)
    }
}

fn serialize_webkit_gradient(
    gradient: &Gradient,
    authored: &AuthoredGradient,
) -> Result<String, EngineError> {
    let serialized = serialize_typed(gradient)?;
    let open = serialized
        .find('(')
        .ok_or_else(|| EngineError::Serialize("legacy gradient has no body".to_owned()))?;
    let close = serialized
        .rfind(')')
        .ok_or_else(|| EngineError::Serialize("legacy gradient has no closing token".to_owned()))?;
    let body = serialized
        .get(open + 1..close)
        .ok_or_else(|| EngineError::Serialize("legacy gradient body is invalid".to_owned()))?;
    let segments = split_top_level_delimiter(body, b',')
        .ok_or_else(|| EngineError::Serialize("legacy gradient items are invalid".to_owned()))?;
    let mut stop_index = 0usize;
    let mut output_segments = Vec::with_capacity(segments.len());
    for segment in segments {
        let trimmed = segment.trim();
        let Some(open) = trimmed.find('(') else {
            output_segments.push(trimmed.to_owned());
            continue;
        };
        let function = trimmed[..open].trim();
        if !matches_ignore_ascii_case(function, &["from", "to", "color-stop"]) {
            output_segments.push(trimmed.to_owned());
            continue;
        }
        let authored_stop = authored.stops.get(stop_index).ok_or_else(|| {
            EngineError::Serialize("legacy gradient lost authored color provenance".to_owned())
        })?;
        stop_index += 1;
        let replacement = if function.eq_ignore_ascii_case("color-stop") {
            let close = trimmed.rfind(')').ok_or_else(|| {
                EngineError::Serialize("legacy gradient stop is unclosed".to_owned())
            })?;
            let inner = trimmed.get(open + 1..close).ok_or_else(|| {
                EngineError::Serialize("legacy gradient stop body is invalid".to_owned())
            })?;
            let parts = split_top_level_delimiter(inner, b',').ok_or_else(|| {
                EngineError::Serialize("legacy gradient stop is malformed".to_owned())
            })?;
            let [position, _] = parts.as_slice() else {
                return Err(EngineError::Serialize(
                    "legacy gradient stop has invalid cardinality".to_owned(),
                ));
            };
            format!("color-stop({}, {})", position.trim(), authored_stop.source)
        } else {
            format!(
                "{}({})",
                function.to_ascii_lowercase(),
                authored_stop.source
            )
        };
        output_segments.push(replacement);
    }
    if stop_index != authored.stops.len() {
        return Err(EngineError::Serialize(
            "legacy gradient has unmatched authored color stops".to_owned(),
        ));
    }
    Ok(format!(
        "{}({})",
        &serialized[..open],
        output_segments.join(", ")
    ))
}

fn append_linear_gradient(
    gradient: &LinearGradient,
    authored: &AuthoredGradient,
    output: &mut String,
) -> Result<(), EngineError> {
    let direction = match &gradient.direction {
        LineDirection::Vertical(value) if serialize_typed(value)? == "bottom" => None,
        LineDirection::Angle(value) if value.to_degrees() == 180.0 => None,
        LineDirection::Angle(value) => Some(serialize_typed(value)?),
        LineDirection::Horizontal(value) => Some(format!("to {}", serialize_typed(value)?)),
        LineDirection::Vertical(value) => Some(format!("to {}", serialize_typed(value)?)),
        LineDirection::Corner {
            horizontal,
            vertical,
        } => Some(format!(
            "to {} {}",
            serialize_typed(vertical)?,
            serialize_typed(horizontal)?
        )),
    };
    let mut header = Vec::new();
    if let Some(direction) = direction {
        header.push(direction);
    }
    if let Some(interpolation) = authored.interpolation {
        header.push(interpolation.canonical_value());
    }
    if !header.is_empty() {
        output.push_str(&header.join(" "));
        output.push_str(", ");
    }
    output.push_str(&serialize_gradient_items(&gradient.items, authored)?);
    Ok(())
}

fn append_radial_gradient(
    gradient: &RadialGradient,
    authored: &AuthoredGradient,
    output: &mut String,
) -> Result<(), EngineError> {
    let default_shape = lightningcss::values::gradient::EndingShape::default();
    let has_shape = gradient.shape != default_shape;
    let mut header = Vec::new();
    if has_shape {
        header.push(serialize_typed(&gradient.shape)?);
    }
    let position = authored.position.as_ref().unwrap_or(&gradient.position);
    if !position.is_center() {
        header.push(format!("at {}", serialize_position(position)?));
    }
    if let Some(interpolation) = authored.interpolation {
        header.push(interpolation.canonical_value());
    }
    if !header.is_empty() {
        output.push_str(&header.join(" "));
        output.push_str(", ");
    }
    output.push_str(&serialize_gradient_items(&gradient.items, authored)?);
    Ok(())
}

fn append_conic_gradient(
    gradient: &ConicGradient,
    authored: &AuthoredGradient,
    output: &mut String,
) -> Result<(), EngineError> {
    let has_angle = gradient.angle.to_degrees() != 0.0;
    let mut header = Vec::new();
    if has_angle {
        header.push(format!("from {}", serialize_typed(&gradient.angle)?));
    }
    let position = authored.position.as_ref().unwrap_or(&gradient.position);
    if !position.is_center() {
        header.push(format!("at {}", serialize_position(position)?));
    }
    if let Some(interpolation) = authored.interpolation {
        header.push(interpolation.canonical_value());
    }
    if !header.is_empty() {
        output.push_str(&header.join(" "));
        output.push_str(", ");
    }
    output.push_str(&serialize_gradient_items(&gradient.items, authored)?);
    Ok(())
}

impl GradientInterpolation {
    fn canonical_value(self) -> String {
        let mut value = format!("in {}", self.color_space);
        if let Some(hue_method) = self.hue_method {
            value.push(' ');
            value.push_str(hue_method);
            value.push_str(" hue");
        }
        value
    }
}

trait GradientPosition {
    fn observable_value(&self) -> Result<String, EngineError>;
}

impl GradientPosition for LengthPercentage {
    fn observable_value(&self) -> Result<String, EngineError> {
        let value = serialize_typed(self)?;
        if value == "0" {
            return Ok("0px".to_owned());
        }
        Ok(value)
    }
}

impl GradientPosition for lightningcss::values::angle::AnglePercentage {
    fn observable_value(&self) -> Result<String, EngineError> {
        serialize_typed(self)
    }
}

fn serialize_gradient_items<D: GradientPosition>(
    items: &[GradientItem<D>],
    authored: &AuthoredGradient,
) -> Result<String, EngineError> {
    let expanded_sources = authored
        .stops
        .iter()
        .flat_map(|stop| std::iter::repeat_n(stop, stop.occurrences))
        .collect::<Vec<_>>();
    let mut color_index = 0usize;
    let mut values = Vec::with_capacity(items.len());
    for item in items {
        match item {
            GradientItem::Hint(value) => values.push(value.observable_value()?),
            GradientItem::ColorStop(stop) => {
                let authored = expanded_sources.get(color_index).ok_or_else(|| {
                    EngineError::Serialize("gradient lost authored color provenance".to_owned())
                })?;
                if authored.color != stop.color {
                    return Err(EngineError::Serialize(
                        "gradient color provenance does not match its semantic value".to_owned(),
                    ));
                }
                color_index += 1;
                let mut value = authored.source.clone();
                if let Some(position) = &stop.position {
                    value.push(' ');
                    value.push_str(&position.observable_value()?);
                }
                values.push(value);
            }
        }
    }
    if color_index != expanded_sources.len() {
        return Err(EngineError::Serialize(
            "gradient retained unused authored color provenance".to_owned(),
        ));
    }
    Ok(values.join(", "))
}

impl BorderShapeValue {
    fn canonical_value(&self) -> Result<String, EngineError> {
        match self {
            Self::None => Ok("none".to_owned()),
            Self::Shapes { outer, inner } => {
                let mut output = outer.canonical_value()?;
                if let Some(inner) = inner {
                    output.push(' ');
                    output.push_str(&inner.canonical_value()?);
                }
                Ok(output)
            }
        }
    }
}

impl BorderShapeItem {
    fn canonical_value(&self) -> Result<String, EngineError> {
        let mut output = self.shape.canonical_value()?;
        if let Some(geometry_box) = self.geometry_box {
            output.push(' ');
            output.push_str(geometry_box);
        }
        Ok(output)
    }
}

impl BasicShapeValue {
    fn canonical_value(&self) -> Result<String, EngineError> {
        match self {
            Self::Inset(value) => value.canonical_value(),
            Self::Circle(value) => value.canonical_value(),
            Self::Ellipse(value) => value.canonical_value(),
            Self::Polygon(value) => value.canonical_value(),
            Self::Rect(value) => value.canonical_value(),
            Self::Xywh(value) => value.canonical_value(),
            Self::Path(value) => value.canonical_value(),
            Self::Shape(value) => value.canonical_value(),
        }
    }
}

impl InsetShape {
    fn canonical_value(&self) -> Result<String, EngineError> {
        let mut body = serialize_four(&self.sides)?;
        append_radii(&mut body, &self.radii)?;
        Ok(format!("inset({body})"))
    }
}

impl CircleShape {
    fn canonical_value(&self) -> Result<String, EngineError> {
        let mut body = self.radius.canonical_value(true)?;
        if self.position_explicit {
            if !body.is_empty() {
                body.push(' ');
            }
            body.push_str("at ");
            body.push_str(&serialize_position(&self.position)?);
        }
        Ok(format!("circle({body})"))
    }
}

impl EllipseShape {
    fn canonical_value(&self) -> Result<String, EngineError> {
        let mut body = if self.radius_x == ShapeRadius::ClosestSide
            && self.radius_y == ShapeRadius::ClosestSide
        {
            String::new()
        } else {
            format!(
                "{} {}",
                self.radius_x.canonical_value(false)?,
                self.radius_y.canonical_value(false)?
            )
        };
        if self.position_explicit {
            if !body.is_empty() {
                body.push(' ');
            }
            body.push_str("at ");
            body.push_str(&serialize_position(&self.position)?);
        }
        Ok(format!("ellipse({body})"))
    }
}

impl PolygonShape {
    fn canonical_value(&self) -> Result<String, EngineError> {
        let mut values = Vec::with_capacity(self.points.len());
        for point in &self.points {
            values.push(point.canonical_value()?);
        }
        let prefix = if self.fill_rule == FillRule::Evenodd {
            "evenodd, "
        } else {
            ""
        };
        Ok(format!("polygon({prefix}{})", values.join(", ")))
    }
}

impl RectShape {
    fn canonical_value(&self) -> Result<String, EngineError> {
        let mut values = Vec::with_capacity(4);
        for side in &self.sides {
            values.push(match side {
                RectSide::Auto => "auto".to_owned(),
                RectSide::Length(value) => value.canonical_value()?,
            });
        }
        let mut body = values.join(" ");
        append_radii(&mut body, &self.radii)?;
        Ok(format!("rect({body})"))
    }
}

impl XywhShape {
    fn canonical_value(&self) -> Result<String, EngineError> {
        let mut body = format!(
            "{} {} {} {}",
            self.x.canonical_value()?,
            self.y.canonical_value()?,
            self.width.canonical_value()?,
            self.height.canonical_value()?
        );
        append_radii(&mut body, &self.radii)?;
        Ok(format!("xywh({body})"))
    }
}

impl BasicShapePath {
    fn canonical_value(&self) -> Result<String, EngineError> {
        let prefix = if self.fill_rule == FillRule::Evenodd {
            "evenodd, "
        } else {
            ""
        };
        Ok(format!("path({prefix}{})", self.path.css_string()?))
    }
}

impl ShapeFunction {
    fn canonical_value(&self) -> Result<String, EngineError> {
        let mut body = if self.fill_rule == FillRule::Evenodd {
            "evenodd from ".to_owned()
        } else {
            "from ".to_owned()
        };
        body.push_str(&serialize_position(&self.origin)?);
        for command in &self.commands {
            body.push_str(", ");
            body.push_str(&command.canonical_value()?);
        }
        Ok(format!("shape({body})"))
    }
}

impl ShapeCommand {
    fn canonical_value(&self) -> Result<String, EngineError> {
        match self {
            Self::Move(endpoint) => Ok(format!("move {}", endpoint.canonical_value()?)),
            Self::Line(endpoint) => Ok(format!("line {}", endpoint.canonical_value()?)),
            Self::Hline(endpoint) => Ok(format!("hline {}", endpoint.canonical_value()?)),
            Self::Vline(endpoint) => Ok(format!("vline {}", endpoint.canonical_value()?)),
            Self::Curve { endpoint, controls } => {
                let values = controls
                    .iter()
                    .map(|value| value.canonical_value(endpoint.is_relative()))
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(format!(
                    "curve {} with {}",
                    endpoint.canonical_value()?,
                    values.join(" / ")
                ))
            }
            Self::Smooth { endpoint, control } => {
                let mut output = format!("smooth {}", endpoint.canonical_value()?);
                if let Some(control) = control {
                    output.push_str(" with ");
                    output.push_str(&control.canonical_value(endpoint.is_relative())?);
                }
                Ok(output)
            }
            Self::Arc {
                endpoint,
                radius_x,
                radius_y,
                single_radius,
                clockwise,
                large,
                angle,
            } => {
                let mut output = format!(
                    "arc {} of {}",
                    endpoint.canonical_value()?,
                    radius_x.canonical_value()?
                );
                if !single_radius {
                    output.push(' ');
                    output.push_str(&radius_y.canonical_value()?);
                }
                if *clockwise {
                    output.push_str(" cw");
                }
                if *large {
                    output.push_str(" large");
                }
                if let Some(angle) = angle {
                    output.push_str(" rotate ");
                    output.push_str(&serialize_typed(angle)?);
                }
                Ok(output)
            }
            Self::Close => Ok("close".to_owned()),
        }
    }
}

impl ShapeEndpoint {
    fn canonical_value(&self) -> Result<String, EngineError> {
        match self {
            Self::To(value) => Ok(format!("to {}", serialize_position(value)?)),
            Self::By(value) => Ok(format!("by {}", value.canonical_value()?)),
        }
    }
}

pub(crate) trait AxisPositionValue {
    fn canonical_value(&self) -> Result<String, EngineError>;
}

impl<S: ToCss> AxisPositionValue for PositionComponent<S> {
    fn canonical_value(&self) -> Result<String, EngineError> {
        serialize_position_component(self)
    }
}

impl AxisPositionValue for ShapeHorizontalPosition {
    fn canonical_value(&self) -> Result<String, EngineError> {
        match self {
            Self::XStart => Ok("x-start".to_owned()),
            Self::XEnd => Ok("x-end".to_owned()),
            Self::Component(value) => serialize_position_component(value),
        }
    }
}

impl AxisPositionValue for ShapeVerticalPosition {
    fn canonical_value(&self) -> Result<String, EngineError> {
        match self {
            Self::YStart => Ok("y-start".to_owned()),
            Self::YEnd => Ok("y-end".to_owned()),
            Self::Component(value) => serialize_position_component(value),
        }
    }
}

impl<T: AxisPositionValue> AxisEndpoint<T> {
    fn canonical_value(&self) -> Result<String, EngineError> {
        match self {
            Self::To(value) => Ok(format!("to {}", value.canonical_value()?)),
            Self::By(value) => Ok(format!("by {}", value.canonical_value()?)),
        }
    }
}

impl ShapeControlPoint {
    fn canonical_value(&self, relative: bool) -> Result<String, EngineError> {
        let (mut output, origin) = match self {
            Self::To { position, origin } => (serialize_position(position)?, *origin),
            Self::By {
                coordinates,
                origin,
            } => (coordinates.canonical_value()?, *origin),
        };
        let default = if relative {
            ControlPointOrigin::Start
        } else {
            ControlPointOrigin::Origin
        };
        if origin != default {
            output.push_str(" from ");
            output.push_str(origin.as_str());
        }
        Ok(output)
    }
}

impl ControlPointOrigin {
    fn as_str(self) -> &'static str {
        match self {
            Self::Start => "start",
            Self::End => "end",
            Self::Origin => "origin",
        }
    }
}

impl ShapeRadius {
    fn canonical_value(&self, omit_closest: bool) -> Result<String, EngineError> {
        match self {
            Self::ClosestSide if omit_closest => Ok(String::new()),
            Self::ClosestSide => Ok("closest-side".to_owned()),
            Self::FarthestSide => Ok("farthest-side".to_owned()),
            Self::Length(value) => value.canonical_value(),
        }
    }
}

impl ShapeLength {
    fn canonical_value(&self) -> Result<String, EngineError> {
        let mut value = serialize_typed(&self.value)?;
        if self.authored_math && !starts_math_function(&value) {
            value = format!("calc({value})");
        }
        if value == "0" {
            return Ok("0px".to_owned());
        }
        Ok(value)
    }
}

impl CoordinatePair {
    fn canonical_value(&self) -> Result<String, EngineError> {
        Ok(format!(
            "{} {}",
            self.x.canonical_value()?,
            self.y.canonical_value()?
        ))
    }
}

impl SvgPathData {
    fn canonical_data(&self) -> String {
        self.commands
            .iter()
            .map(SvgPathCommand::canonical_value)
            .collect::<Vec<_>>()
            .join(" ")
    }

    pub(crate) fn css_string(&self) -> Result<String, EngineError> {
        let mut output = String::new();
        cssparser::serialize_string(&self.canonical_data(), &mut output)
            .map_err(|_| EngineError::Serialize("could not serialize SVG path".to_owned()))?;
        Ok(output)
    }
}

impl SvgPathCommand {
    fn canonical_value(&self) -> String {
        match self {
            Self::MoveTo { absolute, x, y } => svg_command(*absolute, 'M', 'm', &[*x, *y]),
            Self::LineTo { absolute, x, y } => svg_command(*absolute, 'L', 'l', &[*x, *y]),
            Self::HorizontalLineTo { absolute, x } => svg_command(*absolute, 'H', 'h', &[*x]),
            Self::VerticalLineTo { absolute, y } => svg_command(*absolute, 'V', 'v', &[*y]),
            Self::Cubic {
                absolute,
                x1,
                y1,
                x2,
                y2,
                x,
                y,
            } => svg_command(*absolute, 'C', 'c', &[*x1, *y1, *x2, *y2, *x, *y]),
            Self::SmoothCubic {
                absolute,
                x2,
                y2,
                x,
                y,
            } => svg_command(*absolute, 'S', 's', &[*x2, *y2, *x, *y]),
            Self::Quadratic {
                absolute,
                x1,
                y1,
                x,
                y,
            } => svg_command(*absolute, 'Q', 'q', &[*x1, *y1, *x, *y]),
            Self::SmoothQuadratic { absolute, x, y } => svg_command(*absolute, 'T', 't', &[*x, *y]),
            Self::Arc {
                absolute,
                rx,
                ry,
                rotation,
                large,
                sweep,
                x,
                y,
            } => {
                let letter = if *absolute { 'A' } else { 'a' };
                format!(
                    "{letter} {} {} {} {} {} {} {}",
                    svg_number(*rx),
                    svg_number(*ry),
                    svg_number(*rotation),
                    u8::from(*large),
                    u8::from(*sweep),
                    svg_number(*x),
                    svg_number(*y)
                )
            }
            Self::Close => "Z".to_owned(),
        }
    }
}

fn svg_command(absolute: bool, upper: char, lower: char, values: &[f64]) -> String {
    let command = if absolute { upper } else { lower };
    let values = values
        .iter()
        .map(|value| svg_number(*value))
        .collect::<Vec<_>>()
        .join(" ");
    format!("{command} {values}")
}

fn svg_number(value: f64) -> String {
    let value = value as f32;
    if value == 0.0 {
        return "0".to_owned();
    }

    let exponent = value.abs().log10().floor() as i32;
    if !(-4..6).contains(&exponent) {
        let mut output = format!("{value:.5e}");
        if let Some(index) = output.find('e') {
            if output
                .as_bytes()
                .get(index + 1)
                .is_some_and(u8::is_ascii_digit)
            {
                output.insert(index + 1, '+');
            }
        }
        return output;
    }

    let decimal_places = usize::try_from(5 - exponent).unwrap_or_default();
    let mut output = format!("{value:.decimal_places$}");
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

fn append_radii(output: &mut String, radii: &Option<ShapeRadii>) -> Result<(), EngineError> {
    let Some(radii) = radii else {
        return Ok(());
    };
    output.push_str(" round ");
    output.push_str(&serialize_four(&radii.horizontal)?);
    if radii.horizontal != radii.vertical {
        output.push_str(" / ");
        output.push_str(&serialize_four(&radii.vertical)?);
    }
    Ok(())
}

fn serialize_four(values: &[ShapeLength; 4]) -> Result<String, EngineError> {
    let length = if values[0] == values[1] && values[0] == values[2] && values[0] == values[3] {
        1
    } else if values[0] == values[2] && values[1] == values[3] {
        2
    } else if values[1] == values[3] {
        3
    } else {
        4
    };
    let mut output = Vec::with_capacity(length);
    for value in values.iter().take(length) {
        output.push(value.canonical_value()?);
    }
    Ok(output.join(" "))
}

fn serialize_position(position: &Position) -> Result<String, EngineError> {
    Ok(format!(
        "{} {}",
        serialize_position_component(&position.x)?,
        serialize_position_component(&position.y)?
    ))
}

fn serialize_position_component<S: ToCss>(
    value: &PositionComponent<S>,
) -> Result<String, EngineError> {
    match value {
        PositionComponent::Center => Ok("center".to_owned()),
        PositionComponent::Length(value) => ShapeLength {
            value: value.clone(),
            authored_math: false,
        }
        .canonical_value(),
        PositionComponent::Side { side, offset } => {
            let mut output = serialize_typed(side)?;
            if let Some(offset) = offset {
                output.push(' ');
                output.push_str(
                    &ShapeLength {
                        value: offset.clone(),
                        authored_math: false,
                    }
                    .canonical_value()?,
                );
            }
            Ok(output)
        }
    }
}

fn serialize_typed<T: ToCss>(value: &T) -> Result<String, EngineError> {
    value
        .to_css_string(PrinterOptions::default())
        .map_err(|error| EngineError::Serialize(error.to_string()))
}

fn validate_strict_shape_numbers(source: &str) -> Result<(), EngineError> {
    parse_entire(source, |input| validate_component_numbers(input, false))
}

fn validate_component_numbers<'i, 't>(
    input: &mut Parser<'i, 't>,
    math_context: bool,
) -> Result<(), ParseError<'i>> {
    while !input.is_exhausted() {
        let token = input.next()?.clone();
        if !math_context && matches!(token, Token::Number { value, .. } if value != 0.0) {
            return Err(invalid(input));
        }
        if let Token::Function(name) = token {
            let nested_math = math_context || is_math_function(&name);
            input.parse_nested_block(|input| validate_component_numbers(input, nested_math))?;
        } else if matches!(
            token,
            Token::ParenthesisBlock | Token::SquareBracketBlock | Token::CurlyBracketBlock
        ) {
            input.parse_nested_block(|input| validate_component_numbers(input, math_context))?;
        }
    }
    Ok(())
}

fn is_math_function(name: &str) -> bool {
    matches_ignore_ascii_case(
        name,
        &[
            "abs", "acos", "asin", "atan", "atan2", "calc", "clamp", "cos", "exp", "hypot", "log",
            "max", "min", "mod", "pow", "rem", "round", "sign", "sin", "sqrt", "tan",
        ],
    )
}

fn current_token_is_math_function<'i, 't>(
    input: &mut Parser<'i, 't>,
) -> Result<bool, ParseError<'i>> {
    let state = input.state();
    let token = input.next()?.clone();
    input.reset(&state);
    Ok(matches!(token, Token::Function(name) if is_math_function(&name)))
}

fn starts_math_function(value: &str) -> bool {
    let Some((name, _)) = value.split_once('(') else {
        return false;
    };
    is_math_function(name.trim())
}

fn matches_ignore_ascii_case(value: &str, candidates: &[&str]) -> bool {
    candidates
        .iter()
        .any(|candidate| value.eq_ignore_ascii_case(candidate))
}

fn reject_current_nonzero_number<'i, 't>(input: &mut Parser<'i, 't>) -> Result<(), ParseError<'i>> {
    let state = input.state();
    let token = input.next()?.clone();
    input.reset(&state);
    if matches!(token, Token::Number { value, .. } if value != 0.0) {
        return Err(invalid(input));
    }
    Ok(())
}

fn parse_entire<'i, T, F>(source: &'i str, parser: F) -> Result<T, EngineError>
where
    F: for<'t> FnOnce(&mut Parser<'i, 't>) -> Result<T, ParseError<'i>>,
{
    let mut input = ParserInput::new(source);
    let mut css = Parser::new(&mut input);
    css.parse_entirely(parser)
        .map_err(|_| EngineError::Parse("invalid geometric property value".to_owned()))
}

fn invalid<'i, 't>(input: &Parser<'i, 't>) -> ParseError<'i> {
    input.new_custom_error(ParserError::InvalidValue)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn canonical(property: &str, source: &str) -> Result<String, EngineError> {
        parse_geometric_property(property, source)?
            .ok_or_else(|| EngineError::Parse("missing geometric grammar".to_owned()))?
            .canonical_value()
    }

    #[test]
    fn canonicalizes_svg_path_commands_like_chromium() {
        assert_eq!(canonical("d", "none").unwrap(), "none");
        assert_eq!(canonical("d", "path(\"\")").unwrap(), "none");
        assert_eq!(
            canonical("d", "path(\"M10-20l30.1.5.1-20z\")").unwrap(),
            "path(\"M 10 -20 l 30.1 0.5 l 0.1 -20 Z\")"
        );
        assert_eq!(
            canonical("d", "path(\"M0 0 10 10 20 20\")").unwrap(),
            "path(\"M 0 0 L 10 10 L 20 20\")"
        );
        assert_eq!(
            canonical("d", "path(\"M.0000000000000000001 -.0\")").unwrap(),
            "path(\"M 1.00000e-19 0\")"
        );
        assert!(canonical("d", "path(\"M0 0 Z 1 2\")").is_err());
    }

    #[test]
    fn owns_modern_clip_path_shapes_and_geometry_boxes() {
        for (source, expected) in [
            ("path(\"M0 0\")", "path(\"M 0 0\")"),
            (
                "path(evenodd, \"M0 0 L10 10\")",
                "path(evenodd, \"M 0 0 L 10 10\")",
            ),
            (
                "content-box path(nonzero, \"M0 0\")",
                "path(\"M 0 0\") content-box",
            ),
            (
                "rect(auto 1px 20% -3px round 5px) padding-box",
                "rect(auto 1px 20% -3px round 5px) padding-box",
            ),
            (
                "xywh(0 0 10px 20px round 2px) fill-box",
                "xywh(0px 0px 10px 20px round 2px) fill-box",
            ),
            (
                "shape(from 0 0, line to 10px 20px) stroke-box",
                "shape(from 0px 0px, line to 10px 20px) stroke-box",
            ),
        ] {
            assert_eq!(
                canonical("clip-path", source).unwrap(),
                expected,
                "{source}"
            );
        }

        for source in [
            "path()",
            "path(\"M0\")",
            "half-border-box",
            "path(\"M0 0\") half-border-box",
            "content-box path(\"M0 0\") padding-box",
            "path(\"M0 0\"), content-box",
        ] {
            assert!(canonical("clip-path", source).is_err(), "{source}");
        }
    }

    #[test]
    fn restricts_object_view_box_to_rectangular_shapes() {
        for (source, expected) in [
            ("none", "none"),
            ("inset(1px)", "inset(1px)"),
            (
                "rect(auto 1px 20% -3px round 5px)",
                "rect(auto 1px 20% -3px round 5px)",
            ),
            (
                "xywh(-1px -2% 3px 4% round 5px / 10px)",
                "xywh(-1px -2% 3px 4% round 5px / 10px)",
            ),
        ] {
            assert_eq!(canonical("object-view-box", source).unwrap(), expected);
        }
        for source in ["circle()", "path(\"M0 0\")", "xywh(1px 2px -3px 4px)"] {
            assert!(canonical("object-view-box", source).is_err(), "{source}");
        }
    }

    #[test]
    fn owns_shape_commands_semantically() {
        for (source, expected) in [
            (
                "shape(from 0 0, line to 10px 10px)",
                "shape(from 0px 0px, line to 10px 10px)",
            ),
            (
                "shape(evenodd from center, move by 1px 2px, hline to right, vline by 3px, close)",
                "shape(evenodd from center center, move by 1px 2px, hline to right, vline by 3px, close)",
            ),
            (
                "shape(from 0 0, curve to 10px 20px with 1px 2px / 3px 4px)",
                "shape(from 0px 0px, curve to 10px 20px with 1px 2px / 3px 4px)",
            ),
            (
                "shape(from 0 0, arc to 10px 20px large of 5px 6px rotate 30deg cw)",
                "shape(from 0px 0px, arc to 10px 20px of 5px 6px cw large rotate 30deg)",
            ),
            (
                "shape(from 0 0, hline to x-start, hline to x-end, vline to y-start, vline to y-end)",
                "shape(from 0px 0px, hline to x-start, hline to x-end, vline to y-start, vline to y-end)",
            ),
            (
                "shape(from 0 0, curve to 1px 2px with left 3px top 4px from start)",
                "shape(from 0px 0px, curve to 1px 2px with left 3px top 4px from start)",
            ),
        ] {
            assert_eq!(canonical("shape-outside", source).unwrap(), expected);
        }

        for source in [
            "shape(from 0 0, hline to left 2px)",
            "shape(from 0 0, vline to top 2px)",
            "shape(from 0 0, curve to left top with center from start)",
            "shape(from 0 0, curve to left top with left top from end)",
        ] {
            assert!(canonical("shape-outside", source).is_err(), "{source}");
        }
    }

    #[test]
    fn applies_shape_outside_box_and_image_rules() {
        for (source, expected) in [
            ("margin-box", "margin-box"),
            ("circle() margin-box", "circle()"),
            ("margin-box circle()", "circle()"),
            ("circle() border-box", "circle() border-box"),
            ("linear-gradient(red,blue)", "linear-gradient(red, blue)"),
            (
                "repeating-conic-gradient(from .5turn, red 0 10deg, blue 10deg 20deg)",
                "repeating-conic-gradient(from .5turn, red 0deg, red 10deg, blue 10deg, blue 20deg)",
            ),
            (
                "-webkit-gradient(linear, left top, right bottom, from(red), to(blue))",
                "-webkit-gradient(linear, left top, right bottom, from(red), to(blue))",
            ),
            (
                "linear-gradient(in oklab to right, red 0 10%, 20%, blue)",
                "linear-gradient(to right in oklab, red 0px, red 10%, 20%, blue)",
            ),
            (
                "radial-gradient(ellipse farthest-corner at left top, red, blue)",
                "radial-gradient(at left top, red, blue)",
            ),
            (
                "conic-gradient(in oklab from 10deg at left top, red 0, blue .5turn)",
                "conic-gradient(from 10deg at left top in oklab, red 0deg, blue .5turn)",
            ),
        ] {
            assert_eq!(canonical("shape-outside", source).unwrap(), expected);
        }
        assert!(canonical("shape-outside", "circle() url(x)").is_err());
    }

    #[test]
    fn preserves_border_shape_pair_semantics() {
        for (source, expected) in [
            ("circle()", "circle()"),
            ("circle() circle()", "circle() circle()"),
            (
                "circle() border-box circle() padding-box",
                "circle() border-box circle() padding-box",
            ),
            (
                "circle() border-box circle() border-box",
                "circle() border-box",
            ),
            ("circle() half-border-box", "circle() half-border-box"),
        ] {
            assert_eq!(canonical("border-shape", source).unwrap(), expected);
        }
    }

    #[test]
    fn rejects_quirks_only_unitless_lengths_and_negative_sizes() {
        for (property, source) in [
            ("shape-outside", "circle(2)"),
            ("shape-outside", "shape(from 1 0, close)"),
            ("object-view-box", "inset(1)"),
            ("object-view-box", "xywh(0 0 -1px 1px)"),
        ] {
            assert!(canonical(property, source).is_err(), "{property}: {source}");
        }
    }

    #[test]
    fn retains_authored_math_and_allows_deferred_range_validation() {
        for (source, expected) in [
            ("circle(calc(1px + 2px))", "circle(calc(3px))"),
            ("circle(min(3px, 10%))", "circle(min(3px, 10%))"),
            ("circle(max(3px, 4px))", "circle(calc(4px))"),
            (
                "circle(clamp(1px, 2px, 10%))",
                "circle(clamp(1px, 2px, 10%))",
            ),
            (
                "xywh(0 0 calc(3px - 4px) 2px)",
                "xywh(0px 0px calc(-1px) 2px)",
            ),
        ] {
            assert_eq!(canonical("shape-outside", source).unwrap(), expected);
        }
    }

    #[test]
    fn distinguishes_explicit_center_positions_from_shape_defaults() {
        assert_eq!(
            canonical("shape-outside", "circle(at center)").unwrap(),
            "circle(at center center)"
        );
        assert_eq!(
            canonical("shape-outside", "ellipse(at center)").unwrap(),
            "ellipse(at center center)"
        );
        assert_eq!(canonical("shape-outside", "circle()").unwrap(), "circle()");
    }
}
