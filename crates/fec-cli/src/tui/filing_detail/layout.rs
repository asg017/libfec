//! Shared line builders for the per-form cover renderers.
//!
//! Every renderer writes into a [`Doc`], which knows the view width and wraps
//! everything it emits so no line is wider than the view and nothing is
//! truncated:
//!
//! - [`Doc::heading`]: a section heading (cyan, bold, underlined).
//! - [`Doc::field`] and friends: `Label:  value` with one label width for every
//!   form; long values wrap under the value column (hanging indent).
//! - [`Doc::address`]: street, then city/state/ZIP, each under the value column.
//! - [`Doc::note`]: a dim explanatory line.
//! - [`Doc::table`] + [`Doc::row`]/[`Doc::amount`]/[`Doc::caption`]: a money
//!   table with one or two right-aligned columns. Labels wrap (amounts sit on
//!   the label's last line); below a minimum label width the labels take the
//!   whole line and the amounts follow on the next.
//! - [`Doc::cash_flow`]: the start / receipts / disbursements / end block.

use super::format_usd;
use fec_parser::covers::{Address, DetailedSummaryRow};
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};

/// Width of the field label column, including the colon and a gap: fits the
/// longest label (`51st contributor:`) plus one space.
const LABEL_WIDTH: usize = 18;
/// Width of one money amount, enough for `-$999,999,999.99`.
const AMOUNT_WIDTH: usize = 16;
/// One money column: a one-space gap plus the amount.
const COLUMN_WIDTH: usize = AMOUNT_WIDTH + 1;
/// Widest a money-table label column grows on wide views.
const MAX_TABLE_LABEL: usize = 48;
/// Narrowest money-table label column; below this labels get their own lines.
const MIN_TABLE_LABEL: usize = 14;
/// Extra indent of a wrapped money-table label's continuation lines when the
/// label does not start with a line number.
const TABLE_HANG: usize = 2;
/// Width of the label column in [`Doc::cash_flow`].
const CASH_FLOW_LABEL: usize = 22;
/// Narrowest width anything is wrapped to; narrower views let the paragraph
/// wrap instead.
const MIN_WIDTH: usize = 20;

fn label_style() -> Style {
    Style::default()
        .fg(Color::Yellow)
        .add_modifier(Modifier::BOLD)
}

/// Bold, for names and other values that should stand out.
pub(super) fn bold(text: impl Into<String>) -> Span<'static> {
    Span::styled(text.into(), Style::default().add_modifier(Modifier::BOLD))
}

/// A dim aside after a value, e.g. a form line reference `(5b)`.
pub(super) fn dim(text: impl Into<String>) -> Span<'static> {
    Span::styled(text.into(), Style::default().fg(Color::DarkGray))
}

/// The "(changed)" flag for a form's "check if changed" boxes.
pub(super) fn changed() -> Span<'static> {
    Span::styled("(changed)", Style::default().fg(Color::Magenta))
}

/// `"Label (CODE)"`, or just the code when no label is sourced.
pub(super) fn code_with_label(code: &str, label: Option<&str>) -> String {
    match label {
        Some(label) => format!("{label} ({code})"),
        None => code.to_string(),
    }
}

/// A report code as `"October Quarterly (Q3)"`: the form's own label when it
/// has one, else the generic report-code label, else just the code.
pub(super) fn report_code_text(code: &str, label: Option<&str>) -> String {
    let generic = match fec_parser::report_code_label(code) {
        "[Unknown report code]" => None,
        label => Some(label),
    };
    code_with_label(code, label.or(generic))
}

/// `"General (G2024) on 2024-11-05 in CA"` for an election report, from
/// whichever parts are present; `None` when none are.
pub(super) fn election_text(
    code: Option<&str>,
    label: Option<&str>,
    date: Option<jiff::civil::Date>,
    state: Option<&str>,
) -> Option<String> {
    let mut parts = vec![];
    if let Some(code) = code {
        parts.push(code_with_label(code, label));
    }
    if let Some(date) = date {
        parts.push(format!("on {date}"));
    }
    if let Some(state) = state {
        parts.push(format!("in {state}"));
    }
    (!parts.is_empty()).then(|| parts.join(" "))
}

/// Column headings of a money table.
#[derive(Debug, Clone, Copy)]
pub(super) enum Columns {
    /// One unlabeled amount column.
    One,
    /// Column A and Column B, with their headings.
    Two(&'static str, &'static str),
}

