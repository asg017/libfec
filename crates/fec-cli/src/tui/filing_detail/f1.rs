use fec_parser::covers::Form1;
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};

pub struct FilingDetailF1 {
    pub street_1: String,
    pub street_2: Option<String>,
    pub city: String,
    pub state: String,
    pub zip_code: String,
    pub committee_email: Option<String>,
    pub committee_url: Option<String>,
    pub effective_date: Option<String>,
    pub committee_type: Option<String>,
    pub candidate_name: Option<String>,
    pub candidate_id: Option<String>,
    pub candidate_office: Option<String>,
    pub candidate_state: Option<String>,
    pub candidate_district: Option<String>,
    pub party_code: Option<String>,
    pub party_type: Option<String>,
    pub organization_type: Option<String>,
    pub leadership_pac: Option<String>,
}

impl From<&Form1> for FilingDetailF1 {
    fn from(form: &Form1) -> Self {
        let (candidate_name, candidate_id, candidate_office, candidate_state, candidate_district) =
            if let Some(ref c) = form.candidate {
                (
                    Some(c.full_name()),
                    Some(c.candidate_id.clone()),
                    c.office.clone(),
                    c.state.clone(),
                    c.district.clone(),
                )
            } else {
                (None, None, None, None, None)
            };
        Self {
            street_1: form.street_1.clone(),
            street_2: form.street_2.clone(),
            city: form.city.clone(),
            state: form.state.clone(),
            zip_code: form.zip_code.clone(),
            committee_email: form.committee_email.clone(),
            committee_url: form.committee_url.clone(),
            effective_date: form.effective_date.map(|d| d.to_string()),
            committee_type: form.committee_type.clone(),
            candidate_name,
            candidate_id,
            candidate_office,
            candidate_state,
            candidate_district,
            party_code: form.party_code.clone(),
            party_type: form.party_type.clone(),
            organization_type: form.organization_type.clone(),
            leadership_pac: form.leadership_pac.clone(),
        }
    }
}

fn label_line(label: &str, value: String) -> Line<'static> {
    Line::from(vec![
        Span::styled(
            label.to_string(),
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(value),
    ])
}

pub fn append_f1_content_lines(lines: &mut Vec<Line<'static>>, data: &FilingDetailF1) {
    // Address
    let mut addr = data.street_1.clone();
    if let Some(ref s2) = data.street_2 {
        if !s2.is_empty() {
            addr.push_str(", ");
            addr.push_str(s2);
        }
    }
    addr.push_str(&format!(", {}, {} {}", data.city, data.state, data.zip_code));
    lines.push(label_line("Address: ", addr));

    // Email
    if let Some(ref email) = data.committee_email {
        if !email.is_empty() {
            lines.push(label_line("Email: ", email.clone()));
        }
    }

    // Website
    if let Some(ref url) = data.committee_url {
        if !url.is_empty() {
            lines.push(Line::from(vec![
                Span::styled(
                    "Website: ".to_string(),
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    url.clone(),
                    Style::default()
                        .fg(Color::Blue)
                        .add_modifier(Modifier::UNDERLINED),
                ),
            ]));
        }
    }

    // Effective date
    if let Some(ref date) = data.effective_date {
        lines.push(label_line("Effective Date: ", date.clone()));
    }

    // Committee type
    if let Some(ref ct) = data.committee_type {
        lines.push(label_line("Committee Type: ", ct.clone()));
    }

    lines.push(Line::from(""));

    // Candidate info
    if let Some(ref name) = data.candidate_name {
        let mut parts = vec![
            Span::styled(
                "Candidate: ".to_string(),
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                name.clone(),
                Style::default().add_modifier(Modifier::BOLD),
            ),
        ];
        if let Some(ref id) = data.candidate_id {
            parts.push(Span::raw(format!(" ({})", id)));
        }
        lines.push(Line::from(parts));

        // Office / State / District
        let mut office_parts = vec![];
        if let Some(ref office) = data.candidate_office {
            office_parts.push(office.clone());
        }
        if let Some(ref state) = data.candidate_state {
            office_parts.push(state.clone());
        }
        if let Some(ref district) = data.candidate_district {
            office_parts.push(format!("District {}", district));
        }
        if !office_parts.is_empty() {
            lines.push(label_line("Office: ", office_parts.join(" - ")));
        }
    }

    // Party
    if data.party_code.is_some() || data.party_type.is_some() {
        let party = match (&data.party_code, &data.party_type) {
            (Some(code), Some(ptype)) => format!("{} ({})", code, ptype),
            (Some(code), None) => code.clone(),
            (None, Some(ptype)) => ptype.clone(),
            (None, None) => unreachable!(),
        };
        lines.push(label_line("Party: ", party));
    }

    // Organization type
    if let Some(ref org) = data.organization_type {
        if !org.is_empty() {
            lines.push(label_line("Organization Type: ", org.clone()));
        }
    }

    // Leadership PAC
    if let Some(ref pac) = data.leadership_pac {
        if !pac.is_empty() {
            lines.push(label_line("Leadership PAC: ", pac.clone()));
        }
    }

    lines.push(Line::from(""));
}
