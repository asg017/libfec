//! Form 1 (Statement of Organization) cover rendering for the filing detail view.
//!
//! Renders straight from [`fec_parser::covers::Form1`], one section per part of
//! the paper form: committee (Lines 1–3), type & designation and candidate
//! (Line 5), affiliated/connected organization (Line 6), custodian (Line 7),
//! treasurer and designated agent (Line 8) and banks (Line 9).

use super::layout::{bold, changed, code_with_label, Doc};
use fec_parser::covers::{Form1, Form1Affiliated, Form1Contact};
use ratatui::{
    style::{Color, Modifier, Style},
    text::Span,
};

/// `(405) 826-6448` for ten-digit numbers, otherwise the value as filed.
fn format_phone(phone: &str) -> String {
    let phone = phone.trim();
    if phone.len() == 10 && phone.chars().all(|c| c.is_ascii_digit()) {
        format!("({}) {}-{}", &phone[0..3], &phone[3..6], &phone[6..])
    } else {
        phone.to_string()
    }
}

/// True for an affiliate entry that only says there is none, e.g. a name of
/// `NONE` or `N/A` with no ID, address or relationship.
pub fn affiliated_is_placeholder(a: &Form1Affiliated) -> bool {
    let name = a.display_name();
    let name = name.trim().trim_end_matches('.').to_ascii_uppercase();
    matches!(name.as_str(), "" | "NONE" | "N/A" | "NA" | "NOT APPLICABLE")
        && a.committee_id.is_none()
        && a.candidate_id.is_none()
        && a.address.is_empty()
        && a.relationship_code.is_none()
}

fn push_contact(d: &mut Doc, title: &str, contact: &Form1Contact) {
    d.heading(title);
    let name = contact.name.to_string();
    d.field(
        "Name",
        if name.is_empty() {
            "—".to_string()
        } else {
            name
        },
    );
    d.field_opt("Title", contact.title.clone());
    d.address("Address", &contact.address, false);
    d.field_opt("Phone", contact.telephone.as_deref().map(format_phone));
    d.blank();
}

fn push_committee(d: &mut Doc, form: &Form1) {
    d.heading("Committee");
    d.field_changed(
        "Name",
        form.committee_name.clone(),
        form.change_of_committee_name,
    );
    if !form.filer_committee_id_number.is_empty() {
        d.field("FEC ID", form.filer_committee_id_number.clone());
    }
    d.address("Address", &form.address, form.change_of_address);
    if let Some(ref email) = form.committee_email {
        d.field_changed("Email", email.clone(), form.change_of_committee_email);
    }
    if let Some(ref url) = form.committee_url {
        let mut spans = vec![Span::styled(
            url.clone(),
            Style::default()
                .fg(Color::Blue)
                .add_modifier(Modifier::UNDERLINED),
        )];
        if form.change_of_committee_url {
            spans.extend([Span::raw(" "), changed()]);
        }
        d.field_spans("Website", spans);
    }
    d.field_opt("Effective date", form.effective_date.map(|d| d.to_string()));
    d.blank();
}

fn push_type(d: &mut Doc, form: &Form1) {
    d.heading("Type & designation");
    let committee_type = match (
        form.committee_type.as_deref(),
        form.committee_type_line(),
        form.committee_type_label(),
    ) {
        (Some(code), Some(line), Some(label)) => format!("{line} {label} ({code})"),
        (Some(code), _, _) => code.to_string(),
        (None, _, _) => "—".to_string(),
    };
    d.field("Committee type", committee_type);
    if let Some(ref org) = form.organization_type {
        d.field(
            "Connected org",
            code_with_label(org, form.organization_type_label()),
        );
    }
    if form.party_code.is_some() || form.party_type.is_some() {
        let mut parts = vec![];
        if let Some(ref code) = form.party_code {
            parts.push(code_with_label(code, form.party_code_label()));
        }
        if let Some(ref ptype) = form.party_type {
            parts.push(code_with_label(ptype, form.party_type_label()));
        }
        d.field("Party", parts.join(" · "));
    }
    let mut also = vec![];
    if form.pac_flags.is_lobbyist_registrant_pac() {
        also.push("Lobbyist/Registrant PAC");
    }
    if form.pac_flags.leadership_pac {
        also.push("Leadership PAC");
    }
    if !also.is_empty() {
        d.field("Also", also.join(", "));
    }
    d.blank();
}