/// Column layout of the current money table.
#[derive(Debug, Clone, Copy)]
struct Table {
    columns: usize,
    /// Width labels wrap to; amounts start after it.
    label_width: usize,
}

/// A width-aware line builder over the filing detail's content lines.
pub(super) struct Doc<'a> {
    lines: &'a mut Vec<Line<'static>>,
    width: usize,
    table: Table,
}

impl<'a> Doc<'a> {
    pub fn new(lines: &'a mut Vec<Line<'static>>, width: u16) -> Self {
        let width = (width as usize).max(MIN_WIDTH);
        let table = Table::new(width, Columns::One);
        Self {
            lines,
            width,
            table,
        }
    }

    /// Push a pre-built line as-is.
    pub fn push(&mut self, line: Line<'static>) {
        self.lines.push(line);
    }

    pub fn blank(&mut self) {
        self.lines.push(Line::from(""));
    }

    /// Word-wrap `body` after `prefix` (on the first line), indenting
    /// continuation lines by `indent` columns.
    pub fn wrapped(&mut self, prefix: Vec<Span<'static>>, indent: usize, body: Vec<Span<'static>>) {
        let prefix_width = spans_width(&prefix);
        let first = self.width.saturating_sub(prefix_width).max(1);
        let rest = self.width.saturating_sub(indent).max(1);
        let mut prefix = Some(prefix);
        for (spans, _) in wrap(&body, first, rest) {
            let mut line = prefix
                .take()
                .unwrap_or_else(|| vec![Span::raw(" ".repeat(indent))]);
            line.extend(spans);
            self.lines.push(Line::from(line));
        }
    }

