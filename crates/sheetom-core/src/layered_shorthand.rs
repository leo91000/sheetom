use crate::{
    EngineError, RecoveredComponentKind, RecoveredComponentValue, RecoveredToken,
    RecoveredTokenKind, RecoveredValue,
};
use lightningcss::{properties::Property, stylesheet::PrinterOptions, traits::ToCss};

#[derive(Clone, Copy, Default)]
struct LayerPresence {
    position: bool,
    size: bool,
}

/// Ephemeral projection of one parsed layered shorthand.
///
/// Semantic values come from Lightning's typed shorthand AST. For position
/// and size, the recovered component tree contributes only authored-component
/// presence, so compact separators and whitespace never participate in value
/// interpretation. It also identifies authored observable evidence that the
/// semantic printer cannot reproduce.
pub(crate) struct LayeredShorthandProjection<'a> {
    property: &'a Property<'static>,
    presence: Vec<LayerPresence>,
    authored_image_set_url: bool,
}

impl<'a> LayeredShorthandProjection<'a> {
    pub(crate) fn new(
        shorthand: &str,
        property: &'a Property<'static>,
        recovered: &RecoveredValue,
    ) -> Option<Self> {
        let layer_count = match (shorthand, property) {
            ("background", Property::Background(layers)) => layers.len(),
            ("mask" | "-webkit-mask", Property::Mask(layers, _)) => layers.len(),
            _ => return None,
        };
        let presence = layer_presence(recovered);
        (presence.len() == layer_count).then_some(Self {
            property,
            presence,
            authored_image_set_url: contains_authored_image_set_url(recovered.values()),
        })
    }

    pub(crate) fn observable_survives_group_break(&self, longhand: &str) -> bool {
        self.authored_image_set_url && matches!(longhand, "background-image" | "mask-image")
    }