fn push_candidate(d: &mut Doc, form: &Form1) {
    let Some(ref c) = form.candidate else {
        return;
    };
    d.heading("Candidate");
    let mut name = vec![bold(c.full_name())];
    if let Some(ref id) = c.candidate_id {
        name.push(Span::raw(format!(" ({id})")));
    }
    d.field_spans("Name", name);

    let mut office = vec![];
    if let Some(ref o) = c.office {
        office.push(c.office_label().unwrap_or(o).to_string());
    }
    if let Some(ref state) = c.state {
        office.push(state.clone());
    }
    // Only House races have districts; Senate/President filings carry "00".
    if let Some(ref district) = c.district {
        if c.office.as_deref() == Some("H") {
            office.push(format!("District {district}"));
        }
    }
    if !office.is_empty() {
        d.field("Office", office.join(" · "));
    }
    d.blank();
}

fn push_affiliated(d: &mut Doc, form: &Form1) {
    let Some(ref a) = form.affiliated else {
        return;
    };
    if affiliated_is_placeholder(a) {
        return;
    }
    d.heading("Affiliated / connected organization");
    let mut name = a.display_name();
    if let Some(id) = a.committee_id.as_ref().or(a.candidate_id.as_ref()) {
        name.push_str(&format!(" ({id})"));
    }
    d.field("Name", name);
    if let Some(ref code) = a.relationship_code {
        d.field(
            "Relationship",
            code_with_label(code, a.relationship_label()),
        );
    }
    d.address("Address", &a.address, false);
    d.blank();
}

fn push_banks(d: &mut Doc, form: &Form1) {
    if form.banks.is_empty() {
        return;
    }
    d.heading("Banks / depositories");
    for (i, bank) in form.banks.iter().enumerate() {
        d.field(
            &format!("Bank {}", i + 1),
            bank.name.clone().unwrap_or_else(|| "—".to_string()),
        );
        d.address("Address", &bank.address, false);
    }
    d.blank();
}

pub(super) fn append_f1_content_lines(d: &mut Doc, form: &Form1) {
    push_committee(d, form);
    push_type(d, form);
    push_candidate(d, form);
    push_affiliated(d, form);
    if let Some(ref custodian) = form.custodian {
        push_contact(d, "Custodian of records", custodian);
    }
    push_contact(d, "Treasurer", &form.treasurer);
    if let Some(ref agent) = form.agent {
        push_contact(d, "Designated agent", agent);
    }
    push_banks(d, form);
    d.note("Further affiliates, agents and banks may be filed on F1S records.");
    d.blank();
}

#[cfg(test)]
mod tests {
    use super::super::tests::render_fixture;
    use insta::assert_snapshot;

    #[test]
    fn filing_detail_f1_candidate_committee() {
        assert_snapshot!(render_fixture("F1N_1906351.fec", 100, 70));
    }

    #[test]
    fn filing_detail_f1_ssf() {
        assert_snapshot!(render_fixture("F1A_1914988.fec", 100, 70));
    }

    #[test]
    fn filing_detail_f1_leadership_pac_narrow() {
        assert_snapshot!(render_fixture("F1A_1917499.fec", 60, 90));
    }

    #[test]
    fn format_phone() {
        assert_eq!(super::format_phone("4058266448"), "(405) 826-6448");
        assert_eq!(super::format_phone("x123"), "x123");
    }
}
