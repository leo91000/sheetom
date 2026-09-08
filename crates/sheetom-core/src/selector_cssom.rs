use cssparser::{Token, TokenizerWithSpans};

/// Project the engine's selector spelling into observable CSSOM spelling.
/// Token spans keep strings, escapes, and attribute contents out of rewrites.
pub(crate) fn serialize(source: &str, default_namespace: bool) -> String {
    let mut tokenizer = TokenizerWithSpans::new(source);
    let mut tokens = Vec::new();
    while let Ok(token) = tokenizer.next_token() {
        tokens.push(token);
    }
    let mut edits = Vec::new();
    let mut square_depth = 0usize;
    for (index, span) in tokens.iter().enumerate() {
        let start = span.start.byte_index();
        let end = span.end.byte_index();
        match &span.token {
            Token::SquareBracketBlock => square_depth += 1,
            Token::CloseSquareBracket => square_depth = square_depth.saturating_sub(1),
            Token::Ident(name)
                if index > 0
                    && matches!(&tokens[index - 1].token, Token::Function(name) if name.starts_with("nth-")) =>
            {
                if name.eq_ignore_ascii_case("odd") {
                    edits.push((start, end, "2n+1"));
                } else if name.eq_ignore_ascii_case("even") {
                    edits.push((start, end, "2n"));
                }
            }
            Token::Ident(name)
                if matches!(
                    name.as_ref(),
                    "before" | "after" | "first-line" | "first-letter"
                ) && index > 0 =>
            {
                if matches!(tokens[index - 1].token, Token::Colon)
                    && (index < 2 || !matches!(tokens[index - 2].token, Token::Colon))
                {
                    edits.push((start, start, ":"));
                }
            }
            Token::WhiteSpace(_)
                if index > 0
                    && matches!(&tokens[index - 1].token, Token::Function(name) if name.eq_ignore_ascii_case("has")) =>
            {
                edits.push((start, end, ""))
            }
            Token::Delim('*')
                if !default_namespace
                    && square_depth == 0
                    && tokens
                        .get(index + 1)
                        .is_some_and(|next| matches!(next.token, Token::Delim('|'))) =>
            {
                edits.push((start, tokens[index + 1].end.byte_index(), ""));
            }
            _ => {}
        }
    }
    let mut result = String::new();
    let mut cursor = 0;
    for (start, end, replacement) in edits {
        result.push_str(&source[cursor..start]);
        result.push_str(replacement);
        cursor = end;
    }
    result.push_str(&source[cursor..]);
    result
}

/// Visit nested selector arguments without a recursive visitor stack.
pub(crate) fn all_components(
    selectors: &lightningcss::selector::SelectorList<'_>,
    accepts: impl Fn(&lightningcss::selector::Component<'_>) -> bool,
) -> bool {
    use lightningcss::selector::Component;
    let mut pending = selectors.0.iter().collect::<Vec<_>>();
    while let Some(selector) = pending.pop() {
        for component in selector.iter_raw_match_order() {
            if !accepts(component) {
                return false;
            }
            match component {
                Component::Any(_, selectors)
                | Component::Negation(selectors)
                | Component::Is(selectors)
                | Component::Where(selectors)
                | Component::Has(selectors) => pending.extend(selectors.iter()),
                Component::NthOf(nth) => pending.extend(nth.selectors().iter()),
                Component::Host(Some(selector)) | Component::Slotted(selector) => {
                    pending.push(selector)
                }
                _ => {}
            }
        }
    }
    true
}