    pub(crate) fn longhand(&self, name: &str) -> Option<Property<'a>> {
        match self.property {
            Property::Background(layers) => match name {
                "background-color" => Some(Property::BackgroundColor(layers.last()?.color.clone())),
                "background-image" => Some(Property::BackgroundImage(
                    layers.iter().map(|layer| layer.image.clone()).collect(),
                )),
                "background-position-x" => Some(Property::BackgroundPositionX(
                    layers
                        .iter()
                        .map(|layer| layer.position.x.clone())
                        .collect(),
                )),
                "background-position-y" => Some(Property::BackgroundPositionY(
                    layers
                        .iter()
                        .map(|layer| layer.position.y.clone())
                        .collect(),
                )),
                "background-size" => Some(Property::BackgroundSize(
                    layers.iter().map(|layer| layer.size.clone()).collect(),
                )),
                "background-repeat" => Some(Property::BackgroundRepeat(
                    layers.iter().map(|layer| layer.repeat.clone()).collect(),
                )),
                "background-attachment" => Some(Property::BackgroundAttachment(
                    layers.iter().map(|layer| layer.attachment).collect(),
                )),
                "background-origin" => Some(Property::BackgroundOrigin(
                    layers.iter().map(|layer| layer.origin).collect(),
                )),
                "background-clip" => Some(Property::BackgroundClip(
                    layers.iter().map(|layer| layer.clip).collect(),
                    lightningcss::vendor_prefix::VendorPrefix::None,
                )),
                _ => None,
            },
            Property::Mask(layers, prefix) => match name {
                "mask-image" => Some(Property::MaskImage(
                    layers.iter().map(|layer| layer.image.clone()).collect(),
                    *prefix,
                )),
                "-webkit-mask-position-x" => Some(Property::MaskPositionX(
                    layers
                        .iter()
                        .map(|layer| layer.position.x.clone())
                        .collect(),
                )),
                "-webkit-mask-position-y" => Some(Property::MaskPositionY(
                    layers
                        .iter()
                        .map(|layer| layer.position.y.clone())
                        .collect(),
                )),
                "mask-size" => Some(Property::MaskSize(
                    layers.iter().map(|layer| layer.size.clone()).collect(),
                    *prefix,
                )),
                "mask-repeat" => Some(Property::MaskRepeat(
                    layers.iter().map(|layer| layer.repeat.clone()).collect(),
                    *prefix,
                )),
                "mask-origin" => Some(Property::MaskOrigin(
                    layers.iter().map(|layer| layer.origin).collect(),
                    *prefix,
                )),
                "mask-clip" => Some(Property::MaskClip(
                    layers.iter().map(|layer| layer.clip.clone()).collect(),
                    *prefix,
                )),
                "mask-composite" => Some(Property::MaskComposite(
                    layers.iter().map(|layer| layer.composite).collect(),
                )),
                "mask-mode" => Some(Property::MaskMode(
                    layers.iter().map(|layer| layer.mode).collect(),
                )),
                _ => None,
            },
            _ => None,
        }
    }

    pub(crate) fn observable_position_or_size(
        &self,
        longhand: &str,
        projected_observable: &str,
    ) -> Option<Result<String, EngineError>> {
        match self.property {
            Property::Background(layers) => match longhand {
                "background-position-x" | "background-position-y"
                    if self.presence.iter().all(|presence| presence.position) =>
                {
                    Some(Ok(projected_observable.to_owned()))
                }
                "background-position-x" => Some(serialize_layers(
                    layers.iter().zip(&self.presence).map(|(layer, presence)| {
                        presence
                            .position
                            .then_some(&layer.position.x)
                            .map(serialize_typed)
                            .unwrap_or_else(|| Ok("initial".to_owned()))
                    }),
                )),
                "background-position-y" => Some(serialize_layers(
                    layers.iter().zip(&self.presence).map(|(layer, presence)| {
                        presence
                            .position
                            .then_some(&layer.position.y)
                            .map(serialize_typed)
                            .unwrap_or_else(|| Ok("initial".to_owned()))
                    }),
                )),
                "background-size" if self.presence.iter().all(|presence| presence.size) => {
                    Some(Ok(projected_observable.to_owned()))
                }
                "background-size" => Some(serialize_layers(layers.iter().zip(&self.presence).map(
                    |(layer, presence)| {
                        presence
                            .size
                            .then_some(&layer.size)
                            .map(serialize_typed)
                            .unwrap_or_else(|| Ok("initial".to_owned()))
                    },
                ))),
                _ => None,
            },
            Property::Mask(layers, _) => match longhand {
                "-webkit-mask-position-x" | "-webkit-mask-position-y"
                    if self.presence.iter().all(|presence| presence.position) =>
                {
                    Some(Ok(projected_observable.to_owned()))
                }
                "-webkit-mask-position-x" => Some(serialize_layers(
                    layers.iter().zip(&self.presence).map(|(layer, presence)| {
                        presence
                            .position
                            .then_some(&layer.position.x)
                            .map(serialize_typed)
                            .unwrap_or_else(|| Ok("0%".to_owned()))
                    }),
                )),
                "-webkit-mask-position-y" => Some(serialize_layers(
                    layers.iter().zip(&self.presence).map(|(layer, presence)| {
                        presence
                            .position
                            .then_some(&layer.position.y)
                            .map(serialize_typed)
                            .unwrap_or_else(|| Ok("0%".to_owned()))
                    }),
                )),
                "mask-size" if self.presence.iter().all(|presence| presence.size) => {
                    Some(Ok(projected_observable.to_owned()))
                }
                "mask-size" => Some(serialize_layers(
                    layers.iter().map(|layer| serialize_typed(&layer.size)),
                )),
                _ => None,
            },
            _ => None,
        }
    }
}

fn serialize_layers(
    layers: impl Iterator<Item = Result<String, EngineError>>,
) -> Result<String, EngineError> {
    Ok(layers.collect::<Result<Vec<_>, _>>()?.join(", "))
}

fn serialize_typed(value: &impl ToCss) -> Result<String, EngineError> {
    value
        .to_css_string(PrinterOptions::default())
        .map_err(|error| EngineError::Serialize(error.to_string()))
}

