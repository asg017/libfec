//! Form 99 (Miscellaneous Electronic Submission) cover rendering.
//!
//! Rendered straight from [`fec_parser::covers::Form99`]: the filer, the text
//! code with its FEC label, then the message body word-wrapped to the view
//! width behind a quote gutter.

use super::layout::{bold, code_with_label, wrap, Doc};
use fec_parser::covers::Form99;
use ratatui::{
    style::{Color, Style},
    text::{Line, Span},
};

const GUTTER: &str = "  │ ";
const TAB: &str = "    ";

pub(super) fn append_f99_content_lines(d: &mut Doc, form: &Form99) {
    d.heading("Miscellaneous Text");
    let mut filer = vec![bold(form.committee_name.clone())];
    if !form.committee_id.is_empty() {
        filer.push(Span::raw(format!(" ({})", form.committee_id)));
    }
    d.field_spans("Committee", filer);
    d.address("Address", &form.address, false);
    if let Some(ref code) = form.text_code {
        d.field("Text code", code_with_label(code, form.text_code_label()));
    }
    d.field_opt("Frequency", form.filing_frequency.clone());
    if form.pdf_attachment {
        d.field("PDF attached", "Yes");
    }
    d.blank();

    d.heading("Message");
    match form.text.as_deref() {
        Some(text) => {
            let wrap_at = d.width().saturating_sub(GUTTER.chars().count()).max(1);
            let gutter = Style::default().fg(Color::DarkGray);
            for source_line in text.lines() {
                let source = source_line.replace('\t', TAB);
                for (spans, _) in wrap(
                    &[Span::raw(source.trim_end().to_string())],
                    wrap_at,
                    wrap_at,
                ) {
                    let mut line = vec![Span::styled(GUTTER, gutter)];
                    line.extend(spans);
                    d.push(Line::from(line));
                }
            }
        }
        None => d.note("(no message text)"),
    }
    d.blank();
}
