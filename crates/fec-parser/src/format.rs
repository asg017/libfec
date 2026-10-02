//! Format details shared by the record readers: the text-block markers.

/// Whether a field is a `[BEGINTEXT]` marker: caseless, optional space
/// between the words, surrounding whitespace ignored (`[BeginText]`,
/// `[BEGIN TEXT]`).
pub(crate) fn is_begin_text(field: &[u8]) -> bool {
    is_marker(field, b"BEGIN")
}

/// Whether a field is an `[ENDTEXT]` marker; see [`is_begin_text`].
pub(crate) fn is_end_text(field: &[u8]) -> bool {
    is_marker(field, b"END")
}

fn is_marker(field: &[u8], word: &[u8]) -> bool {
    let f = field.trim_ascii();
    let Some(inner) = f.strip_prefix(b"[").and_then(|f| f.strip_suffix(b"]")) else {
        return false;
    };
    if inner.len() < word.len() || !inner[..word.len()].eq_ignore_ascii_case(word) {
        return false;
    }
    let rest = &inner[word.len()..];
    let rest = rest.strip_prefix(b" ").unwrap_or(rest);
    rest.eq_ignore_ascii_case(b"TEXT")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_markers() {
        for m in [
            "[BEGINTEXT]",
            "[BeginText]",
            "[BEGIN TEXT]",
            " [begintext] \r",
        ] {
            assert!(is_begin_text(m.as_bytes()), "{m:?}");
            assert!(!is_end_text(m.as_bytes()), "{m:?}");
        }
        for m in ["[ENDTEXT]", "[EndText]", "[END TEXT]", "[endtext]\t"] {
            assert!(is_end_text(m.as_bytes()), "{m:?}");
        }
        for m in ["BEGINTEXT", "[BEGINTEXTS]", "[BEGIN  TEXT]", "[END]", ""] {
            assert!(
                !is_begin_text(m.as_bytes()) && !is_end_text(m.as_bytes()),
                "{m:?}"
            );
        }
    }
}