fn layer_presence(recovered: &RecoveredValue) -> Vec<LayerPresence> {
    let mut layers = vec![LayerPresence::default()];
    for component in recovered.values() {
        match &component.kind {
            RecoveredComponentKind::Token(RecoveredToken {
                kind: RecoveredTokenKind::Comma,
                ..
            }) => layers.push(LayerPresence::default()),
            RecoveredComponentKind::Token(RecoveredToken {
                kind: RecoveredTokenKind::Delimiter('/'),
                ..
            }) => {
                if let Some(layer) = layers.last_mut() {
                    layer.position = true;
                    layer.size = true;
                }
            }
            RecoveredComponentKind::Token(token) if is_position_token(token) => {
                if let Some(layer) = layers.last_mut() {
                    layer.position = true;
                }
            }
            RecoveredComponentKind::Function { name, .. }
                if is_position_function(name.as_str()) =>
            {
                if let Some(layer) = layers.last_mut() {
                    layer.position = true;
                }
            }
            _ => {}
        }
    }
    layers
}

fn is_position_token(token: &RecoveredToken) -> bool {
    match &token.kind {
        RecoveredTokenKind::Ident(value) => ["left", "right", "top", "bottom", "center"]
            .iter()
            .any(|candidate| value.eq_ignore_ascii_case(candidate)),
        RecoveredTokenKind::Number { .. }
        | RecoveredTokenKind::Percentage { .. }
        | RecoveredTokenKind::Dimension { .. } => true,
        _ => false,
    }
}

fn is_position_function(name: &str) -> bool {
    ["anchor", "anchor-size", "calc", "clamp", "max", "min"]
        .iter()
        .any(|candidate| name.eq_ignore_ascii_case(candidate))
}

fn contains_authored_image_set_url(values: &[RecoveredComponentValue]) -> bool {
    values.iter().any(|component| {
        let RecoveredComponentKind::Function {
            name,
            values: options,
            ..
        } = &component.kind
        else {
            return false;
        };
        ["image-set", "-webkit-image-set"]
            .iter()
            .any(|candidate| name.eq_ignore_ascii_case(candidate))
            && options.iter().any(is_authored_url)
    })
}

fn is_authored_url(component: &RecoveredComponentValue) -> bool {
    match &component.kind {
        RecoveredComponentKind::Token(RecoveredToken {
            kind: RecoveredTokenKind::Url(_),
            ..
        }) => true,
        RecoveredComponentKind::Function { name, .. } => name.eq_ignore_ascii_case("url"),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{parse_semantic_property, SemanticPropertyValue};

    #[test]
    fn compact_and_spaced_separators_have_the_same_structured_projection() {
        for shorthand in ["background", "mask", "-webkit-mask"] {
            let mut observed = Vec::new();
            for source in ["center/cover", "center / cover"] {
                let declaration = parse_semantic_property(shorthand, source).unwrap();
                let SemanticPropertyValue::Standard(property) = declaration.value() else {
                    panic!("{shorthand} did not retain a typed property");
                };
                let projection =
                    LayeredShorthandProjection::new(shorthand, property, declaration.recovered())
                        .unwrap();
                let prefix = if shorthand == "background" {
                    "background"
                } else {
                    "-webkit-mask"
                };
                observed.push((
                    projection
                        .observable_position_or_size(&format!("{prefix}-position-x"), "center")
                        .unwrap()
                        .unwrap(),
                    projection
                        .observable_position_or_size(&format!("{prefix}-position-y"), "center")
                        .unwrap()
                        .unwrap(),
                    projection
                        .observable_position_or_size(
                            if shorthand == "background" {
                                "background-size"
                            } else {
                                "mask-size"
                            },
                            "cover",
                        )
                        .unwrap()
                        .unwrap(),
                ));
            }
            assert_eq!(observed[0], observed[1], "{shorthand}");
            assert_eq!(
                observed[0],
                ("center".to_owned(), "center".to_owned(), "cover".to_owned())
            );
        }
    }
}