    /// A section heading.
    pub fn heading(&mut self, title: &str) {
        let style = Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD | Modifier::UNDERLINED);
        self.wrapped(vec![], 0, vec![Span::styled(title.to_string(), style)]);
    }

    /// A dim explanatory note.
    pub fn note(&mut self, text: &str) {
        let style = Style::default()
            .fg(Color::DarkGray)
            .add_modifier(Modifier::ITALIC);
        self.wrapped(vec![], 0, vec![Span::styled(text.to_string(), style)]);
    }

    /// `Label:           value…`, the value given as spans. An empty label
    /// continues the previous field.
    pub fn field_spans(&mut self, label: &str, value: Vec<Span<'static>>) {
        let label = if label.is_empty() {
            String::new()
        } else {
            format!("{label}:")
        };
        let width = LABEL_WIDTH.max(label.chars().count() + 1);
        let prefix = vec![Span::styled(format!("{label:<width$}"), label_style())];
        self.wrapped(prefix, LABEL_WIDTH, value);
    }

    pub fn field(&mut self, label: &str, value: impl Into<String>) {
        self.field_spans(label, vec![Span::raw(value.into())]);
    }

    /// A field only when `value` is present.
    pub fn field_opt(&mut self, label: &str, value: Option<impl Into<String>>) {
        if let Some(value) = value {
            self.field(label, value);
        }
    }

    /// A field with the "(changed)" flag when `is_changed`.
    pub fn field_changed(&mut self, label: &str, value: impl Into<String>, is_changed: bool) {
        let mut spans = vec![Span::raw(value.into())];
        if is_changed {
            spans.extend([Span::raw(" "), changed()]);
        }
        self.field_spans(label, spans);
    }

    /// An address as street, then city/state/ZIP, both under the value
    /// column; flagged "(changed)" when `is_changed`. Skipped when blank
    /// (unless flagged).
    pub fn address(&mut self, label: &str, address: &Address, is_changed: bool) {
        let street = [address.street_1.as_deref(), address.street_2.as_deref()]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>()
            .join(", ");
        let locality = Address {
            street_1: None,
            street_2: None,
            ..address.clone()
        }
        .one_line();
        let mut parts: Vec<Vec<Span<'static>>> = [street, locality]
            .into_iter()
            .filter(|s| !s.is_empty())
            .map(|s| vec![Span::raw(s)])
            .collect();
        if is_changed {
            match parts.last_mut() {
                Some(last) => last.extend([Span::raw(" "), changed()]),
                None => parts.push(vec![changed()]),
            }
        }
        for (i, part) in parts.into_iter().enumerate() {
            self.field_spans(if i == 0 { label } else { "" }, part);
        }
    }

    /// Cash on hand at the start, receipts, disbursements and cash on hand at
    /// the end, with the change in cash on hand. The percentage change is
    /// shown only when the starting balance is positive; the sign and colour
    /// follow the dollar change.
    pub fn cash_flow(&mut self, begin: f64, receipts: f64, disbursements: f64, end: f64) {
        let row = |label: &str, sign: &str, amount: f64, style: Style| {
            vec![
                Span::raw(format!("{label:<CASH_FLOW_LABEL$}")),
                Span::styled(
                    format!("{sign}{:>AMOUNT_WIDTH$}", format_usd(amount)),
                    style,
                ),
            ]
        };
        self.push(Line::from(row(
            "Cash on Hand - Start",
            " ",
            begin,
            Style::default(),
        )));
        self.push(Line::from(row(
            "Receipts",
            "+",
            receipts,
            Style::default().fg(Color::Blue),
        )));
        self.push(Line::from(row(
            "Disbursements",
            "-",
            disbursements,
            Style::default().fg(Color::Red),
        )));
        let mut end_line = row(
            "Cash on Hand - End",
            " ",
            end,
            Style::default().add_modifier(Modifier::BOLD),
        );

        let change_cents = ((end - begin) * 100.0).round();
        let (sign, color) = if change_cents > 0.0 {
            ("+", Color::Green)
        } else if change_cents < 0.0 {
            ("", Color::Red) // format_usd supplies the minus sign
        } else {
            ("", Color::DarkGray)
        };
        let mut change = if change_cents == 0.0 {
            "no change".to_string()
        } else {
            format!("{sign}{}", format_usd(end - begin))
        };
        if begin > 0.0 && change_cents != 0.0 {
            change.push_str(&format!(", {sign}{:.0}%", (end - begin) / begin * 100.0));
        }
        let change = Span::styled(change, Style::default().fg(color));
        if spans_width(&end_line) + 1 + change.width() <= self.width {
            end_line.extend([Span::raw(" "), change]);
            self.push(Line::from(end_line));
        } else {
            self.push(Line::from(end_line));
            self.push(Line::from(vec![
                Span::raw(" ".repeat(CASH_FLOW_LABEL)),
                change,
            ]));
        }
        self.blank();
    }

    /// Start a money table with `columns`, pushing its column headings.
    pub fn table(&mut self, columns: Columns) {
        self.table = Table::new(self.width, columns);
        if let Columns::Two(a, b) = columns {
            let style = Style::default()
                .fg(Color::Gray)
                .add_modifier(Modifier::BOLD);
            self.push(Line::from(vec![
                Span::raw(" ".repeat(self.table.amount_offset(self.width))),
                Span::styled(format!(" {a:>AMOUNT_WIDTH$}"), style),
                Span::styled(format!(" {b:>AMOUNT_WIDTH$}"), style),
            ]));
        }
    }

    /// A money-table line with no amounts, e.g. `11. Contributions from:`.
    pub fn caption(&mut self, label: &str) {
        let (lead, text) = split_indent(label);
        self.wrapped(
            vec![Span::raw(" ".repeat(lead))],
            hang_indent(label),
            vec![Span::raw(text.to_string())],
        );
    }

    /// A money-table row. `None` leaves that column blank (the form has no
    /// box there). `total` bolds the row.
    pub fn row_ab(&mut self, label: &str, a: Option<f64>, b: Option<f64>, total: bool) {
        let label_style = if total {
            Style::default().add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };
        let (lead, text) = split_indent(label);
        let mut width = self.table.label_width;
        let mut hang = hang_indent(label).min(width / 2);
        // A word too long for the label column: give the label the whole
        // line (amounts follow on the next) rather than split the word.
        let longest = text.split(' ').map(|w| w.chars().count()).max();
        if longest.unwrap_or(0) > width.saturating_sub(hang) {
            width = self.width;
            hang = hang_indent(label).min(width / 2);
        }
        let first = width.saturating_sub(lead).max(1);
        let rest = width.saturating_sub(hang).max(1);
        let pieces = wrap(&[Span::styled(text.to_string(), label_style)], first, rest);
        let count = pieces.len();
        let mut line = vec![];
        let mut last_width = 0;
        for (i, (spans, piece_width)) in pieces.into_iter().enumerate() {
            let indent = if i == 0 { lead } else { hang };
            line = vec![Span::raw(" ".repeat(indent))];
            line.extend(spans);
            if i + 1 < count {
                self.push(Line::from(std::mem::take(&mut line)));
            } else {
                last_width = indent + piece_width;
            }
        }
        let offset = self.table.amount_offset(self.width);
        let used = last_width;
        if used > offset {
            // Stacked layout: the label used the whole line.
            self.push(Line::from(line));
            line = vec![Span::raw(" ".repeat(offset))];
        } else {
            line.push(Span::raw(" ".repeat(offset - used)));
        }
        let cells = [a, b];
        for cell in cells.iter().take(self.table.columns) {
            line.push(match cell {
                Some(v) => {
                    let mut style = if *v == 0.0 {
                        Style::default().fg(Color::DarkGray)
                    } else {
                        Style::default()
                    };
                    if total {
                        style = style.add_modifier(Modifier::BOLD);
                    }
                    Span::styled(format!(" {:>AMOUNT_WIDTH$}", format_usd(*v)), style)
                }
                None => Span::raw(" ".repeat(COLUMN_WIDTH)),
            });
        }
        self.push(Line::from(line));
    }

    /// A Column A / Column B row.
    pub fn row(&mut self, label: &str, row: &DetailedSummaryRow) {
        self.row_ab(label, Some(row.column_a), Some(row.column_b), false);
    }

    /// A bold Column A / Column B total row.
    pub fn total(&mut self, label: &str, row: &DetailedSummaryRow) {
        self.row_ab(label, Some(row.column_a), Some(row.column_b), true);
    }

    /// A single-amount row (Column A).
    pub fn amount(&mut self, label: &str, value: f64, total: bool) {
        self.row_ab(label, Some(value), None, total);
    }
}

