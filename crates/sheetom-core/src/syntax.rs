pub(crate) fn split_top_level_whitespace(value: &str) -> Option<Vec<&str>> {
    split_top_level(value, |byte| byte.is_ascii_whitespace(), false)
}

pub(crate) fn split_top_level_delimiter(value: &str, delimiter: u8) -> Option<Vec<&str>> {
    split_top_level(value, |byte| byte == delimiter, true)
}

fn split_top_level(
    value: &str,
    is_separator: impl Fn(u8) -> bool,
    require_nonempty_parts: bool,
) -> Option<Vec<&str>> {
    let bytes = value.as_bytes();
    let mut components = Vec::new();
    let mut start = None;
    let mut depth = 0usize;
    let mut index = 0usize;
    let mut quote = None;
    let mut in_comment = false;

    while index < bytes.len() {
        let byte = bytes[index];
        if in_comment {
            if byte == b'*' && bytes.get(index + 1) == Some(&b'/') {
                in_comment = false;
                index += 2;
                continue;
            }
            index += 1;
            continue;
        }
        if let Some(delimiter) = quote {
            if byte == b'\\' {
                index = (index + 2).min(bytes.len());
                continue;
            }
            if byte == delimiter {
                quote = None;
            }
            index += 1;
            continue;
        }
        if byte == b'/' && bytes.get(index + 1) == Some(&b'*') {
            in_comment = true;
            index += 2;
            continue;
        }
        if matches!(byte, b'\'' | b'"') {
            quote = Some(byte);
            start.get_or_insert(index);
            index += 1;
            continue;
        }
        if matches!(byte, b'(' | b'[' | b'{') {
            depth += 1;
            start.get_or_insert(index);
            index += 1;
            continue;
        }
        if matches!(byte, b')' | b']' | b'}') {
            if depth == 0 {
                return None;
            }
            depth -= 1;
            index += 1;
            continue;
        }
        if depth == 0 && is_separator(byte) {
            if let Some(component_start) = start.take() {
                let component = value[component_start..index].trim();
                if !component.is_empty() {
                    components.push(component);
                }
            } else if require_nonempty_parts {
                return None;
            }
            index += 1;
            continue;
        }
        start.get_or_insert(index);
        index += 1;
    }
    if depth != 0 || quote.is_some() || in_comment {
        return None;
    }
    if let Some(component_start) = start {
        let component = value[component_start..].trim();
        if !component.is_empty() {
            components.push(component);
        }
    } else if require_nonempty_parts {
        return None;
    }
    Some(components)
}
