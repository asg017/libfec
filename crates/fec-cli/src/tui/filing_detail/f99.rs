//! Form 99 (Miscellaneous Electronic Submission) cover rendering.
//!
//! Rendered straight from [`fec_parser::covers::Form99`]: the filer, the text
//! code with its FEC label, then the message body word-wrapped to the view
//! width behind a quote gutter.

use super::f2::{code_with_label, field_line, field_spans, note_line, push_address, section_line};
use fec_parser::covers::Form99;
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};

const GUTTER: &str = "  │ ";
const TAB: &str = "    ";

pub fn append_f99_content_lines(lines: &mut Vec<Line<'static>>, form: &Form99, width: u16) {
    lines.push(section_line("Miscellaneous Text"));
    let mut filer = vec![Span::styled(
        form.committee_name.clone(),
        Style::default().add_modifier(Modifier::BOLD),
    )];
    if !form.committee_id.is_empty() {
        filer.push(Span::raw(format!(" ({})", form.committee_id)));
    }
    lines.push(field_spans("Committee", filer));
    push_address(lines, &form.address, "");
    if let Some(ref code) = form.text_code {
        lines.push(field_line(
            "Text code",
            code_with_label(code, form.text_code_label()),
        ));
    }
    if let Some(ref freq) = form.filing_frequency {
        lines.push(field_line("Frequency", freq.clone()));
    }
    if form.pdf_attachment {
        lines.push(field_line("PDF attached", "Yes".to_string()));
    }
    lines.push(Line::from(""));

    lines.push(section_line("Message"));
    match form.text.as_deref() {
        Some(text) => {
            let wrap_at = (width as usize)
                .saturating_sub(GUTTER.chars().count())
                .max(20);
            let gutter = Style::default().fg(Color::DarkGray);
            for source_line in text.lines() {
                for piece in wrap(&source_line.replace('\t', TAB), wrap_at) {
                    lines.push(Line::from(vec![
                        Span::styled(GUTTER, gutter),
                        Span::raw(piece),
                    ]));
                }
            }
        }
        None => lines.push(note_line("(no message text)")),
    }
    lines.push(Line::from(""));
}

/// Greedy word wrap of one line to at most `width` characters per piece.
/// Words longer than `width` are split. An empty line yields one empty piece.
fn wrap(line: &str, width: usize) -> Vec<String> {
    let line = line.trim_end();
    let mut pieces = vec![];
    let mut current = String::new();
    // Keep leading indentation on the first piece.
    let indent_len = line.len() - line.trim_start().len();
    current.push_str(&line[..indent_len]);
    for word in line[indent_len..].split(' ') {
        let mut word = word.to_string();
        loop {
            let used = current.chars().count();
            let sep = usize::from(used > 0 && !current.ends_with(' '));
            let len = word.chars().count();
            if used + sep + len <= width {
                if sep == 1 {
                    current.push(' ');
                }
                current.push_str(&word);
                break;
            }
            if used > 0 {
                pieces.push(std::mem::take(&mut current));
                continue;
            }
            // A word longer than the whole line: hard-split it.
            let head: String = word.chars().take(width).collect();
            word = word.chars().skip(width).collect();
            pieces.push(head);
        }
    }
    if !current.is_empty() || pieces.is_empty() {
        pieces.push(current);
    }
    pieces
}

#[cfg(test)]
mod tests {
    use super::wrap;

    #[test]
    fn wraps_words() {
        assert_eq!(wrap("aa bb cc", 5), vec!["aa bb", "cc"]);
        assert_eq!(wrap("", 5), vec![""]);
        assert_eq!(wrap("abcdefgh", 3), vec!["abc", "def", "gh"]);
        assert_eq!(wrap("  indented text", 20), vec!["  indented text"]);
    }
}