impl Table {
    fn new(width: usize, columns: Columns) -> Self {
        let count = match columns {
            Columns::One => 1,
            Columns::Two(..) => 2,
        };
        // Capped so that on wide views single-amount tables line up with
        // Column A of two-column ones.
        let label_width = width
            .saturating_sub(count * COLUMN_WIDTH)
            .min(MAX_TABLE_LABEL);
        let label_width = if label_width < MIN_TABLE_LABEL {
            width // stacked: labels take the whole line
        } else {
            label_width
        };
        Self {
            columns: count,
            label_width,
        }
    }

    /// Column at which the amounts start.
    fn amount_offset(&self, width: usize) -> usize {
        self.label_width
            .min(width.saturating_sub(self.columns * COLUMN_WIDTH))
    }
}

/// Leading spaces of `label` and the rest.
/// Indent of a wrapped money-table label's continuation lines: under the
/// text after a leading line number (`11.  Contributions…`, `(a)(i) Indiv…`),
/// else a little past the label's own indent.
fn hang_indent(label: &str) -> usize {
    let (lead, text) = split_indent(label);
    let number_len = text.find(' ').unwrap_or(0);
    let number = &text[..number_len];
    let is_line_number = number_len > 0
        && number_len <= 10
        && number.chars().any(|c| c.is_ascii_digit() || c == '(');
    if is_line_number {
        let after = text[number_len..].trim_start_matches(' ');
        lead + text.len() - after.len()
    } else {
        lead + TABLE_HANG
    }
}

fn split_indent(label: &str) -> (usize, &str) {
    let text = label.trim_start_matches(' ');
    (label.len() - text.len(), text)
}

fn spans_width(spans: &[Span<'_>]) -> usize {
    spans.iter().map(|s| s.content.chars().count()).sum()
}

/// Greedy word wrap of styled `spans`: the first line holds at most `first`
/// columns, the rest at most `rest`. Runs of spaces between words are kept
/// within a line and dropped at a break; leading spaces on the first line are
/// kept. Words longer than a line are split. Returns each line's spans and
/// width; always at least one (possibly empty) line.
pub(super) fn wrap(
    spans: &[Span<'static>],
    first: usize,
    rest: usize,
) -> Vec<(Vec<Span<'static>>, usize)> {
    // Tokenize into words (runs of non-space chars, possibly spanning styles)
    // separated by space runs.
    enum Token {
        Space(usize, Style),
        Word(Vec<(String, Style)>),
    }
    let mut tokens: Vec<Token> = vec![];
    for span in spans {
        for ch in span.content.chars() {
            match (ch == ' ', tokens.last_mut()) {
                (true, Some(Token::Space(n, _))) => *n += 1,
                (true, _) => tokens.push(Token::Space(1, span.style)),
                (false, Some(Token::Word(parts))) => match parts.last_mut() {
                    Some((text, style)) if *style == span.style => text.push(ch),
                    _ => parts.push((ch.to_string(), span.style)),
                },
                (false, _) => tokens.push(Token::Word(vec![(ch.to_string(), span.style)])),
            }
        }
    }

    let mut out: Vec<(Vec<Span<'static>>, usize)> = vec![];
    let mut line: Vec<Span<'static>> = vec![];
    let mut used = 0;
    let mut pending: Option<(usize, Style)> = None;
    let avail = |out: &Vec<_>| if out.is_empty() { first } else { rest };
    for token in tokens {
        match token {
            Token::Space(n, style) => {
                if out.is_empty() && used == 0 {
                    // Leading indentation of the first line.
                    line.push(Span::styled(" ".repeat(n), style));
                    used += n;
                } else {
                    pending = Some((n, style));
                }
            }
            Token::Word(parts) => {
                let len: usize = parts.iter().map(|(t, _)| t.chars().count()).sum();
                let gap = pending.take().filter(|_| used > 0);
                let gap_len = gap.map_or(0, |(n, _)| n);
                if used > 0 && used + gap_len + len > avail(&out) {
                    out.push((std::mem::take(&mut line), used));
                    used = 0;
                } else if let Some((n, style)) = gap {
                    line.push(Span::styled(" ".repeat(n), style));
                    used += n;
                }
                // Place the word, hard-splitting it if it is wider than a line.
                for (text, style) in parts {
                    let mut chars = text.chars().peekable();
                    while chars.peek().is_some() {
                        let room = avail(&out).saturating_sub(used);
                        if room == 0 {
                            out.push((std::mem::take(&mut line), used));
                            used = 0;
                            continue;
                        }
                        let piece: String = chars.by_ref().take(room).collect();
                        used += piece.chars().count();
                        line.push(Span::styled(piece, style));
                    }
                }
            }
        }
    }
    out.push((line, used));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn texts(spans: &[Span<'static>], first: usize, rest: usize) -> Vec<String> {
        wrap(spans, first, rest)
            .into_iter()
            .map(|(spans, width)| {
                let s: String = spans.iter().map(|s| s.content.as_ref()).collect();
                assert_eq!(s.chars().count(), width);
                s
            })
            .collect()
    }

    #[test]
    fn wraps_words() {
        let raw = |s: &str| vec![Span::raw(s.to_string())];
        assert_eq!(texts(&raw("aa bb cc"), 5, 5), vec!["aa bb", "cc"]);
        assert_eq!(texts(&raw(""), 5, 5), vec![""]);
        assert_eq!(texts(&raw("abcdefgh"), 3, 3), vec!["abc", "def", "gh"]);
        assert_eq!(
            texts(&raw("  indented text"), 20, 20),
            vec!["  indented text"]
        );
        assert_eq!(texts(&raw("aa  bb cc"), 6, 4), vec!["aa  bb", "cc"]);
        assert_eq!(texts(&raw("aaa bbbbbb"), 5, 4), vec!["aaa", "bbbb", "bb"]);
    }

    #[test]
    fn wraps_across_spans() {
        let spans = vec![bold("Name"), Span::raw(" (C001)")];
        assert_eq!(texts(&spans, 8, 8), vec!["Name", "(C001)"]);
        assert_eq!(texts(&spans, 20, 8), vec!["Name (C001)"]);
    }

    #[test]
    fn hangs_under_line_text() {
        assert_eq!(hang_indent("11.  Contributions from:"), 5);
        assert_eq!(hang_indent("6(a) Cash on Hand"), 5);
        assert_eq!(hang_indent("  (a)(i)   Individuals"), 11);
        assert_eq!(hang_indent("Totals"), 2);
        assert_eq!(hang_indent("  Primary"), 4);
    }

    #[test]
    fn code_labels() {
        assert_eq!(code_with_label("H", Some("House")), "House (H)");
        assert_eq!(code_with_label("X", None), "X");
        assert_eq!(report_code_text("Q3", None), "October Quarterly (Q3)");
        assert_eq!(report_code_text("ZZZ", None), "ZZZ");
    }
}
